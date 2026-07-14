use std::path::Path;

use super::{run_screencapture, validate_output, CaptureError, CaptureMode, CaptureOutcome};

/// Capture via the system `screencapture` tool. Interactive modes present the
/// native crosshair / window-picker; Escape cancels and writes no file.
pub fn capture(mode: CaptureMode, dest: &Path) -> Result<CaptureOutcome, CaptureError> {
    let args: &[&str] = match mode {
        // -i: interactive selection (drag an area; Space toggles window mode)
        CaptureMode::Area => &["-i", "-t", "png"],
        // -w + -o: window picker without the drop shadow border
        CaptureMode::Window => &["-i", "-W", "-o", "-t", "png"],
        CaptureMode::Fullscreen => &["-t", "png"],
    };
    // screencapture exits non-zero on real failures; a cancelled -i exits 0.
    run_screencapture(args, dest)?;
    validate_output(dest)
}
