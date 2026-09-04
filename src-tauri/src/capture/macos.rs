use std::path::Path;

use super::display::display_under_cursor;
use super::{run_screencapture, validate_output, CaptureError, CaptureMode, CaptureOutcome};

pub fn capture(mode: CaptureMode, dest: &Path) -> Result<CaptureOutcome, CaptureError> {
    let mut args: Vec<&str> = match mode {
        CaptureMode::Area => vec!["-i", "-t", "png"],
        CaptureMode::Window => vec!["-i", "-W", "-o", "-t", "png"],
        CaptureMode::Fullscreen => vec!["-t", "png"],
    };
    let rect = (mode == CaptureMode::Fullscreen)
        .then(display_under_cursor)
        .flatten()
        .map(|display| display.screencapture_rect());
    if let Some(rect) = &rect {
        args.extend(["-R", rect]);
    }
    run_screencapture(&args, dest)?;
    validate_output(dest)
}
