//! screencap core: the screenshot library index.
//!
//! Screenshots stay ordinary image files in the library folder — the index
//! (`index.json` next to them) only adds what the file system can't hold:
//! names, tags, favorites, capture kind. If files disappear or appear on
//! disk, `sync` reconciles the index instead of fighting it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShotKind {
    Region,
    Window,
    Screen,
    Import,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub id: Uuid,
    /// File name inside the library folder.
    pub file: String,
    /// Display name (defaults to the file stem).
    pub name: String,
    pub kind: ShotKind,
    pub captured_at: DateTime<Utc>,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Filter {
    All,
    Favorites,
    Region,
    Window,
    Screen,
    Edits,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Library {
    /// Newest first.
    pub shots: Vec<Shot>,
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = t.trim();
        if t.is_empty() {
            continue;
        }
        if !out.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            out.push(t.to_string());
        }
    }
    out
}

impl Library {
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        file: &str,
        kind: ShotKind,
        width: u32,
        height: u32,
        size_bytes: u64,
    ) -> Uuid {
        let stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file)
            .to_string();
        let shot = Shot {
            id: Uuid::new_v4(),
            file: file.to_string(),
            name: stem,
            kind,
            captured_at: Utc::now(),
            width,
            height,
            size_bytes,
            tags: Vec::new(),
            favorite: false,
        };
        let id = shot.id;
        self.shots.insert(0, shot);
        id
    }

    pub fn get(&self, id: Uuid) -> Option<&Shot> {
        self.shots.iter().find(|s| s.id == id)
    }

    pub fn rename(&mut self, id: Uuid, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        if let Some(s) = self.shots.iter_mut().find(|s| s.id == id) {
            s.name = name.to_string();
            true
        } else {
            false
        }
    }

    pub fn set_tags(&mut self, id: Uuid, tags: &[String]) {
        if let Some(s) = self.shots.iter_mut().find(|s| s.id == id) {
            s.tags = normalize_tags(tags);
        }
    }

    pub fn set_favorite(&mut self, id: Uuid, favorite: bool) {
        if let Some(s) = self.shots.iter_mut().find(|s| s.id == id) {
            s.favorite = favorite;
        }
    }

    /// Removes the shot from the index; returns its file name.
    pub fn remove(&mut self, id: Uuid) -> Option<String> {
        let pos = self.shots.iter().position(|s| s.id == id)?;
        Some(self.shots.remove(pos).file)
    }

    pub fn search(&self, query: &str, filter: Filter) -> Vec<&Shot> {
        let q = query.trim().to_lowercase();
        self.shots
            .iter()
            .filter(|s| match filter {
                Filter::All => true,
                Filter::Favorites => s.favorite,
                Filter::Region => s.kind == ShotKind::Region,
                Filter::Window => s.kind == ShotKind::Window,
                Filter::Screen => s.kind == ShotKind::Screen,
                Filter::Edits => s.kind == ShotKind::Edit,
            })
            .filter(|s| {
                q.is_empty()
                    || s.name.to_lowercase().contains(&q)
                    || s.file.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Reconciles the index with the files actually on disk.
    /// `on_disk` is the list of image file names in the library folder.
    /// Returns the file names that exist on disk but are not indexed yet
    /// (candidates for import).
    pub fn sync(&mut self, on_disk: &[String]) -> Vec<String> {
        self.shots.retain(|s| on_disk.contains(&s.file));
        on_disk
            .iter()
            .filter(|f| !self.shots.iter().any(|s| &s.file == *f))
            .cloned()
            .collect()
    }
}

/// A library file name must be a plain name inside the library folder — no
/// path separators, no `.`/`..`, no drive/stream colon (Windows), no NUL.
/// `index.json` is user-editable (and may live in a synced/shared folder),
/// so every entry is checked before its name is joined onto the folder.
pub fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':', '\0'])
        && name.trim() == name
}

/// Image files screencap manages (PNG/JPG). Hidden files (`.foo.png`, macOS
/// `._foo.png` AppleDouble files on external drives) are ignored.
pub fn is_library_image(name: &str) -> bool {
    if name.starts_with('.') || !is_safe_file_name(name) {
        return false;
    }
    let lower = name.to_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
}

pub fn save_library(lib: &Library, path: &Path) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(lib).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_library(path: &Path) -> Result<Library, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut lib: Library = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    // never trust file names from disk: drop anything that could escape the folder
    lib.shots.retain(|s| is_safe_file_name(&s.file));
    Ok(lib)
}

/// Loads the index; a missing file yields an empty library. A corrupt file is
/// moved aside as `index.corrupt-<timestamp>.json` (instead of being silently
/// overwritten by the next save), then an empty library is started.
pub fn load_library_or_recover(path: &Path) -> Library {
    if !path.exists() {
        return Library::default();
    }
    match load_library(path) {
        Ok(lib) => lib,
        Err(_) => {
            let stamp = Utc::now().format("%Y%m%d-%H%M%S");
            let backup = path.with_file_name(format!("index.corrupt-{stamp}.json"));
            let _ = std::fs::rename(path, backup);
            Library::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_search() {
        let mut lib = Library::default();
        lib.add("screencap-1.png", ShotKind::Region, 800, 600, 1234);
        let id = lib.add("screencap-2.png", ShotKind::Window, 400, 300, 999);
        lib.rename(id, "Login-Dialog");
        lib.set_tags(id, &["bug".into(), "Login".into()]);
        assert_eq!(lib.search("", Filter::All).len(), 2);
        assert_eq!(lib.search("login", Filter::All).len(), 1);
        assert_eq!(lib.search("BUG", Filter::All).len(), 1);
        assert_eq!(lib.search("", Filter::Window).len(), 1);
        assert_eq!(lib.search("nada", Filter::All).len(), 0);
    }

    #[test]
    fn newest_first() {
        let mut lib = Library::default();
        lib.add("a.png", ShotKind::Screen, 1, 1, 1);
        lib.add("b.png", ShotKind::Screen, 1, 1, 1);
        assert_eq!(lib.shots[0].file, "b.png");
    }

    #[test]
    fn tags_normalized() {
        let mut lib = Library::default();
        let id = lib.add("a.png", ShotKind::Region, 1, 1, 1);
        lib.set_tags(id, &[" bug ".into(), "Bug".into(), "".into(), "ui".into()]);
        assert_eq!(lib.get(id).unwrap().tags, vec!["bug", "ui"]);
    }

    #[test]
    fn favorites_filter() {
        let mut lib = Library::default();
        let id = lib.add("a.png", ShotKind::Region, 1, 1, 1);
        lib.add("b.png", ShotKind::Region, 1, 1, 1);
        lib.set_favorite(id, true);
        let favs = lib.search("", Filter::Favorites);
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].file, "a.png");
    }

    #[test]
    fn rename_rejects_empty() {
        let mut lib = Library::default();
        let id = lib.add("a.png", ShotKind::Region, 1, 1, 1);
        assert!(!lib.rename(id, "   "));
        assert_eq!(lib.get(id).unwrap().name, "a");
    }

    #[test]
    fn sync_drops_missing_and_reports_unknown() {
        let mut lib = Library::default();
        lib.add("keep.png", ShotKind::Region, 1, 1, 1);
        lib.add("gone.png", ShotKind::Region, 1, 1, 1);
        let unknown = lib.sync(&["keep.png".into(), "new.png".into()]);
        assert_eq!(lib.shots.len(), 1);
        assert_eq!(lib.shots[0].file, "keep.png");
        assert_eq!(unknown, vec!["new.png".to_string()]);
    }

    #[test]
    fn save_load_roundtrip() {
        let mut lib = Library::default();
        let id = lib.add("a.png", ShotKind::Edit, 10, 20, 30);
        lib.set_favorite(id, true);
        let dir = std::env::temp_dir().join(format!("screencap-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        save_library(&lib, &path).unwrap();
        let loaded = load_library(&path).unwrap();
        assert_eq!(loaded.shots.len(), 1);
        assert!(loaded.shots[0].favorite);
        assert_eq!(loaded.shots[0].kind, ShotKind::Edit);
    }

    #[test]
    fn safe_file_names() {
        assert!(is_safe_file_name("screencap-20260101-120000.png"));
        assert!(is_safe_file_name("Bild mit Leerzeichen.jpg"));
        for bad in [
            "", ".", "..", "../x.png", "a/b.png", "..\\x.png", "C:x.png", "C:\\x.png",
            "x.png:stream", " x.png", "x\0.png",
        ] {
            assert!(!is_safe_file_name(bad), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn library_image_filter() {
        assert!(is_library_image("a.png"));
        assert!(is_library_image("B.JPG"));
        assert!(is_library_image("c.jpeg"));
        assert!(!is_library_image("index.json"));
        assert!(!is_library_image("d.webp"));
        assert!(!is_library_image("._a.png"));
        assert!(!is_library_image(".hidden.png"));
    }

    #[test]
    fn load_drops_traversal_entries() {
        let mut lib = Library::default();
        lib.add("ok.png", ShotKind::Region, 1, 1, 1);
        lib.add("evil.png", ShotKind::Region, 1, 1, 1);
        lib.shots[0].file = "../../.ssh/id_rsa".into();
        let dir = std::env::temp_dir().join(format!("screencap-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        save_library(&lib, &path).unwrap();
        let loaded = load_library(&path).unwrap();
        assert_eq!(loaded.shots.len(), 1);
        assert_eq!(loaded.shots[0].file, "ok.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_index_is_moved_aside() {
        let dir = std::env::temp_dir().join(format!("screencap-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        assert!(load_library_or_recover(&path).shots.is_empty()); // missing = empty
        std::fs::write(&path, b"{ not json").unwrap();
        let lib = load_library_or_recover(&path);
        assert!(lib.shots.is_empty());
        assert!(!path.exists());
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("index.corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_overwrites_existing_index() {
        // Windows: rename onto an existing file must replace it
        let dir = std::env::temp_dir().join(format!("screencap-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        let mut lib = Library::default();
        lib.add("a.png", ShotKind::Region, 1, 1, 1);
        save_library(&lib, &path).unwrap();
        lib.add("b.png", ShotKind::Region, 1, 1, 1);
        save_library(&lib, &path).unwrap();
        assert_eq!(load_library(&path).unwrap().shots.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_returns_file() {
        let mut lib = Library::default();
        let id = lib.add("a.png", ShotKind::Region, 1, 1, 1);
        assert_eq!(lib.remove(id), Some("a.png".to_string()));
        assert!(lib.shots.is_empty());
        assert_eq!(lib.remove(id), None);
    }
}
