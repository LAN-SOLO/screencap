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
/// On Windows additionally everything the file system can't hold or treats
/// specially: `< > " | ? *`, control characters, a trailing dot and the
/// reserved device names (`CON`, `NUL.png`, `COM1.jpg` …).
pub fn is_safe_file_name(name: &str) -> bool {
    is_safe_file_name_for(name, cfg!(windows))
}

/// [`is_safe_file_name`] with the Windows rules switchable (for tests).
pub fn is_safe_file_name_for(name: &str, windows: bool) -> bool {
    let basic = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':', '\0'])
        && name.trim() == name;
    if !basic || !windows {
        return basic;
    }
    !name.contains(['<', '>', '"', '|', '?', '*'])
        && !name.chars().any(|c| c.is_control())
        && !name.ends_with('.')
        && !is_reserved_device_name(name)
}

/// Windows device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`,
/// `LPT1`–`LPT9`) — reserved with any extension and in every folder.
pub fn is_reserved_device_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT"))
            && base.len() == 4
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

/// Turns any file stem (e.g. of an imported file) into one that is valid on
/// macOS *and* Windows, so the library folder stays portable: characters
/// Windows rejects become `-`, leading dots (hidden) and trailing dots/spaces
/// are dropped, device names get a `_` prefix, the length is capped.
pub fn sanitize_stem(stem: &str, fallback: &str) -> String {
    let mapped: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .collect();
    let trimmed = mapped.trim().trim_start_matches('.').trim_end_matches(['.', ' ']).trim();
    // 120 characters keep `<stem>-edit-12.png` well inside every limit
    let mut out: String = trimmed.chars().take(120).collect();
    out = out.trim_end_matches(['.', ' ']).to_string();
    if out.is_empty() {
        return fallback.to_string();
    }
    if is_reserved_device_name(&out) {
        out.insert(0, '_');
    }
    out
}

/// Removes a leading UTF-8 byte order mark — Windows Notepad writes one when
/// a user edits `index.json` / `settings.json` by hand.
pub fn strip_bom(data: &[u8]) -> &[u8] {
    data.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(data)
}

/// `rename` that replaces the target and, on Windows, retries for a moment
/// when another process (virus scanner, indexer, OneDrive, an image viewer)
/// briefly holds the target open — the classic sharing violation that makes
/// a single `MoveFileEx` fail with "access denied" / "used by another process".
pub fn rename_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    retry_io(|| std::fs::rename(from, to))
}

/// Retries an I/O operation on transient Windows locking errors
/// (ERROR_ACCESS_DENIED 5, ERROR_SHARING_VIOLATION 32, ERROR_LOCK_VIOLATION 33),
/// ~1.5 s in total. Other errors (and every error on other systems) return
/// immediately.
pub fn retry_io<T>(mut op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let mut attempt = 0u64;
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) if attempt < 8 && cfg!(windows) && is_transient_lock(&e) => {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(25 * attempt * attempt / 2 + 20));
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_transient_lock(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33))
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
    rename_replace(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })?;
    Ok(())
}

pub fn load_library(path: &Path) -> Result<Library, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut lib: Library = serde_json::from_slice(strip_bom(&data)).map_err(|e| e.to_string())?;
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
            // local time, like the capture file names the user sees
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
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
    fn windows_rules_for_index_names() {
        for bad in [
            "CON", "con.png", "NUL.jpg", "Aux.png", "COM1.png", "lpt9.jpg", "a<b.png", "a>b.png",
            "a\"b.png", "a|b.png", "a?.png", "*.png", "x.png.", "tab\there.png",
        ] {
            assert!(!is_safe_file_name_for(bad, true), "{bad:?} must be rejected on Windows");
        }
        for ok in ["console.png", "COM10.png", "COM0.png", "Bild Ä ö ü ß.png", "a-b_c (1).jpg", "nullable.png"] {
            assert!(is_safe_file_name_for(ok, true), "{ok:?} is fine on Windows");
        }
        // macOS keeps accepting what its file system can hold
        assert!(is_safe_file_name_for("a?b.png", false));
        assert!(is_safe_file_name_for("CON.png", false));
    }

    #[test]
    fn sanitized_stems_are_portable() {
        assert_eq!(sanitize_stem("Bericht: Q1/Q2", "import"), "Bericht- Q1-Q2");
        assert_eq!(sanitize_stem("a*b?c\"d<e>f|g", "import"), "a-b-c-d-e-f-g");
        assert_eq!(sanitize_stem("...hidden", "import"), "hidden");
        assert_eq!(sanitize_stem("trailing. . ", "import"), "trailing");
        assert_eq!(sanitize_stem("CON", "import"), "_CON");
        assert_eq!(sanitize_stem("nul", "import"), "_nul");
        assert_eq!(sanitize_stem("   ", "import"), "import");
        assert_eq!(sanitize_stem("..", "import"), "import");
        assert_eq!(sanitize_stem("Grüße aus Köln", "import"), "Grüße aus Köln");
        assert_eq!(sanitize_stem(&"x".repeat(300), "import").chars().count(), 120);
        for s in ["Bericht: Q1/Q2", "CON", "...hidden", "a\tb"] {
            let out = format!("{}.png", sanitize_stem(s, "import"));
            assert!(is_safe_file_name_for(&out, true), "{out:?}");
        }
    }

    #[test]
    fn index_with_bom_loads() {
        let mut lib = Library::default();
        lib.add("a.png", ShotKind::Region, 1, 1, 1);
        let dir = std::env::temp_dir().join(format!("screencap-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.json");
        let mut data = b"\xEF\xBB\xBF".to_vec();
        data.extend(serde_json::to_vec(&lib).unwrap());
        std::fs::write(&path, data).unwrap();
        let loaded = load_library_or_recover(&path);
        assert_eq!(loaded.shots.len(), 1, "BOM must not count as a corrupt index");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn retry_io_gives_up_on_permanent_errors() {
        let mut calls = 0;
        let r: std::io::Result<()> = retry_io(|| {
            calls += 1;
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        });
        assert!(r.is_err());
        assert_eq!(calls, 1);
        let mut n = 0;
        let r = retry_io(|| {
            n += 1;
            if n < 3 && cfg!(windows) {
                Err(std::io::Error::from_raw_os_error(32))
            } else {
                Ok(n)
            }
        });
        assert!(r.is_ok());
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
