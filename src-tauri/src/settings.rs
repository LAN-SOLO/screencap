//! App settings, stored as JSON in the OS config directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// "de" | "en"
    pub language: String,
    /// Library folder; empty = default (~/Pictures/screencap).
    pub library_dir: String,
    /// "png" | "jpg"
    pub format: String,
    /// Default capture delay in seconds.
    pub delay_default: u32,
    /// Copy every capture to the clipboard as well.
    pub copy_after_capture: bool,
    /// Open the editor right after a capture.
    pub open_editor_after: bool,
    /// Keep the macOS window shadow on window captures.
    pub window_shadow: bool,
    /// Launch screencap at login.
    pub autostart: bool,
    pub shortcut_region: String,
    pub shortcut_window: String,
    pub shortcut_screen: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: if sys_locale_is_german() { "de" } else { "en" }.into(),
            library_dir: String::new(),
            format: "png".into(),
            delay_default: 0,
            copy_after_capture: false,
            open_editor_after: true,
            window_shadow: true,
            autostart: false,
            shortcut_region: "CmdOrCtrl+Shift+7".into(),
            shortcut_window: "CmdOrCtrl+Shift+8".into(),
            shortcut_screen: "CmdOrCtrl+Shift+9".into(),
        }
    }
}

impl Settings {
    pub fn resolved_library_dir(&self) -> PathBuf {
        if !self.library_dir.trim().is_empty() {
            return PathBuf::from(self.library_dir.trim());
        }
        dirs::picture_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("screencap")
    }
}

fn sys_locale_is_german() -> bool {
    std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .map(|l| l.to_lowercase().starts_with("de"))
        .unwrap_or(false)
}

fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}

pub fn load(app: &tauri::AppHandle) -> Settings {
    std::fs::read_to_string(settings_path(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn store(app: &tauri::AppHandle, settings: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(settings_path(app), json);
    }
}
