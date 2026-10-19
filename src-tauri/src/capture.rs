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
        write_thumb(&img, thumb_path);
    }
}

/// Scales an already decoded image down and writes the thumbnail via temp
/// file + rename — `get_thumb` may read the same thumbnail concurrently and
/// must never see a half-written PNG.
pub fn write_thumb(img: &image::DynamicImage, thumb_path: &Path) {
    let thumb = img.thumbnail(THUMB_MAX, THUMB_MAX);
    let tmp = thumb_path.with_extension(format!("{}.tmp", Uuid::new_v4().simple()));
    if thumb.save_with_format(&tmp, image::ImageFormat::Png).is_ok()
        && screencap_core::rename_replace(&tmp, thumb_path).is_ok()
    {
        return;
    }
    let _ = std::fs::remove_file(&tmp);
}

pub fn unique_file_name(dir: &Path, stem: &str, ext: &str) -> String {
    let mut name = format!("{stem}.{ext}");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem}-{n}.{ext}");
        n += 1;
    }
    name
}

pub fn copy_image_to_clipboard(path: &Path) -> Result<(), String> {
    let img = image::open(path)
        .map_err(|e| {
            crate::settings::tr(format!("Bild nicht lesbar: {e}"), format!("Cannot read image: {e}"))
        })?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let data = arboard::ImageData {
        width: w as usize,
        height: h as usize,
        bytes: std::borrow::Cow::Owned(img.into_raw()),
    };
    let clip_err = |e: arboard::Error| {
        crate::settings::tr(
            format!("Zwischenablage nicht verfügbar: {e}"),
            format!("Clipboard not available: {e}"),
        )
    };
    let mut cb = arboard::Clipboard::new().map_err(clip_err)?;
    cb.set_image(data).map_err(clip_err)
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
    let status = cmd.status().map_err(|e| {
        crate::settings::tr(
            format!("screencapture ließ sich nicht starten: {e}"),
            format!("Could not start screencapture: {e}"),
        )
    })?;
    if !status.success() {
        return Err(crate::settings::tr(
            "screencapture meldete einen Fehler",
            "screencapture reported an error",
        ));
    }
    Ok(())
}

/// Region the selection overlay reported back (physical pixels, relative to
/// the captured monitor). `None` = the user cancelled (Esc / empty drag).
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub struct RegionSel {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Error from the Windows capture backend (xcap / image encoding), prefixed
/// in the UI language — xcap's own messages are English only.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn cap_err(e: impl std::fmt::Display) -> String {
    crate::settings::tr(format!("Aufnahme fehlgeschlagen: {e}"), format!("Capture failed: {e}"))
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
        let all = xcap::Monitor::all().map_err(cap_err)?;
        all.iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .cloned()
            .or_else(|| all.into_iter().next())
            .ok_or_else(|| crate::settings::tr("Kein Bildschirm gefunden", "No screen found"))
    };

    let img: image::RgbaImage = match kind {
        ShotKind::Screen => {
            wait(delay);
            monitor_under_cursor()?
                .capture_image()
                .map_err(cap_err)?
        }
        ShotKind::Window => {
            wait(delay);
            let own_pid = std::process::id();
            let windows = xcap::Window::all().map_err(cap_err)?;
            let win = windows
                .into_iter()
                .filter(|w| w.pid().map(|p| p != own_pid).unwrap_or(true))
                .filter(|w| !w.is_minimized().unwrap_or(false))
                .find(|w| w.is_focused().unwrap_or(false))
                .ok_or_else(|| crate::settings::tr("Kein Fenster im Vordergrund", "No window in the foreground"))?;
            win.capture_image().map_err(cap_err)?
        }
        _ => {
            // region: transparent overlay on the monitor under the cursor,
            // the frontend reports the dragged rectangle via `region_result`
            let monitor = monitor_under_cursor()?;
            let (mx, my) = (
                monitor.x().map_err(cap_err)?,
                monitor.y().map_err(cap_err)?,
            );
            let (mw, mh) = (
                monitor.width().map_err(cap_err)?,
                monitor.height().map_err(cap_err)?,
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
                .map_err(|e| {
                    crate::settings::tr(
                        format!("Auswahl-Overlay ließ sich nicht öffnen: {e}"),
                        format!("Could not open the selection overlay: {e}"),
                    )
                })?;
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
                .map_err(cap_err)?
        }
    };

    let dyn_img = image::DynamicImage::ImageRgba8(img);
    if format == "jpg" {
        dyn_img
            .to_rgb8()
            .save_with_format(target, image::ImageFormat::Jpeg)
            .map_err(cap_err)
    } else {
        dyn_img
            .save_with_format(target, image::ImageFormat::Png)
            .map_err(cap_err)
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
    Err(crate::settings::tr(
        "Aufnahme ist bisher nur unter macOS und Windows umgesetzt",
        "Capturing is only implemented on macOS and Windows so far",
    ))
}

/// What a finished capture run means: `Ok(true)` = a file to file away,
/// `Ok(false)` = nothing to do (cancelled), `Err` = a real failure.
/// macOS: `screencapture -i` exits non-zero when the user presses Esc, so for
/// the interactive kinds a missing file is a cancel, not an error
/// (`exit_code_means_cancel`). Windows: a cancel in the overlay returns
/// `Ok(())` without a file — an `Err` there (no foreground window, overlay
/// or xcap failure) is real and must reach the user.
pub fn capture_outcome(
    kind: ShotKind,
    captured: bool,
    result: Result<(), String>,
    exit_code_means_cancel: bool,
) -> Result<bool, String> {
    if captured {
        return Ok(true);
    }
    match (kind, result) {
        (ShotKind::Region | ShotKind::Window, Err(_)) if exit_code_means_cancel => Ok(false),
        (_, Err(e)) => Err(e),
        (_, Ok(())) => Ok(false),
    }
}

/// Clears the "capture running" flag when a capture ends (also on early
/// returns and errors).
struct CaptureGuard<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for CaptureGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Runs a capture and files the result. Returns None if the user cancelled.
/// Blocking — call from a worker thread / spawn_blocking.
pub fn do_capture(app: &AppHandle, kind: ShotKind, delay: Option<u32>) -> Result<Option<Uuid>, String> {
    let st = app.state::<AppState>();
    // one capture at a time: a second shortcut press while the region
    // overlay / screencapture is still open would otherwise race for the
    // overlay window and the result channel
    if st.capturing.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Ok(None);
    }
    let _guard = CaptureGuard(&st.capturing);
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
    let delay = delay.unwrap_or(default_delay).min(crate::settings::MAX_DELAY);
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
    if !capture_outcome(kind, captured, result, cfg!(target_os = "macos"))? {
        return Ok(None); // user hit Esc
    }

    let (width, height) = image::image_dimensions(&target).unwrap_or((0, 0));
    let size_bytes = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    let id = {
        let mut lib = st.lib.lock().unwrap();
        lib.add(&file, kind, width, height, size_bytes)
    };
    make_thumb(&target, &st.thumbs_dir().join(format!("{id}.png")));
    // the capture itself worked — report follow-up problems, don't fail it
    if let Err(e) = st.persist() {
        let _ = app.emit("capture-error", e);
    }
    let _ = app.emit("library-changed", ());
    if copy_after {
        if let Err(e) = copy_image_to_clipboard(&target) {
            let _ = app.emit("capture-error", e);
        }
    }
    if open_editor {
        let _ = app.emit("open-editor", id);
    }
    Ok(Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_on_interactive_capture_is_a_cancel_on_macos() {
        let err = || Err("screencapture meldete einen Fehler".to_string());
        assert_eq!(capture_outcome(ShotKind::Region, false, err(), true), Ok(false));
        assert_eq!(capture_outcome(ShotKind::Window, false, err(), true), Ok(false));
        assert_eq!(capture_outcome(ShotKind::Region, false, Ok(()), true), Ok(false));
    }

    #[test]
    fn windows_interactive_errors_are_reported() {
        // overlay cancel = Ok(()) without file; an Err is a real failure
        assert_eq!(capture_outcome(ShotKind::Region, false, Ok(()), false), Ok(false));
        assert!(capture_outcome(ShotKind::Window, false, Err("no window".into()), false).is_err());
        assert!(capture_outcome(ShotKind::Region, false, Err("overlay".into()), false).is_err());
    }

    #[test]
    fn screen_failure_is_reported() {
        for mac in [true, false] {
            assert!(capture_outcome(ShotKind::Screen, false, Err("x".into()), mac).is_err());
            assert_eq!(capture_outcome(ShotKind::Screen, false, Ok(()), mac), Ok(false));
        }
    }

    #[test]
    fn a_written_file_always_counts() {
        assert_eq!(capture_outcome(ShotKind::Screen, true, Err("x".into()), false), Ok(true));
        assert_eq!(capture_outcome(ShotKind::Region, true, Ok(()), true), Ok(true));
    }

    #[test]
    fn unique_names_count_up() {
        let dir = std::env::temp_dir().join(format!("screencap-cap-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_file_name(&dir, "shot", "png"), "shot.png");
        std::fs::write(dir.join("shot.png"), b"x").unwrap();
        assert_eq!(unique_file_name(&dir, "shot", "png"), "shot-2.png");
        std::fs::write(dir.join("shot-2.png"), b"x").unwrap();
        assert_eq!(unique_file_name(&dir, "shot", "png"), "shot-3.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn thumbnail_is_bounded() {
        let dir = std::env::temp_dir().join(format!("screencap-cap-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("big.png");
        image::RgbaImage::from_pixel(1600, 400, image::Rgba([10, 20, 30, 255])).save(&src).unwrap();
        let thumb = dir.join("t.png");
        make_thumb(&src, &thumb);
        let (w, h) = image::image_dimensions(&thumb).unwrap();
        assert_eq!((w, h), (THUMB_MAX, 128));
        // written via temp file + rename: nothing else left behind
        make_thumb(&src, &thumb);
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
        names.sort();
        assert_eq!(names, vec!["big.png", "t.png"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
