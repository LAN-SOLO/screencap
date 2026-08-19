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
    let file = st.lib.lock().unwrap().remove(id).ok_or("unbekannte Aufnahme")?;
    let _ = std::fs::remove_file(st.lib_dir().join(&file));
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
    let (width, height) = image::load_from_memory(&data)
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

    let result_id;
    if mode == "overwrite" {
        // edits are always saved as PNG; swap the file if it wasn't one
        let new_file = format!("{stem}.png");
        std::fs::write(dir.join(&new_file), &data).map_err(|e| e.to_string())?;
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
        let mut n = 1;
        let mut new_file = format!("{stem}-edit.png");
        while dir.join(&new_file).exists() {
            n += 1;
            new_file = format!("{stem}-edit-{n}.png");
        }
        std::fs::write(dir.join(&new_file), &data).map_err(|e| e.to_string())?;
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
            .unwrap_or("png")
            .to_lowercase();
        if !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
            continue;
        }
        let mut target_name = format!("{stem}.{ext}");
        let mut n = 2;
        while dir.join(&target_name).exists() {
            target_name = format!("{stem}-{n}.{ext}");
            n += 1;
        }
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
    let old = st.settings.lock().unwrap().clone();
    settings::store(&app, &new);
    *st.settings.lock().unwrap() = new.clone();

    if old.autostart != new.autostart {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        let res = if new.autostart {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        res.map_err(|e| e.to_string())?;
    }
    if old.shortcut_region != new.shortcut_region
        || old.shortcut_window != new.shortcut_window
        || old.shortcut_screen != new.shortcut_screen
    {
        crate::register_shortcuts(&app, &new)?;
    }
    if old.resolved_library_dir() != new.resolved_library_dir() {
        // switch library: load (or start) the index in the new folder
        let idx = st.index_path();
        let lib = screencap_core::load_library(&idx).unwrap_or_default();
        *st.lib.lock().unwrap() = lib;
        sync_library_internal(&app);
    }
    Ok(())
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
