use crate::settings::Settings;
use screencap_core::Library;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub lib: Mutex<Library>,
    pub settings: Mutex<Settings>,
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

    pub fn persist(&self) {
        let lib = self.lib.lock().unwrap();
        let _ = screencap_core::save_library(&lib, &self.index_path());
    }

    /// Image files (by extension) currently in the library folder.
    pub fn files_on_disk(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(self.lib_dir()) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(".png")
                    || lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".webp")
                {
                    out.push(name);
                }
            }
        }
        out
    }
}
