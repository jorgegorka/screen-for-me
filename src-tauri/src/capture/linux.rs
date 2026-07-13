use std::path::Path;

use ashpd::desktop::screenshot::Screenshot;
use ashpd::WindowIdentifier;

use super::{validate_output, CaptureError, CaptureMode, CaptureOutcome};

/// Capture via the xdg-desktop-portal Screenshot API (X11 and Wayland).
/// The portal owns the selection UI, so area and window modes both go through
/// its interactive flow; fullscreen skips it.
pub fn capture(mode: CaptureMode, dest: &Path) -> Result<CaptureOutcome, CaptureError> {
    let interactive = !matches!(mode, CaptureMode::Fullscreen);
    let response = tauri::async_runtime::block_on(async move {
        Screenshot::request()
            .identifier(WindowIdentifier::default())
            .interactive(interactive)
            .modal(true)
            .send()
            .await?
            .response()
    });
    let screenshot = match response {
        Ok(screenshot) => screenshot,
        Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {
            return Ok(CaptureOutcome::Cancelled);
        }
        Err(err) => return Err(CaptureError::Tool(err.to_string())),
    };
    let source = screenshot.uri().to_file_path().map_err(|_| {
        CaptureError::Tool(format!(
            "portal returned non-file URI: {}",
            screenshot.uri()
        ))
    })?;
    std::fs::copy(&source, dest)?;
    let _ = std::fs::remove_file(&source);
    validate_output(dest)
}
