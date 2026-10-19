use crate::capture::RegionSel;
use crate::settings::Settings;
use screencap_core::Library;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct AppState {
    pub lib: Mutex<Library>,
    pub settings: Mutex<Settings>,
    /// Set while a region capture waits for the selection overlay
    /// (Windows); the `region_result` command reports through it.
    pub region_tx: Mutex<Option<Sender<Option<RegionSel>>>>,
    /// Last global-shortcut registration error (shown in Settings).
    pub shortcut_error: Mutex<Option<String>>,
    /// No settings file existed at startup.
    pub first_run: bool,
    /// A capture is running (one at a time).
    pub capturing: AtomicBool,
    /// An update download/install is running (one at a time).
    pub updating: AtomicBool,
}

impl AppState {
    pub fn lib_dir(&self) -> PathBuf {
        let dir = self.settings.lock().unwrap().resolved_library_dir();
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    pub fn index_path(&self) -> PathBuf {
        self.lib_dir().join("index.json")
    }

    pub fn thumbs_dir(&self) -> PathBuf {
        let dir = self.lib_dir().join(".thumbs");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Writes index.json. A failure (read-only or vanished folder, file
    /// locked) is reported — otherwise names/tags/favorites would silently be
    /// lost at the next start.
    pub fn persist(&self) -> Result<(), String> {
        let path = self.index_path();
        let lib = self.lib.lock().unwrap();
        screencap_core::save_library(&lib, &path).map_err(|e| {
            crate::settings::tr(
                format!("Bibliothek nicht gespeichert (index.json): {e}"),
                format!("Library not saved (index.json): {e}"),
            )
        })
    }

    /// Image files (PNG/JPG, regular files only — no symlinked folders)
    /// currently in the library folder.
    pub fn files_on_disk(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(self.lib_dir()) {
            for e in entries.flatten() {
                let is_file = e.file_type().map(|t| t.is_file()).unwrap_or(false);
                let Some(name) = e.file_name().to_str().map(str::to_string) else {
                    continue; // non-UTF-8 names can't round-trip through the index
                };
                if is_file && screencap_core::is_library_image(&name) {
                    out.push(name);
                }
            }
        }
        out
    }
}
