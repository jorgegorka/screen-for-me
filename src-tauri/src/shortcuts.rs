use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::capture::CaptureMode;
use crate::commands::{trigger_capture, AppState};

/// Capture actions bound to user-configurable global shortcuts. The serde
/// names double as the frontend's action ids — never localise them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    Area,
    Window,
    Fullscreen,
}

pub const ACTIONS: [ShortcutAction; 3] = [
    ShortcutAction::Area,
    ShortcutAction::Window,
    ShortcutAction::Fullscreen,
];

impl ShortcutAction {
    /// macOS reserves Cmd+Shift+3/4/5 for the system tools, so defaults use
    /// 7/8/9. Mirrored in DEFAULT_ACCELS in src/shared/accelerator.ts.
    pub fn default_accel(self) -> &'static str {
        match self {
            ShortcutAction::Area => "CmdOrCtrl+Shift+7",
            ShortcutAction::Window => "CmdOrCtrl+Shift+8",
            ShortcutAction::Fullscreen => "CmdOrCtrl+Shift+9",
        }
    }

    pub fn mode(self) -> CaptureMode {
        match self {
            ShortcutAction::Area => CaptureMode::Area,
            ShortcutAction::Window => CaptureMode::Window,
            ShortcutAction::Fullscreen => CaptureMode::Fullscreen,
        }
    }
}

/// Parse and police an accelerator string; the returned shortcut lets callers
/// compare combos across actions (equality is on resolved modifiers + key, so
/// "Cmd+Shift+7" and "CmdOrCtrl+Shift+7" collide on macOS). The same string
/// grammar is accepted by the global-shortcut registration and the tray's
/// menu-item accelerator labels.
pub fn validate(accel: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = accel
        .parse()
        .map_err(|_| crate::i18n::t("settings.shortcut_error_invalid"))?;
    if !shortcut
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        return Err(crate::i18n::t("settings.shortcut_error_modifier"));
    }
    #[cfg(target_os = "macos")]
    if shortcut.mods == Modifiers::SUPER | Modifiers::SHIFT
        && [Code::Digit3, Code::Digit4, Code::Digit5].contains(&shortcut.key)
    {
        return Err(crate::i18n::t("settings.shortcut_error_reserved"));
    }
    Ok(shortcut)
}

fn register(app: &AppHandle, action: ShortcutAction, accel: &str) -> Result<(), String> {
    let shortcut = validate(accel)?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                trigger_capture(app, action.mode());
            }
        })
        .map_err(|e| e.to_string())
}

/// Register the persisted shortcuts. A single failed registration (e.g. a
/// combo held by another app) is logged and skipped so launch never aborts.
pub fn setup(app: &AppHandle) {
    let settings = app.state::<AppState>().settings.get();
    for action in ACTIONS {
        let accel = settings.shortcut(action).to_string();
        if let Err(err) = register(app, action, &accel) {
            eprintln!("failed to register {accel} for {action:?}: {err}");
        }
    }
}

/// Swap an action's registration to a new combo. On failure the old combo is
/// restored and the (localised) error is returned for the Settings UI.
pub fn rebind(
    app: &AppHandle,
    action: ShortcutAction,
    old_accel: &str,
    new_accel: &str,
) -> Result<(), String> {
    let new_shortcut = validate(new_accel)?;
    if let Ok(old_shortcut) = old_accel.parse::<Shortcut>() {
        if old_shortcut == new_shortcut {
            return Ok(());
        }
        let _ = app.global_shortcut().unregister(old_shortcut);
    }
    if let Err(err) = register(app, action, new_accel) {
        eprintln!("failed to register {new_accel} for {action:?}: {err}");
        if let Err(err) = register(app, action, old_accel) {
            eprintln!("failed to restore {old_accel} for {action:?}: {err}");
        }
        return Err(crate::i18n::t("settings.shortcut_error_taken"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        for action in ACTIONS {
            validate(action.default_accel()).expect("default accelerator must validate");
        }
    }

    #[test]
    fn good_combos_are_accepted() {
        for accel in ["CmdOrCtrl+Shift+7", "Alt+Shift+A", "Ctrl+F5", "Cmd+Comma"] {
            assert!(validate(accel).is_ok(), "{accel} should validate");
        }
    }

    #[test]
    fn unparseable_strings_are_rejected() {
        for accel in ["", "NotAKey", "Cmd+Shift", "Cmd+Shift+7+8"] {
            assert!(validate(accel).is_err(), "{accel} should be rejected");
        }
    }

    #[test]
    fn shift_only_and_bare_keys_are_rejected() {
        for accel in ["7", "Shift+7", "Shift+A"] {
            assert!(validate(accel).is_err(), "{accel} needs a non-Shift modifier");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_screenshot_combos_are_reserved() {
        for accel in ["Cmd+Shift+3", "CmdOrCtrl+Shift+4", "Cmd+Shift+5"] {
            assert!(validate(accel).is_err(), "{accel} is reserved by macOS");
        }
        // Adding another modifier makes them fair game again.
        assert!(validate("Cmd+Alt+Shift+3").is_ok());
    }

    #[test]
    fn equivalent_spellings_compare_equal() {
        let canonical = validate("CmdOrCtrl+Shift+7").unwrap();
        #[cfg(target_os = "macos")]
        let spelled = validate("Cmd+Shift+Digit7").unwrap();
        #[cfg(not(target_os = "macos"))]
        let spelled = validate("Ctrl+Shift+Digit7").unwrap();
        assert_eq!(canonical, spelled);
    }
}
