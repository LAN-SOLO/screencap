//! Capture pipeline: drives the native macOS `screencapture` tool (or, on
//! Windows, `xcap` plus an own selection overlay), files the result into the
//! library and generates a thumbnail.

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
    _app: &AppHandle,
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

/// Region the selection overlay reported back (physical pixels, relative to
/// the captured monitor). `None` = the user cancelled (Esc / empty drag).
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct RegionSel {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Windows: no system screenshot tool to drive, so capture via `xcap` — the
/// monitor under the cursor, the foreground window, or a region the user
/// drags out on a transparent full-screen overlay window (`region` label,
/// see `RegionPicker.tsx`).
#[cfg(target_os = "windows")]
fn run_screencapture(
    app: &AppHandle,
    kind: ShotKind,
    delay: u32,
    format: &str,
    _window_shadow: bool,
    target: &Path,
) -> Result<(), String> {
    use std::time::Duration;
    use tauri::{PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

    let wait = |secs: u32| {
        if secs > 0 {
            std::thread::sleep(Duration::from_secs(secs as u64));
        }
    };
    let monitor_under_cursor = || -> Result<xcap::Monitor, String> {
        if let Ok(pos) = app.cursor_position() {
            if let Ok(m) = xcap::Monitor::from_point(pos.x as i32, pos.y as i32) {
                return Ok(m);
            }
        }
        let all = xcap::Monitor::all().map_err(|e| e.to_string())?;
        all.iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .cloned()
            .or_else(|| all.into_iter().next())
            .ok_or_else(|| "Kein Bildschirm gefunden".to_string())
    };

    let img: image::RgbaImage = match kind {
        ShotKind::Screen => {
            wait(delay);
            monitor_under_cursor()?
                .capture_image()
                .map_err(|e| e.to_string())?
        }
        ShotKind::Window => {
            wait(delay);
            let own_pid = std::process::id();
            let windows = xcap::Window::all().map_err(|e| e.to_string())?;
            let win = windows
                .into_iter()
                .filter(|w| w.pid().map(|p| p != own_pid).unwrap_or(true))
                .filter(|w| !w.is_minimized().unwrap_or(false))
                .find(|w| w.is_focused().unwrap_or(false))
                .ok_or_else(|| "Kein Fenster im Vordergrund".to_string())?;
            win.capture_image().map_err(|e| e.to_string())?
        }
        _ => {
            // region: transparent overlay on the monitor under the cursor,
            // the frontend reports the dragged rectangle via `region_result`
            let monitor = monitor_under_cursor()?;
            let (mx, my) = (
                monitor.x().map_err(|e| e.to_string())?,
                monitor.y().map_err(|e| e.to_string())?,
            );
            let (mw, mh) = (
                monitor.width().map_err(|e| e.to_string())?,
                monitor.height().map_err(|e| e.to_string())?,
            );
            let st = app.state::<AppState>();
            let (tx, rx) = std::sync::mpsc::channel::<Option<RegionSel>>();
            *st.region_tx.lock().unwrap() = Some(tx);
            if let Some(old) = app.get_webview_window("region") {
                let _ = old.destroy();
            }
            let overlay = WebviewWindowBuilder::new(app, "region", WebviewUrl::App("index.html".into()))
                .title("screencap")
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .visible(false)
                .build()
                .map_err(|e| format!("Overlay: {e}"))?;
            let _ = overlay.set_position(PhysicalPosition::new(mx, my));
            let _ = overlay.set_size(PhysicalSize::new(mw, mh));
            let _ = overlay.show();
            let _ = overlay.set_focus();
            let sel = rx.recv_timeout(Duration::from_secs(180)).ok().flatten();
            *st.region_tx.lock().unwrap() = None;
            let _ = overlay.destroy();
            let Some(sel) = sel else {
                return Ok(()); // cancelled — no file, do_capture reports None
            };
            // let the compositor drop the overlay before we read the screen
            std::thread::sleep(Duration::from_millis(150));
            wait(delay);
            let w = sel.w.min(mw.saturating_sub(sel.x));
            let h = sel.h.min(mh.saturating_sub(sel.y));
            if w < 2 || h < 2 {
                return Ok(());
            }
            monitor
                .capture_region(sel.x, sel.y, w, h)
                .map_err(|e| e.to_string())?
        }
    };

    let dyn_img = image::DynamicImage::ImageRgba8(img);
    if format == "jpg" {
        dyn_img
            .to_rgb8()
            .save_with_format(target, image::ImageFormat::Jpeg)
            .map_err(|e| e.to_string())
    } else {
        dyn_img
            .save_with_format(target, image::ImageFormat::Png)
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn run_screencapture(
    _app: &AppHandle,
    _kind: ShotKind,
    _delay: u32,
    _format: &str,
    _window_shadow: bool,
    _target: &Path,
) -> Result<(), String> {
    Err("Aufnahme ist bisher nur unter macOS und Windows umgesetzt".into())
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

    let result = run_screencapture(app, kind, delay, &format, window_shadow, &target);

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
