//! Capture pipeline: drives the native macOS `screencapture` tool, files the
//! result into the library and generates a thumbnail.

use crate::state::AppState;
use chrono::Local;
use screencap_core::ShotKind;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

pub const THUMB_MAX: u32 = 512;

/// Creates/refreshes the thumbnail for a source image; ignores failures
/// (the UI falls back to the full image).
pub fn make_thumb(src: &Path, thumb_path: &Path) {
    if let Ok(img) = image::open(src) {
        let thumb = img.thumbnail(THUMB_MAX, THUMB_MAX);
        let _ = thumb.save_with_format(thumb_path, image::ImageFormat::Png);
    }
}

fn unique_file_name(dir: &Path, stem: &str, ext: &str) -> String {
    let mut name = format!("{stem}.{ext}");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem}-{n}.{ext}");
        n += 1;
    }
    name
}

pub fn copy_image_to_clipboard(path: &Path) -> Result<(), String> {
    let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    let (w, h) = img.dimensions();
    let data = arboard::ImageData {
        width: w as usize,
        height: h as usize,
        bytes: std::borrow::Cow::Owned(img.into_raw()),
    };
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(data).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn run_screencapture(
    kind: ShotKind,
    delay: u32,
    format: &str,
    window_shadow: bool,
    target: &Path,
) -> Result<(), String> {
    let mut cmd = std::process::Command::new("/usr/sbin/screencapture");
    cmd.arg("-x"); // no shutter sound
    match kind {
        ShotKind::Region => {
            cmd.arg("-i");
        }
        ShotKind::Window => {
            cmd.arg("-i").arg("-W");
            if !window_shadow {
                cmd.arg("-o");
            }
        }
        _ => {} // full screen: no extra flags
    }
    if delay > 0 {
        cmd.arg("-T").arg(delay.to_string());
    }
    cmd.arg("-t").arg(format);
    cmd.arg(target);
    let status = cmd.status().map_err(|e| format!("screencapture: {e}"))?;
    if !status.success() {
        return Err("screencapture meldete einen Fehler".into());
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn run_screencapture(
    _kind: ShotKind,
    _delay: u32,
    _format: &str,
    _window_shadow: bool,
    _target: &Path,
) -> Result<(), String> {
    Err("Aufnahme ist bisher nur unter macOS umgesetzt".into())
}

/// Runs a capture and files the result. Returns None if the user cancelled.
/// Blocking — call from a worker thread / spawn_blocking.
pub fn do_capture(app: &AppHandle, kind: ShotKind, delay: Option<u32>) -> Result<Option<Uuid>, String> {
    let st = app.state::<AppState>();
    let (format, window_shadow, default_delay, copy_after, open_editor) = {
        let s = st.settings.lock().unwrap();
        (
            if s.format == "jpg" { "jpg".to_string() } else { "png".to_string() },
            s.window_shadow,
            s.delay_default,
            s.copy_after_capture,
            s.open_editor_after,
        )
    };
    let delay = delay.unwrap_or(default_delay);
    let dir = st.lib_dir();
    let stem = Local::now().format("screencap-%Y%m%d-%H%M%S").to_string();
    let file = unique_file_name(&dir, &stem, &format);
    let target = dir.join(&file);

    // get our own window out of the shot
    let win = app.get_webview_window("main");
    let was_visible = win
        .as_ref()
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);
    if was_visible {
        if let Some(w) = &win {
            let _ = w.hide();
        }
        std::thread::sleep(std::time::Duration::from_millis(350));
    }

    let result = run_screencapture(kind, delay, &format, window_shadow, &target);

    let captured = target.exists();
    if was_visible || (captured && open_editor) {
        if let Some(w) = &win {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
    result?;
    if !captured {
        return Ok(None); // user hit Esc
    }

    let (width, height) = image::image_dimensions(&target).unwrap_or((0, 0));
    let size_bytes = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    let id = {
        let mut lib = st.lib.lock().unwrap();
        lib.add(&file, kind, width, height, size_bytes)
    };
    make_thumb(&target, &st.thumbs_dir().join(format!("{id}.png")));
    st.persist();
    let _ = app.emit("library-changed", ());
    if copy_after {
        let _ = copy_image_to_clipboard(&target);
    }
    if open_editor {
        let _ = app.emit("open-editor", id);
    }
    Ok(Some(id))
}
