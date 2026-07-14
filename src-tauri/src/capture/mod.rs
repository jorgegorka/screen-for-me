use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub mod stitch;
#[cfg(target_os = "macos")]
pub mod scroll_input;
#[cfg(target_os = "macos")]
pub mod scrolling;

/// Scroll direction for scrolling capture. Lives here (not in the macOS-only
/// `stitch` module) so the IPC command can deserialize it on every platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    Area,
    Window,
    Fullscreen,
}

#[derive(Debug)]
pub enum CaptureOutcome {
    /// A capture was written to the given path.
    Captured(PathBuf),
    /// The user dismissed the selection UI (Escape); not an error.
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("failed to run capture tool: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("capture tool failed: {0}")]
    Tool(String),
    #[error(
        "capture produced no usable image — on macOS this usually means the app lacks \
         Screen Recording permission (System Settings → Privacy & Security → Screen Recording)"
    )]
    EmptyCapture,
}

/// Capture the screen with the OS-native tool, writing a PNG to `dest`.
pub fn capture(mode: CaptureMode, dest: &Path) -> Result<CaptureOutcome, CaptureError> {
    #[cfg(target_os = "macos")]
    return macos::capture(mode, dest);
    #[cfg(target_os = "linux")]
    return linux::capture(mode, dest);
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (mode, dest);
        Err(CaptureError::Tool("unsupported platform".into()))
    }
}

/// Run `/usr/sbin/screencapture` with `args` plus the destination path,
/// shaping a non-zero exit into `CaptureError::Tool` (stderr when present).
/// Output validation stays with the callers — interactive and silent modes
/// disagree on what a missing file means.
#[cfg(target_os = "macos")]
pub(crate) fn run_screencapture(args: &[&str], dest: &Path) -> Result<(), CaptureError> {
    let output = std::process::Command::new("/usr/sbin/screencapture")
        .args(args)
        .arg(dest)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CaptureError::Tool(if stderr.is_empty() {
            format!("screencapture exited with {}", output.status)
        } else {
            stderr
        }));
    }
    Ok(())
}

/// A cancelled interactive selection leaves no file; a permission-starved
/// capture can leave a tiny/blank one. Anything below this is not a screenshot.
const MIN_PLAUSIBLE_PNG_BYTES: u64 = 1024;

pub(crate) fn validate_output(dest: &Path) -> Result<CaptureOutcome, CaptureError> {
    match std::fs::metadata(dest) {
        Err(_) => Ok(CaptureOutcome::Cancelled),
        Ok(meta) if meta.len() < MIN_PLAUSIBLE_PNG_BYTES => {
            let _ = std::fs::remove_file(dest);
            Err(CaptureError::EmptyCapture)
        }
        Ok(_) => Ok(CaptureOutcome::Captured(dest.to_path_buf())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_cancelled() {
        let dir = std::env::temp_dir().join("sfm-test-none.png");
        let _ = std::fs::remove_file(&dir);
        assert!(matches!(
            validate_output(&dir).unwrap(),
            CaptureOutcome::Cancelled
        ));
    }

    #[test]
    fn tiny_file_is_empty_capture_error() {
        let path = std::env::temp_dir().join("sfm-test-tiny.png");
        std::fs::write(&path, b"png?").unwrap();
        assert!(matches!(
            validate_output(&path),
            Err(CaptureError::EmptyCapture)
        ));
        assert!(!path.exists(), "invalid capture file should be cleaned up");
    }

    #[test]
    fn plausible_file_is_captured() {
        let path = std::env::temp_dir().join("sfm-test-ok.png");
        std::fs::write(&path, vec![0u8; 4096]).unwrap();
        assert!(matches!(
            validate_output(&path).unwrap(),
            CaptureOutcome::Captured(_)
        ));
        let _ = std::fs::remove_file(&path);
    }
}
