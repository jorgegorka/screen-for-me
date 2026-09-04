use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::capture::CaptureMode;
use crate::commands::{trigger_capture, AppState};

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

pub fn setup(app: &AppHandle) {
    let settings = app.state::<AppState>().settings.get();
    for action in ACTIONS {
        let accel = settings.shortcut(action).to_string();
        if let Err(err) = register(app, action, &accel) {
            eprintln!("failed to register {accel} for {action:?}: {err}");
        }
    }
}

pub fn rebind(
    app: &AppHandle,
    action: ShortcutAction,
    old_accel: &str,
    new_accel: &str,
) -> Result<(), String> {
    let new_shortcut = validate(new_accel)?;
    if old_accel.parse::<Shortcut>().ok() == Some(new_shortcut) {
        return Ok(());
    }
    rebind_all(
        app,
        &[(action, old_accel.to_string(), new_accel.to_string())],
        "settings.shortcut_error_taken",
    )
}

fn unregister(app: &AppHandle, accel: &str) {
    if let Ok(shortcut) = accel.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(shortcut);
    }
}

pub fn rebind_all(
    app: &AppHandle,
    changes: &[(ShortcutAction, String, String)],
    error_key: &str,
) -> Result<(), String> {
    for (_, old, _) in changes {
        unregister(app, old);
    }
    for (i, (action, _, new)) in changes.iter().enumerate() {
        if let Err(err) = register(app, *action, new) {
            eprintln!("failed to register {new} for {action:?}: {err}");
            for (_, _, done_new) in &changes[..i] {
                unregister(app, done_new);
            }
            for (action, old, _) in changes {
                if let Err(err) = register(app, *action, old) {
                    eprintln!("failed to restore {old} for {action:?}: {err}");
                }
            }
            return Err(crate::i18n::t(error_key));
        }
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

    #[test]
    fn macos_screenshot_combos_validate() {
        for accel in ["Cmd+Shift+3", "Cmd+Shift+4", "Cmd+Shift+5"] {
            validate(accel).expect("screenshot combos are assignable");
        }
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
