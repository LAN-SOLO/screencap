use crate::capture::{self, make_thumb};
use crate::settings::{self, Settings};
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

#[tauri::command]
pub fn list_shots(st: State<'_, AppState>, query: String, filter: Filter) -> Vec<ShotDto> {
    let lib = st.lib.lock().unwrap();
    lib.search(&query, filter).into_iter().map(to_dto).collect()
}

#[tauri::command]
pub fn get_thumb(st: State<'_, AppState>, id: Uuid) -> Result<String, String> {
    let thumb_path = st.thumbs_dir().join(format!("{id}.png"));
    if !thumb_path.exists() {
        let file = {
            let lib = st.lib.lock().unwrap();
            lib.get(id).ok_or("unbekannte Aufnahme")?.file.clone()
        };
        make_thumb(&st.lib_dir().join(&file), &thumb_path);
    }
    let data = std::fs::read(&thumb_path).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", b64().encode(data)))
}

#[tauri::command]
pub fn get_image(st: State<'_, AppState>, id: Uuid) -> Result<String, String> {
    let file = {
        let lib = st.lib.lock().unwrap();
        lib.get(id).ok_or("unbekannte Aufnahme")?.file.clone()
    };
    let data = std::fs::read(st.lib_dir().join(&file)).map_err(|e| e.to_string())?;
    Ok(format!("data:{};base64,{}", mime_for(&file), b64().encode(data)))
}

#[tauri::command]
pub fn rename_shot(app: AppHandle, st: State<'_, AppState>, id: Uuid, name: String) -> Result<(), String> {
    if !st.lib.lock().unwrap().rename(id, &name) {
        return Err("Name darf nicht leer sein".into());
    }
    st.persist();
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_tags(app: AppHandle, st: State<'_, AppState>, id: Uuid, tags: Vec<String>) {
    st.lib.lock().unwrap().set_tags(id, &tags);
    st.persist();
    let _ = app.emit("library-changed", ());
}

#[tauri::command]
pub fn set_favorite(app: AppHandle, st: State<'_, AppState>, id: Uuid, favorite: bool) {
    st.lib.lock().unwrap().set_favorite(id, favorite);
    st.persist();
    let _ = app.emit("library-changed", ());
}

#[tauri::command]
pub fn delete_shot(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    let file = {
        let lib = st.lib.lock().unwrap();
        lib.get(id).ok_or("unbekannte Aufnahme")?.file.clone()
    };
    let path = st.lib_dir().join(&file);
    // Windows: a file another program holds open can't be deleted — keep the
    // index entry then, otherwise the next sync would re-adopt it as import
    if let Err(e) = std::fs::remove_file(&path) {
        if path.exists() {
            return Err(format!("Datei konnte nicht gelöscht werden: {e}"));
        }
    }
    st.lib.lock().unwrap().remove(id);
    let _ = std::fs::remove_file(st.thumbs_dir().join(format!("{id}.png")));
    st.persist();
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn reveal_shot(st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    let file = {
        let lib = st.lib.lock().unwrap();
        lib.get(id).ok_or("unbekannte Aufnahme")?.file.clone()
    };
    tauri_plugin_opener::reveal_item_in_dir(st.lib_dir().join(file)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_shot(st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    let file = {
        let lib = st.lib.lock().unwrap();
        lib.get(id).ok_or("unbekannte Aufnahme")?.file.clone()
    };
    capture::copy_image_to_clipboard(&st.lib_dir().join(file))
}

/// File an overwrite-save lands in: the same file for PNGs; for a JPG the
/// PNG sibling — unless that name already belongs to another file, then a
/// free `-2`, `-3` … name (never clobber another capture).
pub fn overwrite_target(dir: &std::path::Path, stem: &str, old_file: &str) -> String {
    let png = format!("{stem}.png");
    if png == old_file || !dir.join(&png).exists() {
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
fn write_atomic(dir: &std::path::Path, file: &str, data: &[u8]) -> Result<(), String> {
    let tmp = dir.join(format!(".{file}.tmp"));
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join(file)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// Saves an edited PNG. `mode` is "overwrite" or "copy".
#[tauri::command]
pub fn save_edit(
    app: AppHandle,
    st: State<'_, AppState>,
    id: Uuid,
    png_base64: String,
    mode: String,
) -> Result<ShotDto, String> {
    let data = b64().decode(png_base64).map_err(|e| e.to_string())?;
    if image::guess_format(&data).ok() != Some(image::ImageFormat::Png) {
        return Err("Editor-Daten sind kein PNG".into());
    }
    let (width, height) = image::load_from_memory_with_format(&data, image::ImageFormat::Png)
        .map(|i| (i.width(), i.height()))
        .map_err(|e| e.to_string())?;
    let dir = st.lib_dir();
    let (old_file, old_name) = {
        let lib = st.lib.lock().unwrap();
        let s = lib.get(id).ok_or("unbekannte Aufnahme")?;
        (s.file.clone(), s.name.clone())
    };
    let stem = std::path::Path::new(&old_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("shot")
        .to_string();

    if mode != "overwrite" && mode != "copy" {
        return Err("unbekannter Speichermodus".into());
    }
    let result_id;
    if mode == "overwrite" {
        // edits are always saved as PNG; swap the file if it wasn't one
        let new_file = overwrite_target(&dir, &stem, &old_file);
        write_atomic(&dir, &new_file, &data)?;
        if new_file != old_file {
            let _ = std::fs::remove_file(dir.join(&old_file));
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
        make_thumb(&dir.join(&new_file), &st.thumbs_dir().join(format!("{id}.png")));
        result_id = id;
    } else {
        let new_file = copy_target(&dir, &stem);
        write_atomic(&dir, &new_file, &data)?;
        let new_id = {
            let mut lib = st.lib.lock().unwrap();
            let nid = lib.add(&new_file, ShotKind::Edit, width, height, data.len() as u64);
            lib.rename(nid, &format!("{old_name} (edit)"));
            nid
        };
        make_thumb(&dir.join(&new_file), &st.thumbs_dir().join(format!("{new_id}.png")));
        result_id = new_id;
    }
    st.persist();
    let _ = app.emit("library-changed", ());
    let lib = st.lib.lock().unwrap();
    lib.get(result_id).map(to_dto).ok_or("Speichern fehlgeschlagen".into())
}

#[tauri::command]
pub fn import_files(app: AppHandle, st: State<'_, AppState>, paths: Vec<String>) -> Result<u32, String> {
    let dir = st.lib_dir();
    let mut imported = 0u32;
    for p in paths {
        let src = std::path::PathBuf::from(&p);
        let Some(file_name) = src.file_name().and_then(|f| f.to_str()) else {
            continue;
        };
        let stem = std::path::Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("import");
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["png", "jpg", "jpeg"].contains(&ext.as_str()) || !src.is_file() {
            continue;
        }
        // only real images — the content decides, not the extension
        if image::image_dimensions(&src).is_err() {
            continue;
        }
        // hidden names would be skipped by sync — strip leading dots
        // (and ':' — not allowed on Windows, rejected by the index check)
        let stem = stem.trim_start_matches('.').replace(':', "-");
        let stem = if stem.trim().is_empty() { "import".to_string() } else { stem.trim().to_string() };
        let target_name = capture::unique_file_name(&dir, &stem, &ext);
        if std::fs::copy(&src, dir.join(&target_name)).is_err() {
            continue;
        }
        let (w, h) = image::image_dimensions(dir.join(&target_name)).unwrap_or((0, 0));
        let size = std::fs::metadata(dir.join(&target_name)).map(|m| m.len()).unwrap_or(0);
        let id = st.lib.lock().unwrap().add(&target_name, ShotKind::Import, w, h, size);
        make_thumb(&dir.join(&target_name), &st.thumbs_dir().join(format!("{id}.png")));
        imported += 1;
    }
    if imported > 0 {
        st.persist();
        let _ = app.emit("library-changed", ());
    }
    Ok(imported)
}

/// Reconciles index and disk; files found on disk without an index entry are
/// adopted as imports. Called at startup and on demand.
pub fn sync_library_internal(app: &AppHandle) {
    let st = app.state::<AppState>();
    let on_disk = st.files_on_disk();
    let unknown = st.lib.lock().unwrap().sync(&on_disk);
    let dir = st.lib_dir();
    for file in unknown {
        let (w, h) = image::image_dimensions(dir.join(&file)).unwrap_or((0, 0));
        let size = std::fs::metadata(dir.join(&file)).map(|m| m.len()).unwrap_or(0);
        let id = st.lib.lock().unwrap().add(&file, ShotKind::Import, w, h, size);
        make_thumb(&dir.join(&file), &st.thumbs_dir().join(format!("{id}.png")));
    }
    st.persist();
    let _ = app.emit("library-changed", ());
}

#[tauri::command]
pub fn sync_library(app: AppHandle) {
    sync_library_internal(&app);
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
pub fn set_settings(app: AppHandle, st: State<'_, AppState>, new: Settings) -> Result<(), String> {
    let new = new.normalized();
    let old = st.settings.lock().unwrap().clone();
    new.validate_shortcuts()?;

    // library folder must be creatable before anything is switched over
    let new_dir = new.resolved_library_dir();
    if old.resolved_library_dir() != new_dir {
        std::fs::create_dir_all(&new_dir)
            .map_err(|e| format!("Ablage-Ordner „{}“ nicht nutzbar: {e}", new_dir.display()))?;
        if !new_dir.is_dir() {
            return Err(format!("Ablage-Ordner „{}“ ist kein Ordner", new_dir.display()));
        }
    }

    let shortcuts_changed = old.shortcut_region != new.shortcut_region
        || old.shortcut_window != new.shortcut_window
        || old.shortcut_screen != new.shortcut_screen;
    if shortcuts_changed {
        if let Err(e) = crate::register_shortcuts(&app, &new) {
            // e.g. taken by another app — put the old ones back
            let _ = crate::register_shortcuts(&app, &old);
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
                let _ = crate::register_shortcuts(&app, &old);
            }
            return Err(format!("Autostart: {e}"));
        }
    }

    settings::store(&app, &new)?;
    *st.settings.lock().unwrap() = new.clone();

    if old.language != new.language {
        crate::update_tray_language(&app, &new);
    }
    if old.resolved_library_dir() != new_dir {
        // switch library: load (or start) the index in the new folder
        let lib = screencap_core::load_library_or_recover(&st.index_path());
        *st.lib.lock().unwrap() = lib;
        sync_library_internal(&app);
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
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateInfoDto {
            version: update.version.clone(),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Update-Prüfung fehlgeschlagen: {e}")),
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("Update-Prüfung fehlgeschlagen: {e}"))?
        .ok_or("Kein Update verfügbar")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Update fehlgeschlagen: {e}"))?;
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
