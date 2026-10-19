//! App settings, stored as JSON in the OS config directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// "de" | "en"
    pub language: String,
    /// "dark" | "light" — Darstellung der Oberfläche (Default dark).
    #[serde(default = "default_theme")]
    pub theme: String,
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
            theme: default_theme(),
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

/// Longest delay the UI offers / the backend accepts (seconds).
pub const MAX_DELAY: u32 = 60;

impl Settings {
    /// Clamps every field to a value the app can handle — settings.json is
    /// user-editable and the frontend indexes its dictionaries by `language`.
    pub fn normalized(mut self) -> Self {
        if self.language != "de" && self.language != "en" {
            self.language = "en".into();
        }
        if self.theme != "light" {
            self.theme = "dark".into();
        }
        if self.format != "jpg" {
            self.format = "png".into();
        }
        self.delay_default = self.delay_default.min(MAX_DELAY);
        self.library_dir = self.library_dir.trim().to_string();
        self.shortcut_region = self.shortcut_region.trim().to_string();
        self.shortcut_window = self.shortcut_window.trim().to_string();
        self.shortcut_screen = self.shortcut_screen.trim().to_string();
        self
    }

    /// All three shortcuts must parse and differ from each other.
    pub fn validate_shortcuts(&self) -> Result<(), String> {
        use std::str::FromStr;
        use tauri_plugin_global_shortcut::Shortcut;
        let combos = [&self.shortcut_region, &self.shortcut_window, &self.shortcut_screen];
        let mut parsed = Vec::new();
        for c in combos {
            let sc = Shortcut::from_str(c).map_err(|e| format!("Kürzel „{c}“ ungültig: {e}"))?;
            if parsed.contains(&sc) {
                return Err(format!("Kürzel „{c}“ ist doppelt belegt"));
            }
            parsed.push(sc);
        }
        Ok(())
    }

    pub fn resolved_library_dir(&self) -> PathBuf {
        if !self.library_dir.trim().is_empty() {
            return PathBuf::from(self.library_dir.trim());
        }
        dirs::picture_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("screencap")
    }
}

fn default_theme() -> String {
    "dark".into()
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

/// Loads the settings; the flag says whether a settings file existed
/// (false = first run, the UI may then adopt the system language).
pub fn load(app: &tauri::AppHandle) -> (Settings, bool) {
    let path = settings_path(app);
    let existed = path.exists();
    let s: Settings = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    (s.normalized(), existed)
}

pub fn store(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    let path = settings_path(app);
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("Einstellungen nicht gespeichert: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Einstellungen nicht gespeichert: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bad_values() {
        let s = Settings {
            language: "fr".into(),
            theme: "neon".into(),
            format: "gif".into(),
            delay_default: 9999,
            library_dir: "  /tmp/x  ".into(),
            shortcut_region: " CmdOrCtrl+Shift+7 ".into(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(s.language, "en");
        assert_eq!(s.theme, "dark");
        assert_eq!(s.format, "png");
        assert_eq!(s.delay_default, MAX_DELAY);
        assert_eq!(s.library_dir, "/tmp/x");
        assert_eq!(s.shortcut_region, "CmdOrCtrl+Shift+7");
    }

    #[test]
    fn keeps_valid_values() {
        let s = Settings {
            language: "de".into(),
            theme: "light".into(),
            format: "jpg".into(),
            delay_default: 5,
            ..Settings::default()
        }
        .normalized();
        assert_eq!((s.language.as_str(), s.theme.as_str(), s.format.as_str()), ("de", "light", "jpg"));
        assert_eq!(s.delay_default, 5);
    }

    #[test]
    fn partial_json_gets_defaults() {
        let s: Settings = serde_json::from_str(r#"{"language":"de"}"#).unwrap();
        assert_eq!(s.format, "png");
        assert_eq!(s.shortcut_screen, "CmdOrCtrl+Shift+9");
    }

    #[test]
    fn default_shortcuts_are_valid() {
        assert!(Settings::default().validate_shortcuts().is_ok());
    }

    #[test]
    fn rejects_bad_and_duplicate_shortcuts() {
        let bad = Settings { shortcut_window: "Ctrl+Banana".into(), ..Settings::default() };
        assert!(bad.validate_shortcuts().is_err());
        let dup = Settings { shortcut_window: "CmdOrCtrl+Shift+7".into(), ..Settings::default() };
        assert!(dup.validate_shortcuts().unwrap_err().contains("doppelt"));
    }

    #[test]
    fn windows_style_library_dir_is_kept_verbatim() {
        let s = Settings { library_dir: r"D:\Bilder\screencap".into(), ..Settings::default() };
        assert_eq!(s.resolved_library_dir(), PathBuf::from(r"D:\Bilder\screencap"));
    }
}
