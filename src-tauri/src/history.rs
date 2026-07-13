use std::fs;
use std::path::{Path, PathBuf};

/// Keep the newest N captures on disk; older ones are pruned after each capture.
const MAX_CAPTURES: usize = 50;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CaptureEntry {
    /// Absolute path to the PNG on disk.
    pub path: PathBuf,
    /// File name (stable id for IPC).
    pub id: String,
    /// Unix milliseconds, derived from the file name.
    pub created_ms: u64,
}

pub struct History {
    dir: PathBuf,
}

impl History {
    pub fn new(dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Reserve a destination path for a new capture.
    pub fn new_capture_path(&self) -> PathBuf {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.dir.join(format!("capture-{now}.png"))
    }

    /// All captures, newest first.
    pub fn list(&self) -> Vec<CaptureEntry> {
        let mut entries: Vec<CaptureEntry> = fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| entry_from_path(e.path()))
            .collect();
        entries.sort_by(|a, b| b.created_ms.cmp(&a.created_ms));
        entries
    }

    pub fn resolve(&self, id: &str) -> Option<CaptureEntry> {
        // ids are bare file names we generated; reject anything path-like
        if id.contains('/') || id.contains("..") {
            return None;
        }
        let path = self.dir.join(id);
        path.exists().then(|| entry_from_path(path)).flatten()
    }

    pub fn delete(&self, id: &str) -> bool {
        match self.resolve(id) {
            Some(entry) => fs::remove_file(entry.path).is_ok(),
            None => false,
        }
    }

    pub fn prune(&self) {
        let entries = self.list();
        for old in entries.iter().skip(MAX_CAPTURES) {
            let _ = fs::remove_file(&old.path);
        }
    }
}

fn entry_from_path(path: PathBuf) -> Option<CaptureEntry> {
    let name = path.file_name()?.to_str()?.to_string();
    let created_ms = name
        .strip_prefix("capture-")?
        .strip_suffix(".png")?
        .parse()
        .ok()?;
    // Skip empty files: a crashed or cancelled capture can leave a 0-byte
    // stub, which must never surface as a capture (broken thumbnail / blank
    // editor).
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) == 0 {
        return None;
    }
    Some(CaptureEntry {
        path,
        id: name,
        created_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_history() -> History {
        let dir = std::env::temp_dir().join(format!(
            "sfm-history-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        History::new(dir).unwrap()
    }

    #[test]
    fn list_is_newest_first_and_ignores_foreign_files() {
        let h = temp_history();
        fs::write(h.dir().join("capture-1000.png"), b"a").unwrap();
        fs::write(h.dir().join("capture-3000.png"), b"b").unwrap();
        fs::write(h.dir().join("capture-2000.png"), b"c").unwrap();
        fs::write(h.dir().join("notes.txt"), b"x").unwrap();
        let ids: Vec<_> = h.list().into_iter().map(|e| e.id).collect();
        assert_eq!(
            ids,
            ["capture-3000.png", "capture-2000.png", "capture-1000.png"]
        );
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn empty_files_are_skipped() {
        let h = temp_history();
        fs::write(h.dir().join("capture-1000.png"), b"a").unwrap();
        fs::write(h.dir().join("capture-2000.png"), b"").unwrap();
        let ids: Vec<_> = h.list().into_iter().map(|e| e.id).collect();
        assert_eq!(ids, ["capture-1000.png"]);
        assert!(h.resolve("capture-2000.png").is_none());
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn resolve_rejects_traversal() {
        let h = temp_history();
        assert!(h.resolve("../etc/passwd").is_none());
        assert!(h.resolve("capture-1.png/x").is_none());
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn prune_keeps_newest() {
        let h = temp_history();
        for i in 0..(MAX_CAPTURES + 5) {
            fs::write(h.dir().join(format!("capture-{}.png", 1000 + i)), b"a").unwrap();
        }
        h.prune();
        let entries = h.list();
        assert_eq!(entries.len(), MAX_CAPTURES);
        assert_eq!(
            entries[0].id,
            format!("capture-{}.png", 1000 + MAX_CAPTURES + 4)
        );
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn delete_removes_file() {
        let h = temp_history();
        fs::write(h.dir().join("capture-1000.png"), b"a").unwrap();
        assert!(h.delete("capture-1000.png"));
        assert!(h.list().is_empty());
        assert!(!h.delete("capture-1000.png"));
        fs::remove_dir_all(h.dir()).unwrap();
    }
}
