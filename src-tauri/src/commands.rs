use crate::capture::{self, make_thumb};
use crate::settings::{self, tr, Settings};
use crate::state::AppState;
use base64::Engine;
use screencap_core::{Filter, Shot, ShotKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShotDto {
    pub id: Uuid,
    pub file: String,
    pub name: String,
    pub kind: ShotKind,
    pub captured_at: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub tags: Vec<String>,
    pub favorite: bool,
}

fn to_dto(s: &Shot) -> ShotDto {
    ShotDto {
        id: s.id,
        file: s.file.clone(),
        name: s.name.clone(),
        kind: s.kind,
        captured_at: s.captured_at.to_rfc3339(),
        width: s.width,
        height: s.height,
        size_bytes: s.size_bytes,
        tags: s.tags.clone(),
        favorite: s.favorite,
    }
}

fn unknown_shot() -> String {
    tr("unbekannte Aufnahme", "unknown capture")
}

fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn mime_for(file: &str) -> &'static str {
    let lower = file.to_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}

// --- capture ---

#[tauri::command]
pub async fn capture(
    app: AppHandle,
    kind: ShotKind,
    delay: Option<u32>,
) -> Result<Option<Uuid>, String> {
    tauri::async_runtime::spawn_blocking(move || capture::do_capture(&app, kind, delay))
        .await
        .map_err(|e| e.to_string())?
}

/// Selection overlay (Windows region capture) reports the dragged rectangle
/// — or `null` on Esc — back to the waiting `do_capture`.
#[tauri::command]
pub fn region_result(st: State<'_, AppState>, sel: Option<capture::RegionSel>) {
    if let Some(tx) = st.region_tx.lock().unwrap().take() {
        let _ = tx.send(sel);
    }
}

// --- library ---

/// Runs blocking work (image decoding, base64 of multi-MB files, disk I/O
/// with Windows lock retries, clipboard) on the blocking pool. Synchronous
/// Tauri commands run on the main thread and would freeze the window.
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())?
}

fn shot_file(st: &AppState, id: Uuid) -> Result<String, String> {
    let lib = st.lib.lock().unwrap();
    Ok(lib.get(id).ok_or_else(unknown_shot)?.file.clone())
}

fn read_err(file: &str, e: std::io::Error) -> String {
    tr(format!("„{file}“ nicht lesbar: {e}"), format!("Cannot read “{file}”: {e}"))
}

#[tauri::command]
pub fn list_shots(st: State<'_, AppState>, query: String, filter: Filter) -> Vec<ShotDto> {
    let lib = st.lib.lock().unwrap();
    lib.search(&query, filter).into_iter().map(to_dto).collect()
}

#[tauri::command]
pub async fn get_thumb(app: AppHandle, id: Uuid) -> Result<String, String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let thumb_path = st.thumbs_dir().join(format!("{id}.png"));
        if !thumb_path.exists() {
            // one thumbnail at a time: a freshly adopted folder with hundreds
            // of big images would otherwise decode them all in parallel
            // (blocking pool) and spike the memory
            static THUMB_GEN: std::sync::Mutex<()> = std::sync::Mutex::new(());
            let _one = THUMB_GEN.lock().unwrap_or_else(|e| e.into_inner());
            if !thumb_path.exists() {
                let file = shot_file(&st, id)?;
                make_thumb(&st.lib_dir().join(&file), &thumb_path);
            }
        }
        let data = std::fs::read(&thumb_path).map_err(|e| e.to_string())?;
        Ok(format!("data:image/png;base64,{}", b64().encode(data)))
    })
    .await
}

#[tauri::command]
pub async fn get_image(app: AppHandle, id: Uuid) -> Result<String, String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let file = shot_file(&st, id)?;
        let data = std::fs::read(st.lib_dir().join(&file)).map_err(|e| read_err(&file, e))?;
        Ok(format!("data:{};base64,{}", mime_for(&file), b64().encode(data)))
    })
    .await
}

fn changed(app: &AppHandle) -> Result<(), String> {
    app.state::<AppState>().persist()?;
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn rename_shot(app: AppHandle, id: Uuid, name: String) -> Result<(), String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let _io = st.io_lock();
        if !st.lib.lock().unwrap().rename(id, &name) {
            return Err(tr("Name darf nicht leer sein", "Name must not be empty"));
        }
        changed(app)
    })
    .await
}

#[tauri::command]
pub async fn set_tags(app: AppHandle, id: Uuid, tags: Vec<String>) -> Result<(), String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let _io = st.io_lock();
        st.lib.lock().unwrap().set_tags(id, &tags);
        changed(app)
    })
    .await
}

#[tauri::command]
pub async fn set_favorite(app: AppHandle, id: Uuid, favorite: bool) -> Result<(), String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let _io = st.io_lock();
        st.lib.lock().unwrap().set_favorite(id, favorite);
        changed(app)
    })
    .await
}

#[tauri::command]
pub async fn delete_shot(app: AppHandle, id: Uuid) -> Result<(), String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let _io = st.io_lock();
        let file = shot_file(&st, id)?;
        let path = st.lib_dir().join(&file);
        // Windows: a file another program holds open can't be deleted (retried
        // briefly for scanners/indexers) — keep the index entry then, otherwise
        // the next sync would re-adopt it as import
        if let Err(e) = screencap_core::retry_io(|| std::fs::remove_file(&path)) {
            if path.exists() {
                return Err(tr(
                    format!("Datei konnte nicht gelöscht werden: {e}"),
                    format!("File could not be deleted: {e}"),
                ));
            }
        }
        st.lib.lock().unwrap().remove(id);
        let _ = std::fs::remove_file(st.thumbs_dir().join(format!("{id}.png")));
        changed(app)
    })
    .await
}

#[tauri::command]
pub fn reveal_shot(st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    let file = shot_file(&st, id)?;
    tauri_plugin_opener::reveal_item_in_dir(st.lib_dir().join(&file)).map_err(|e| {
        tr(format!("Ordner konnte nicht geöffnet werden: {e}"), format!("Could not open the folder: {e}"))
    })
}

#[tauri::command]
pub async fn copy_shot(app: AppHandle, id: Uuid) -> Result<(), String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let file = shot_file(&st, id)?;
        capture::copy_image_to_clipboard(&st.lib_dir().join(file))
    })
    .await
}

/// File an overwrite-save lands in: the same file for PNGs (whatever the case
/// of the extension); for a JPG the PNG sibling — unless that name already
/// belongs to another file, then a free `-2`, `-3` … name (never clobber
/// another capture). `exists()` is case-insensitive on APFS/NTFS, so
/// `Shot.PNG` vs. `shot.png` count as the same name there.
pub fn overwrite_target(dir: &std::path::Path, stem: &str, old_file: &str) -> String {
    if old_file.to_ascii_lowercase().ends_with(".png") {
        return old_file.to_string();
    }
    let png = format!("{stem}.png");
    if !dir.join(&png).exists() {
        png
    } else {
        capture::unique_file_name(dir, stem, "png")
    }
}

/// File a save-as-copy lands in: `<stem>-edit.png`, `<stem>-edit-2.png`, …
pub fn copy_target(dir: &std::path::Path, stem: &str) -> String {
    let mut n = 1;
    let mut new_file = format!("{stem}-edit.png");
    while dir.join(&new_file).exists() {
        n += 1;
        new_file = format!("{stem}-edit-{n}.png");
    }
    new_file
}

/// Writes via a hidden temp file + rename so a crash never leaves a half
/// written capture (the temp name starts with "." and is ignored by sync).
/// The rename retries on Windows sharing violations (scanner, OneDrive,
/// an image viewer that has the file open).
pub fn write_atomic(dir: &std::path::Path, file: &str, data: &[u8]) -> Result<(), String> {
    let tmp = dir.join(format!(".{file}.tmp"));
    let fail = |e: std::io::Error| {
        tr(format!("„{file}“ nicht gespeichert: {e}"), format!("“{file}” not saved: {e}"))
    };
    std::fs::write(&tmp, data).map_err(fail)?;
    screencap_core::rename_replace(&tmp, &dir.join(file)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e)
    })
}

/// Saves an edited PNG. `mode` is "overwrite" or "copy".
#[tauri::command]
pub async fn save_edit(app: AppHandle, id: Uuid, png_base64: String, mode: String) -> Result<ShotDto, String> {
    blocking(app, move |app| save_edit_blocking(app, id, png_base64, mode)).await
}

fn save_edit_blocking(app: &AppHandle, id: Uuid, png_base64: String, mode: String) -> Result<ShotDto, String> {
    let st = app.state::<AppState>();
    let _io = st.io_lock();
    if mode != "overwrite" && mode != "copy" {
        return Err(tr("unbekannter Speichermodus", "unknown save mode"));
    }
    let not_png = || tr("Editor-Daten sind kein PNG", "Editor data is not a PNG");
    let data = b64().decode(png_base64).map_err(|_| not_png())?;
    if image::guess_format(&data).ok() != Some(image::ImageFormat::Png) {
        return Err(not_png());
    }
    let img = image::load_from_memory_with_format(&data, image::ImageFormat::Png).map_err(|_| not_png())?;
    let (width, height) = (img.width(), img.height());
    let dir = st.lib_dir();
    let (old_file, old_name) = {
        let lib = st.lib.lock().unwrap();
        let s = lib.get(id).ok_or_else(unknown_shot)?;
        (s.file.clone(), s.name.clone())
    };
    let stem = std::path::Path::new(&old_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("shot")
        .to_string();

    let result_id = if mode == "overwrite" {
        // edits are always saved as PNG; swap the file if it wasn't one
        let new_file = overwrite_target(&dir, &stem, &old_file);
        write_atomic(&dir, &new_file, &data)?;
        if new_file != old_file {
            let _ = screencap_core::retry_io(|| std::fs::remove_file(dir.join(&old_file)));
        }
        {
            let mut lib = st.lib.lock().unwrap();
            if let Some(s) = lib.shots.iter_mut().find(|s| s.id == id) {
                s.file = new_file.clone();
                s.width = width;
                s.height = height;
                s.size_bytes = data.len() as u64;
            }
        }
        capture::write_thumb(&img, &st.thumbs_dir().join(format!("{id}.png")));
        id
    } else {
        let new_file = copy_target(&dir, &stem);
        write_atomic(&dir, &new_file, &data)?;
        let new_id = {
            let mut lib = st.lib.lock().unwrap();
            let nid = lib.add(&new_file, ShotKind::Edit, width, height, data.len() as u64);
            lib.rename(nid, &format!("{old_name} (edit)"));
            nid
        };
        capture::write_thumb(&img, &st.thumbs_dir().join(format!("{new_id}.png")));
        new_id
    };
    changed(app)?;
    let lib = st.lib.lock().unwrap();
    lib.get(result_id)
        .map(to_dto)
        .ok_or_else(|| tr("Speichern fehlgeschlagen", "Saving failed"))
}

/// Name an imported file gets in the library: portable stem (see
/// `sanitize_stem`), lowercase extension, never an existing name.
pub fn import_target(dir: &std::path::Path, src: &std::path::Path) -> Option<String> {
    let ext = src.extension().and_then(|e| e.to_str())?.to_lowercase();
    if !["png", "jpg", "jpeg"].contains(&ext.as_str()) {
        return None;
    }
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("import");
    let stem = screencap_core::sanitize_stem(stem, "import");
    Some(capture::unique_file_name(dir, &stem, &ext))
}

#[tauri::command]
pub async fn import_files(app: AppHandle, paths: Vec<String>) -> Result<u32, String> {
    blocking(app, move |app| {
        let st = app.state::<AppState>();
        let _io = st.io_lock();
        let dir = st.lib_dir();
        let mut imported = 0u32;
        let mut last_err: Option<String> = None;
        for p in paths {
            let src = std::path::PathBuf::from(&p);
            if !src.is_file() {
                continue;
            }
            // only real images — the content decides, not the extension
            if image::image_dimensions(&src).is_err() {
                continue;
            }
            let Some(target_name) = import_target(&dir, &src) else {
                continue;
            };
            if let Err(e) = std::fs::copy(&src, dir.join(&target_name)) {
                last_err = Some(e.to_string());
                continue;
            }
            let (w, h) = image::image_dimensions(dir.join(&target_name)).unwrap_or((0, 0));
            let size = std::fs::metadata(dir.join(&target_name)).map(|m| m.len()).unwrap_or(0);
            let id = st.lib.lock().unwrap().add(&target_name, ShotKind::Import, w, h, size);
            make_thumb(&dir.join(&target_name), &st.thumbs_dir().join(format!("{id}.png")));
            imported += 1;
        }
        if imported > 0 {
            changed(app)?;
        } else if let Some(e) = last_err {
            return Err(tr(format!("Import fehlgeschlagen: {e}"), format!("Import failed: {e}")));
        }
        Ok(imported)
    })
    .await
}

/// Reconciles index and disk; files found on disk without an index entry are
/// adopted as imports. Called at startup and on demand. Only reads image
/// headers — thumbnails are generated lazily by `get_thumb`, so a folder
/// with hundreds of foreign files doesn't stall startup.
pub fn sync_library_internal(app: &AppHandle) -> Result<(), String> {
    let st = app.state::<AppState>();
    let on_disk = st.files_on_disk();
    let unknown = st.lib.lock().unwrap().sync(&on_disk);
    let dir = st.lib_dir();
    for file in unknown {
        let (w, h) = image::image_dimensions(dir.join(&file)).unwrap_or((0, 0));
        let size = std::fs::metadata(dir.join(&file)).map(|m| m.len()).unwrap_or(0);
        st.lib.lock().unwrap().add(&file, ShotKind::Import, w, h, size);
    }
    changed(app)
}

#[tauri::command]
pub async fn sync_library(app: AppHandle) -> Result<(), String> {
    blocking(app, |app| {
        let st = app.state::<AppState>();
        let _io = st.io_lock();
        sync_library_internal(app)
    })
    .await
}

#[tauri::command]
pub fn library_path(st: State<'_, AppState>) -> String {
    st.lib_dir().to_string_lossy().to_string()
}

// --- settings ---

#[tauri::command]
pub fn get_settings(st: State<'_, AppState>) -> Settings {
    st.settings.lock().unwrap().clone()
}

#[tauri::command]
pub async fn set_settings(app: AppHandle, new: Settings) -> Result<(), String> {
    blocking(app, move |app| set_settings_blocking(app, new)).await
}

fn set_settings_blocking(app: &AppHandle, new: Settings) -> Result<(), String> {
    let st = app.state::<AppState>();
    // whole switch under the library write lock: no rename/import/save/
    // capture filing can land between "settings point to the new folder"
    // and "the new folder's index is loaded"
    let _io = st.io_lock();
    let new = new.normalized();
    let old = st.settings.lock().unwrap().clone();
    new.validate_shortcuts()?;
    if let Some(e) = new.library_dir_error() {
        return Err(e);
    }

    // library folder must be creatable before anything is switched over
    let new_dir = new.resolved_library_dir();
    let dir_changes = old.resolved_library_dir() != new_dir;
    if dir_changes {
        // a running capture files its result into the current library
        if st.capturing.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(tr(
                "Während einer Aufnahme lässt sich der Ablage-Ordner nicht wechseln",
                "The library folder can't be changed while a capture is running",
            ));
        }
        std::fs::create_dir_all(&new_dir).map_err(|e| {
            tr(
                format!("Ablage-Ordner „{}“ nicht nutzbar: {e}", new_dir.display()),
                format!("Library folder “{}” not usable: {e}", new_dir.display()),
            )
        })?;
        if !new_dir.is_dir() {
            return Err(tr(
                format!("Ablage-Ordner „{}“ ist kein Ordner", new_dir.display()),
                format!("Library folder “{}” is not a folder", new_dir.display()),
            ));
        }
    }

    let shortcuts_changed = old.shortcut_region != new.shortcut_region
        || old.shortcut_window != new.shortcut_window
        || old.shortcut_screen != new.shortcut_screen;
    if shortcuts_changed {
        if let Err(e) = crate::register_shortcuts(app, &new) {
            // e.g. taken by another app — put the old ones back
            let _ = crate::register_shortcuts(app, &old);
            return Err(e);
        }
        *st.shortcut_error.lock().unwrap() = None;
    }

    if old.autostart != new.autostart {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        let res = if new.autostart {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        if let Err(e) = res {
            if shortcuts_changed {
                let _ = crate::register_shortcuts(app, &old);
            }
            return Err(tr(
                format!("Autostart nicht geändert: {e}"),
                format!("Start at login not changed: {e}"),
            ));
        }
    }

    if let Err(e) = settings::store(app, &new) {
        // nothing persisted — undo what was already switched over
        if shortcuts_changed {
            let _ = crate::register_shortcuts(app, &old);
        }
        if old.autostart != new.autostart {
            use tauri_plugin_autostart::ManagerExt;
            let _ = if old.autostart { app.autolaunch().enable() } else { app.autolaunch().disable() };
        }
        return Err(e);
    }
    *st.settings.lock().unwrap() = new.clone();
    settings::set_message_language(&new.language);

    if old.language != new.language {
        crate::update_tray_language(app, &new);
    }
    if dir_changes {
        // switch library: load (or start) the index in the new folder
        let lib = screencap_core::load_library_or_recover(&st.index_path());
        *st.lib.lock().unwrap() = lib;
        sync_library_internal(app)?;
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    /// No settings file existed at startup — the UI may adopt the OS language.
    pub first_run: bool,
    /// Global shortcuts that could not be registered (e.g. taken by another app).
    pub shortcut_error: Option<String>,
}

#[tauri::command]
pub fn app_status(st: State<'_, AppState>) -> AppStatusDto {
    AppStatusDto {
        first_run: st.first_run,
        shortcut_error: st.shortcut_error.lock().unwrap().clone(),
    }
}

// --- updates ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfoDto>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| {
        tr(format!("Update-Prüfung fehlgeschlagen: {e}"), format!("Update check failed: {e}"))
    })?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateInfoDto {
            version: update.version.clone(),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(tr(
            format!("Update-Prüfung fehlgeschlagen: {e}"),
            format!("Update check failed: {e}"),
        )),
    }
}

/// Clears the "update installing" flag again when the install fails.
struct UpdateGuard<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for UpdateGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let st = app.state::<AppState>();
    // banner and settings dialog both offer "install" — only one download
    if st.updating.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(tr("Update wird bereits installiert", "Update is already being installed"));
    }
    let _guard = UpdateGuard(&st.updating);
    let updater = app.updater().map_err(|e| {
        tr(format!("Update-Prüfung fehlgeschlagen: {e}"), format!("Update check failed: {e}"))
    })?;
    let update = updater
        .check()
        .await
        .map_err(|e| tr(format!("Update-Prüfung fehlgeschlagen: {e}"), format!("Update check failed: {e}")))?
        .ok_or_else(|| tr("Kein Update verfügbar", "No update available"))?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| tr(format!("Update fehlgeschlagen: {e}"), format!("Update failed: {e}")))?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("screencap-cmd-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn overwrite_png_keeps_its_file() {
        let d = tmpdir();
        std::fs::write(d.join("a.png"), b"x").unwrap();
        assert_eq!(overwrite_target(&d, "a", "a.png"), "a.png");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn overwrite_png_with_uppercase_extension_keeps_its_file() {
        let d = tmpdir();
        std::fs::write(d.join("Shot.PNG"), b"x").unwrap();
        // case-insensitive APFS/NTFS: "Shot.png" exists — must not become Shot-2.png
        assert_eq!(overwrite_target(&d, "Shot", "Shot.PNG"), "Shot.PNG");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn import_names_are_portable_and_unique() {
        let d = tmpdir();
        let t = import_target(&d, std::path::Path::new("/src/Bericht: Q1.PNG")).unwrap();
        assert_eq!(t, "Bericht- Q1.png");
        std::fs::write(d.join(&t), b"x").unwrap();
        assert_eq!(import_target(&d, std::path::Path::new("/src/Bericht: Q1.png")).unwrap(), "Bericht- Q1-2.png");
        assert_eq!(import_target(&d, std::path::Path::new("/src/CON.jpg")).unwrap(), "_CON.jpg");
        assert_eq!(import_target(&d, std::path::Path::new("/src/.versteckt.jpeg")).unwrap(), "versteckt.jpeg");
        assert_eq!(import_target(&d, std::path::Path::new("/src/Grüße Ä.jpg")).unwrap(), "Grüße Ä.jpg");
        assert!(import_target(&d, std::path::Path::new("/src/a.webp")).is_none());
        assert!(import_target(&d, std::path::Path::new("/src/noext")).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn atomic_write_reports_a_localized_error() {
        let d = tmpdir();
        let err = write_atomic(&d.join("missing-dir"), "c.png", b"x").unwrap_err();
        assert!(err.contains("c.png"), "{err}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn overwrite_jpg_never_clobbers_a_png_sibling() {
        let d = tmpdir();
        std::fs::write(d.join("a.jpg"), b"x").unwrap();
        assert_eq!(overwrite_target(&d, "a", "a.jpg"), "a.png");
        std::fs::write(d.join("a.png"), b"other capture").unwrap();
        assert_eq!(overwrite_target(&d, "a", "a.jpg"), "a-2.png");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn copies_count_up() {
        let d = tmpdir();
        assert_eq!(copy_target(&d, "a"), "a-edit.png");
        std::fs::write(d.join("a-edit.png"), b"x").unwrap();
        assert_eq!(copy_target(&d, "a"), "a-edit-2.png");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn atomic_write_leaves_no_temp() {
        let d = tmpdir();
        write_atomic(&d, "b.png", b"one").unwrap();
        write_atomic(&d, "b.png", b"two").unwrap();
        assert_eq!(std::fs::read(d.join("b.png")).unwrap(), b"two");
        let names: Vec<_> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(names.len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mime_types() {
        assert_eq!(mime_for("a.JPG"), "image/jpeg");
        assert_eq!(mime_for("a.jpeg"), "image/jpeg");
        assert_eq!(mime_for("a.png"), "image/png");
    }
}
