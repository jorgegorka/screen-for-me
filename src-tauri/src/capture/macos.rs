use std::path::Path;
use std::process::Command;

use super::{validate_output, CaptureError, CaptureMode, CaptureOutcome};

/// Capture via the system `screencapture` tool. Interactive modes present the
/// native crosshair / window-picker; Escape cancels and writes no file.
pub fn capture(mode: CaptureMode, dest: &Path) -> Result<CaptureOutcome, CaptureError> {
    let mut cmd = Command::new("/usr/sbin/screencapture");
    match mode {
        // -i: interactive selection (drag an area; Space toggles window mode)
        CaptureMode::Area => cmd.arg("-i"),
        // -w + -o: window picker without the drop shadow border
        CaptureMode::Window => cmd.args(["-i", "-W", "-o"]),
        CaptureMode::Fullscreen => &mut cmd,
    };
    let output = cmd.args(["-t", "png"]).arg(dest).output()?;
    if !output.status.success() {
        // screencapture exits non-zero on real failures; a cancelled -i exits 0.
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CaptureError::Tool(if stderr.is_empty() {
            format!("screencapture exited with {}", output.status)
        } else {
            stderr
        }));
    }
    validate_output(dest)
}
