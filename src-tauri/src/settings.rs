//! App settings, stored as JSON in the OS config directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// UI language for messages produced in Rust (errors shown as toasts or in
/// the settings dialog) — mirrors `Settings::language`.
static LANG_DE: AtomicBool = AtomicBool::new(false);

pub fn set_message_language(lang: &str) {
    LANG_DE.store(lang == "de", Ordering::Relaxed);
}

/// Picks the German or English variant of a user-facing message.
pub fn tr(de: impl Into<String>, en: impl Into<String>) -> String {
    if LANG_DE.load(Ordering::Relaxed) {
        de.into()
    } else {
        en.into()
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
            let sc = Shortcut::from_str(c).map_err(|e| {
                tr(format!("Kürzel „{c}“ ungültig: {e}"), format!("Shortcut “{c}” is invalid: {e}"))
            })?;
            if parsed.contains(&sc) {
                return Err(tr(
                    format!("Kürzel „{c}“ ist doppelt belegt"),
                    format!("Shortcut “{c}” is used twice"),
                ));
            }
            parsed.push(sc);
        }
        Ok(())
    }

    pub fn resolved_library_dir(&self) -> PathBuf {
        let custom = self.library_dir.trim();
        if !custom.is_empty() {
            return expand_library_dir(custom, dirs::home_dir());
        }
        default_library_dir()
    }

    /// A typed-in library folder must be an absolute path (a relative one
    /// would depend on the working directory, which is `/` for an app started
    /// from Finder/Dock and `C:\Windows\System32` for autostart).
    pub fn library_dir_error(&self) -> Option<String> {
        let custom = self.library_dir.trim();
        if custom.is_empty() || self.resolved_library_dir().is_absolute() {
            return None;
        }
        Some(tr(
            format!("Ablage-Ordner „{custom}“ ist kein vollständiger Pfad (z. B. {})", example_dir()),
            format!("Library folder “{custom}” is not a full path (e.g. {})", example_dir()),
        ))
    }
}

fn default_library_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("screencap")
}

fn example_dir() -> &'static str {
    if cfg!(windows) {
        r"D:\Bilder\screencap"
    } else {
        "/Users/name/Pictures/screencap"
    }
}

/// `~` / `~/…` (also `~\…`) → home folder; a bare drive `D:` (which Windows
/// reads as "current folder on drive D") → the drive root `D:\`.
pub fn expand_library_dir(custom: &str, home: Option<PathBuf>) -> PathBuf {
    if let Some(home) = home {
        if custom == "~" {
            return home;
        }
        if let Some(rest) = custom.strip_prefix("~/").or_else(|| custom.strip_prefix("~\\")) {
            return home.join(rest);
        }
    }
    let b = custom.as_bytes();
    if b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return PathBuf::from(format!("{custom}\\"));
    }
    PathBuf::from(custom)
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
    (parse_settings(std::fs::read(&path).ok().as_deref()), existed)
}

/// settings.json → Settings; tolerates a UTF-8 BOM (hand-edited in Notepad),
/// falls back to the defaults for a missing or unreadable file.
pub fn parse_settings(data: Option<&[u8]>) -> Settings {
    data.and_then(|d| serde_json::from_slice::<Settings>(screencap_core::strip_bom(d)).ok())
        .unwrap_or_default()
        .normalized()
}

pub fn store(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    let path = settings_path(app);
    let tmp = path.with_extension("json.tmp");
    let fail = |e: std::io::Error| {
        tr(format!("Einstellungen nicht gespeichert: {e}"), format!("Settings not saved: {e}"))
    };
    std::fs::write(&tmp, json).map_err(fail)?;
    screencap_core::rename_replace(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e)
    })
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
        let err = dup.validate_shortcuts().unwrap_err();
        assert!(err.contains("doppelt") || err.contains("twice"), "{err}");
    }

    #[test]
    fn settings_json_with_bom_is_read() {
        let mut data = b"\xEF\xBB\xBF".to_vec();
        data.extend_from_slice(br#"{"language":"de","format":"jpg"}"#);
        let s = parse_settings(Some(&data));
        assert_eq!((s.language.as_str(), s.format.as_str()), ("de", "jpg"));
        assert_eq!(parse_settings(None).format, "png");
        assert_eq!(parse_settings(Some(b"{ kaputt")).format, "png");
    }

    #[test]
    fn library_dir_expansion() {
        let home = Some(PathBuf::from("/home/u"));
        assert_eq!(expand_library_dir("~", home.clone()), PathBuf::from("/home/u"));
        assert_eq!(expand_library_dir("~/Bilder", home.clone()), PathBuf::from("/home/u/Bilder"));
        assert_eq!(expand_library_dir("D:", home.clone()), PathBuf::from("D:\\"));
        assert_eq!(expand_library_dir("/x/y", home), PathBuf::from("/x/y"));
    }

    #[test]
    fn relative_library_dir_is_rejected() {
        let rel = Settings { library_dir: "screens".into(), ..Settings::default() };
        assert!(rel.library_dir_error().is_some());
        assert!(Settings::default().library_dir_error().is_none());
        let abs = if cfg!(windows) { r"D:\Bilder" } else { "/tmp/screens" };
        let ok = Settings { library_dir: abs.into(), ..Settings::default() };
        assert!(ok.library_dir_error().is_none());
        let home = Settings { library_dir: "~/screens".into(), ..Settings::default() };
        assert!(home.library_dir_error().is_none());
    }

    #[test]
    fn windows_style_library_dir_is_kept_verbatim() {
        let s = Settings { library_dir: r"D:\Bilder\screencap".into(), ..Settings::default() };
        assert_eq!(s.resolved_library_dir(), PathBuf::from(r"D:\Bilder\screencap"));
    }
}
