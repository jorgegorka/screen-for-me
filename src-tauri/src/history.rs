use std::fs;
use std::path::{Path, PathBuf};

pub const MIN_PLAUSIBLE_MP4_BYTES: u64 = 16 * 1024;
const MAX_CAPTURES: usize = 50;
const MAX_VIDEOS: usize = 10;
const IN_PROGRESS_PREFIX: &str = "recording-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    Image,
    Video,
}

impl CaptureKind {
    fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "png" => Some(Self::Image),
            "mp4" => Some(Self::Video),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CaptureEntry {
    pub path: PathBuf,
    pub id: String,
    pub created_ms: u64,
    pub kind: CaptureKind,
    pub poster: Option<PathBuf>,
}

pub struct History {
    dir: PathBuf,
}

impl History {
    pub fn new(dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let history = Self { dir };
        history.sweep_in_progress();
        Ok(history)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn new_capture_path(&self, ext: &str) -> PathBuf {
        self.dir.join(format!("capture-{}.{ext}", now_ms()))
    }

    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn new_recording_paths(&self) -> (PathBuf, PathBuf) {
        let now = now_ms();
        (
            self.dir.join(format!("{IN_PROGRESS_PREFIX}{now}.mp4")),
            self.dir.join(format!("capture-{now}.mp4")),
        )
    }

    fn sweep_in_progress(&self) {
        for entry in fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            let path = entry.path();
            let leftover = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(IN_PROGRESS_PREFIX) && n.ends_with(".mp4"));
            if leftover {
                let _ = fs::remove_file(&path);
            }
        }
    }

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
        if id.contains('/') || id.contains("..") {
            return None;
        }
        entry_from_path(self.dir.join(id))
    }

    pub fn prune(&self) {
        let entries = self.list();
        let mut videos = 0;
        for (index, entry) in entries.iter().enumerate() {
            let over_video_cap = entry.kind == CaptureKind::Video && {
                videos += 1;
                videos > MAX_VIDEOS
            };
            if index >= MAX_CAPTURES || over_video_cap {
                remove_entry(entry);
            }
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn poster_path(video: &Path) -> PathBuf {
    let mut name = video.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".png");
    video.with_file_name(name)
}

fn remove_entry(entry: &CaptureEntry) {
    let _ = fs::remove_file(&entry.path);
    if let Some(poster) = &entry.poster {
        let _ = fs::remove_file(poster);
    }
}

fn entry_from_path(path: PathBuf) -> Option<CaptureEntry> {
    let name = path.file_name()?.to_str()?.to_string();
    let (stem, ext) = name.strip_prefix("capture-")?.rsplit_once('.')?;
    let kind = CaptureKind::from_extension(ext)?;
    let created_ms = stem.parse().ok()?;
    let min_bytes = match kind {
        CaptureKind::Video => MIN_PLAUSIBLE_MP4_BYTES,
        CaptureKind::Image => 1,
    };
    if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) < min_bytes {
        return None;
    }
    let poster = match kind {
        CaptureKind::Video => Some(poster_path(&path)).filter(|p| p.is_file()),
        CaptureKind::Image => None,
    };
    Some(CaptureEntry {
        path,
        id: name,
        created_ms,
        kind,
        poster,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sfm-history-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn temp_history() -> History {
        History::new(temp_dir()).unwrap()
    }

    fn write_video(path: PathBuf) {
        fs::write(path, vec![0u8; MIN_PLAUSIBLE_MP4_BYTES as usize]).unwrap();
    }

    fn ids(h: &History) -> Vec<String> {
        h.list().into_iter().map(|e| e.id).collect()
    }

    #[test]
    fn list_is_newest_first_and_ignores_foreign_files() {
        let h = temp_history();
        fs::write(h.dir().join("capture-1000.png"), b"a").unwrap();
        fs::write(h.dir().join("capture-3000.png"), b"b").unwrap();
        fs::write(h.dir().join("capture-2000.png"), b"c").unwrap();
        fs::write(h.dir().join("notes.txt"), b"x").unwrap();
        fs::write(h.dir().join("capture-4000.mov"), b"x").unwrap();
        assert_eq!(
            ids(&h),
            ["capture-3000.png", "capture-2000.png", "capture-1000.png"]
        );
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn empty_files_are_skipped() {
        let h = temp_history();
        fs::write(h.dir().join("capture-1000.png"), b"a").unwrap();
        fs::write(h.dir().join("capture-2000.png"), b"").unwrap();
        assert_eq!(ids(&h), ["capture-1000.png"]);
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
    fn new_capture_path_uses_the_requested_extension() {
        let h = temp_history();
        assert!(h.new_capture_path("mp4").to_string_lossy().ends_with(".mp4"));
        assert!(h.new_capture_path("png").to_string_lossy().ends_with(".png"));
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn videos_are_listed_with_kind_and_poster() {
        let h = temp_history();
        fs::write(h.dir().join("capture-1000.png"), b"a").unwrap();
        write_video(h.dir().join("capture-2000.mp4"));
        fs::write(h.dir().join("capture-2000.mp4.png"), b"p").unwrap();
        write_video(h.dir().join("capture-3000.mp4"));
        let entries = h.list();
        assert_eq!(
            ids(&h),
            ["capture-3000.mp4", "capture-2000.mp4", "capture-1000.png"]
        );
        assert_eq!(entries[0].kind, CaptureKind::Video);
        assert_eq!(entries[0].poster, None);
        assert_eq!(entries[1].kind, CaptureKind::Video);
        assert_eq!(
            entries[1].poster.as_deref(),
            Some(h.dir().join("capture-2000.mp4.png").as_path())
        );
        assert_eq!(entries[2].kind, CaptureKind::Image);
        assert_eq!(entries[2].poster, None);
        assert!(h.resolve("capture-2000.mp4.png").is_none());
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
    fn prune_caps_videos_and_removes_their_posters() {
        let h = temp_history();
        for i in 0..(MAX_VIDEOS + 2) {
            write_video(h.dir().join(format!("capture-{}.mp4", 1000 + i)));
            fs::write(h.dir().join(format!("capture-{}.mp4.png", 1000 + i)), b"p").unwrap();
        }
        fs::write(h.dir().join("capture-100.png"), b"a").unwrap();
        h.prune();
        let entries = h.list();
        let videos = entries.iter().filter(|e| e.kind == CaptureKind::Video).count();
        assert_eq!(videos, MAX_VIDEOS);
        assert!(h.resolve("capture-100.png").is_some());
        assert!(!h.dir().join("capture-1000.mp4").exists());
        assert!(!h.dir().join("capture-1000.mp4.png").exists());
        assert!(!h.dir().join("capture-1001.mp4").exists());
        assert!(h.dir().join("capture-1002.mp4.png").exists());
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn in_progress_recordings_are_never_listed() {
        let h = temp_history();
        write_video(h.dir().join("recording-1000.mp4"));
        write_video(h.dir().join("capture-2000.mp4"));
        assert_eq!(ids(&h), ["capture-2000.mp4"]);
        assert!(h.resolve("recording-1000.mp4").is_none());
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn new_recording_paths_share_one_timestamp() {
        let h = temp_history();
        let (progress, final_path) = h.new_recording_paths();
        assert_eq!(progress.parent(), Some(h.dir()));
        assert_eq!(final_path.parent(), Some(h.dir()));
        let progress_ms = progress
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("recording-"))
            .and_then(|n| n.strip_suffix(".mp4"))
            .map(str::to_string);
        let final_ms = final_path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("capture-"))
            .and_then(|n| n.strip_suffix(".mp4"))
            .map(str::to_string);
        assert!(progress_ms.is_some());
        assert_eq!(progress_ms, final_ms);
        write_video(progress.clone());
        assert!(h.list().is_empty());
        fs::rename(&progress, &final_path).unwrap();
        assert_eq!(h.list().len(), 1);
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn leftover_in_progress_recordings_are_swept_on_open() {
        let dir = temp_dir();
        fs::create_dir_all(&dir).unwrap();
        write_video(dir.join("recording-1000.mp4"));
        write_video(dir.join("recording-2000.mp4"));
        write_video(dir.join("capture-3000.mp4"));
        fs::write(dir.join("notes.txt"), b"x").unwrap();
        let h = History::new(dir.clone()).unwrap();
        assert!(!dir.join("recording-1000.mp4").exists());
        assert!(!dir.join("recording-2000.mp4").exists());
        assert!(dir.join("capture-3000.mp4").exists());
        assert!(dir.join("notes.txt").exists());
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn undersized_videos_are_skipped() {
        let h = temp_history();
        fs::write(
            h.dir().join("capture-1000.mp4"),
            vec![0u8; MIN_PLAUSIBLE_MP4_BYTES as usize - 1],
        )
        .unwrap();
        write_video(h.dir().join("capture-2000.mp4"));
        assert_eq!(ids(&h), ["capture-2000.mp4"]);
        assert!(h.resolve("capture-1000.mp4").is_none());
        fs::remove_dir_all(h.dir()).unwrap();
    }

    #[test]
    fn poster_path_appends_png() {
        assert_eq!(
            poster_path(Path::new("/x/capture-5.mp4")),
            PathBuf::from("/x/capture-5.mp4.png")
        );
    }
}
