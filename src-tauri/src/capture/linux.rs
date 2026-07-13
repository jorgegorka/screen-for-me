use std::path::Path;

use super::{CaptureError, CaptureMode, CaptureOutcome};

/// Linux capture lands in Phase 4 via the xdg-desktop-portal Screenshot API
/// (`ashpd`), which works on both X11 and Wayland.
pub fn capture(_mode: CaptureMode, _dest: &Path) -> Result<CaptureOutcome, CaptureError> {
    Err(CaptureError::Tool(
        "Linux capture is not implemented yet (coming via xdg-desktop-portal)".into(),
    ))
}
