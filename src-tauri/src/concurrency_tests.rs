//! Real-thread tests for the library write lock (`AppState::io`): the
//! library operations run on Tauri's blocking pool in parallel, so these run
//! them from several OS threads against a real folder and check index.json
//! on disk afterwards. (Tauri's runtime itself is not involved — the
//! commands are thin wrappers around the `*_op` functions tested here.)

use crate::capture::file_capture;
use crate::commands::{
    import_op, load_current_library, rename_op, replace_with_png, save_edit_op, set_tags_op,
    sync_library_internal, sync_op,
};
use crate::settings::Settings;
use crate::state::AppState;
use base64::Engine;
use screencap_core::{load_library, Library, ShotKind};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("screencap-conc-{tag}-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn state_for(dir: &Path) -> Arc<AppState> {
    let s = Settings {
        library_dir: dir.to_string_lossy().to_string(),
        ..Settings::default()
    };
    Arc::new(AppState::new(s, Library::default(), false))
}

fn write_png(path: &Path, w: u32, shade: u8) {
    image::RgbaImage::from_pixel(w, 8, image::Rgba([shade, 0, 0, 255])).save(path).unwrap();
}

fn png_b64(w: u32, shade: u8) -> String {
    let mut buf = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(w, 8, image::Rgba([shade, 1, 2, 255]))
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
}

fn images_in(dir: &Path) -> HashSet<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|n| screencap_core::is_library_image(n))
        .collect()
}

/// index.json on disk lists exactly the images in its folder, each once.
fn assert_index_matches_folder(dir: &Path) -> Library {
    let lib = load_library(&dir.join("index.json")).unwrap();
    let files: Vec<_> = lib.shots.iter().map(|s| s.file.clone()).collect();
    let unique: HashSet<_> = files.iter().cloned().collect();
    assert_eq!(files.len(), unique.len(), "duplicate entries in {}", dir.display());
    assert_eq!(unique, images_in(dir), "index.json vs. folder {}", dir.display());
    lib
}

/// Mirrors the folder switch in `set_settings`: under the write lock, point
/// the settings to the new folder, then load its index.
fn switch_folder(st: &AppState, dir: &Path) {
    let _io = st.io_lock();
    st.settings.lock().unwrap().library_dir = dir.to_string_lossy().to_string();
    load_current_library(st).unwrap();
}

#[test]
fn import_and_library_switch_keep_both_folders_consistent() {
    let src = tmpdir("src");
    let mut paths = Vec::new();
    for i in 0..40 {
        let p = src.join(format!("pic{i}.png"));
        write_png(&p, 64, i as u8);
        paths.push(p.to_string_lossy().to_string());
    }
    for _round in 0..5 {
        let (a, b) = (tmpdir("a"), tmpdir("b"));
        write_png(&b.join("already-in-b.png"), 16, 9);
        let st = state_for(&a);
        let st2 = st.clone();
        let p2 = paths.clone();
        let importer = std::thread::spawn(move || import_op(&st2, &p2).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(3));
        switch_folder(&st, &b);
        let n = importer.join().unwrap();
        // the import ran either completely before or completely after the switch
        assert_eq!(n, 40);
        assert_index_matches_folder(&b);
        if a.join("index.json").exists() {
            assert_index_matches_folder(&a);
        }
        let disk_b = load_library(&b.join("index.json")).unwrap();
        let mem: HashSet<_> = st.lib.lock().unwrap().shots.iter().map(|s| s.id).collect();
        let disk: HashSet<_> = disk_b.shots.iter().map(|s| s.id).collect();
        assert_eq!(mem, disk, "in-memory library = index.json of the active folder");
        for d in [a, b] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
    let _ = std::fs::remove_dir_all(src);
}

#[test]
fn save_edit_and_capture_filing_lose_nothing() {
    let dir = tmpdir("edit");
    let st = state_for(&dir);
    write_png(&dir.join("orig.png"), 32, 1);
    sync_library_internal(&st).unwrap();
    let id = st.lib.lock().unwrap().shots[0].id;
    let caps: Vec<String> = (0..25).map(|i| format!("screencap-cap-{i}.png")).collect();
    for c in &caps {
        write_png(&dir.join(c), 20, 3);
    }
    let st2 = st.clone();
    let editor = std::thread::spawn(move || {
        for i in 0..25u32 {
            save_edit_op(&st2, id, &png_b64(40 + i, i as u8), "overwrite").unwrap();
        }
    });
    let st3 = st.clone();
    let caps2 = caps.clone();
    let capturer = std::thread::spawn(move || {
        caps2
            .iter()
            .map(|c| {
                let (cid, r) = file_capture(&st3, &dir_of(&st3), c, ShotKind::Region);
                r.unwrap();
                cid
            })
            .collect::<Vec<_>>()
    });
    editor.join().unwrap();
    let cap_ids = capturer.join().unwrap();
    let lib = assert_index_matches_folder(&dir);
    let edited = lib.get(id).unwrap();
    assert_eq!((edited.width, edited.rev), (64, 25), "last save wins, every save counted");
    for cid in cap_ids {
        assert_eq!(lib.get(cid).unwrap().kind, ShotKind::Region);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn dir_of(st: &AppState) -> PathBuf {
    st.lib_dir()
}

#[test]
fn rename_and_tags_during_sync_are_kept() {
    let dir = tmpdir("sync");
    let st = state_for(&dir);
    write_png(&dir.join("mine.png"), 16, 1);
    sync_library_internal(&st).unwrap();
    let id = st.lib.lock().unwrap().shots[0].id;
    for i in 0..60 {
        write_png(&dir.join(format!("foreign-{i}.png")), 8, 2);
    }
    let st2 = st.clone();
    let syncer = std::thread::spawn(move || {
        for _ in 0..10 {
            let _io = st2.io_lock();
            sync_library_internal(&st2).unwrap();
        }
    });
    for i in 0..50 {
        rename_op(&st, id, &format!("name-{i}")).unwrap();
        set_tags_op(&st, id, &[format!("t{i}")]).unwrap();
    }
    syncer.join().unwrap();
    let lib = assert_index_matches_folder(&dir);
    let s = lib.get(id).unwrap();
    assert_eq!(s.name, "name-49");
    assert_eq!(s.tags, vec!["t49".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn locked_jpg_is_not_left_behind_on_overwrite() {
    let dir = tmpdir("jpg");
    std::fs::write(dir.join("a.jpg"), b"old").unwrap();
    let locked = |_: &Path| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    let err = replace_with_png(&dir, "a.jpg", "a.png", b"new", locked).unwrap_err();
    assert!(err.contains("a.jpg"), "{err}");
    // nothing changed: the JPG is still the shot, no orphan PNG for the next sync
    assert_eq!(images_in(&dir), HashSet::from(["a.jpg".to_string()]));
    replace_with_png(&dir, "a.jpg", "a.png", b"new", |p| std::fs::remove_file(p)).unwrap();
    assert_eq!(images_in(&dir), HashSet::from(["a.png".to_string()]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn jpg_overwrite_through_save_edit_swaps_the_file_and_bumps_rev() {
    let dir = tmpdir("jpg2");
    let st = state_for(&dir);
    image::RgbImage::from_pixel(10, 10, image::Rgb([1, 2, 3]))
        .save(dir.join("shot.jpg"))
        .unwrap();
    sync_library_internal(&st).unwrap();
    let id = st.lib.lock().unwrap().shots[0].id;
    let dto = save_edit_op(&st, id, &png_b64(10, 7), "overwrite").unwrap();
    assert_eq!((dto.file.as_str(), dto.rev), ("shot.png", 1));
    let lib = assert_index_matches_folder(&dir);
    assert_eq!(lib.get(id).unwrap().rev, 1);
    // same size and dimensions again: only rev tells the thumbnail cache apart
    let dto2 = save_edit_op(&st, id, &png_b64(10, 7), "overwrite").unwrap();
    assert_eq!((dto2.size_bytes, dto2.rev), (dto.size_bytes, 2));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_only_folder_reports_instead_of_losing_silently() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmpdir("ro");
        let st = state_for(&dir);
        write_png(&dir.join("x.png"), 8, 1);
        sync_library_internal(&st).unwrap();
        let id = st.lib.lock().unwrap().shots[0].id;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let tag_err = set_tags_op(&st, id, &["a".into()]).unwrap_err();
        assert!(tag_err.contains("index.json"), "{tag_err}");
        let save_err = save_edit_op(&st, id, &png_b64(8, 2), "copy").unwrap_err();
        assert!(save_err.contains("x-edit.png"), "{save_err}");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn focus_sync_during_a_capture_does_not_adopt_the_new_file() {
    use std::sync::atomic::Ordering;
    let dir = tmpdir("focus");
    let st = state_for(&dir);
    st.capturing.store(true, Ordering::SeqCst);
    // screencapture has written the file, the window got focus back → sync
    write_png(&dir.join("screencap-new.png"), 12, 1);
    sync_op(&st).unwrap();
    assert!(st.lib.lock().unwrap().shots.is_empty(), "not adopted while capturing");
    let (id, r) = file_capture(&st, &dir, "screencap-new.png", ShotKind::Window);
    r.unwrap();
    st.capturing.store(false, Ordering::SeqCst);
    sync_op(&st).unwrap();
    let lib = assert_index_matches_folder(&dir);
    assert_eq!(lib.shots.len(), 1);
    assert_eq!(lib.get(id).unwrap().kind, ShotKind::Window);
    // a file copied in by hand later is adopted by the next focus sync
    write_png(&dir.join("dropped-in.png"), 12, 2);
    sync_op(&st).unwrap();
    let lib = assert_index_matches_folder(&dir);
    assert_eq!(lib.shots.iter().filter(|s| s.kind == ShotKind::Import).count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unchanged_sync_does_not_write_the_index() {
    let dir = tmpdir("nowrite");
    let st = state_for(&dir);
    write_png(&dir.join("a.png"), 8, 1);
    sync_op(&st).unwrap();
    assert!(dir.join("index.json").exists());
    std::fs::remove_file(dir.join("index.json")).unwrap();
    sync_op(&st).unwrap();
    assert!(!dir.join("index.json").exists(), "nothing changed → nothing written");
    let _ = std::fs::remove_dir_all(&dir);
}
