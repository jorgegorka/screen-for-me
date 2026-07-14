use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::capture::CaptureMode;
use crate::commands::trigger_capture;

// macOS reserves Cmd+Shift+3/4/5 for the system tools, so we use 7/8/9.
// Accelerator strings are shown in the tray menu; keep both in sync.
pub const ACCEL_AREA: &str = "CmdOrCtrl+Shift+7";
pub const ACCEL_WINDOW: &str = "CmdOrCtrl+Shift+8";
pub const ACCEL_FULLSCREEN: &str = "CmdOrCtrl+Shift+9";

#[cfg(target_os = "macos")]
const PRIMARY: Modifiers = Modifiers::SUPER;
#[cfg(not(target_os = "macos"))]
const PRIMARY: Modifiers = Modifiers::CONTROL;

pub fn setup(app: &AppHandle) -> Result<(), tauri_plugin_global_shortcut::Error> {
    let bindings = [
        (Code::Digit7, CaptureMode::Area),
        (Code::Digit8, CaptureMode::Window),
        (Code::Digit9, CaptureMode::Fullscreen),
    ];
    for (code, mode) in bindings {
        let shortcut = Shortcut::new(Some(PRIMARY | Modifiers::SHIFT), code);
        app.global_shortcut()
            .on_shortcut(shortcut, move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    trigger_capture(app, mode);
                }
            })?;
    }
    Ok(())
}
