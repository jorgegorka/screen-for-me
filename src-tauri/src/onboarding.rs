//! Shortcut onboarding: the Welcome window's guided flow for taking over the
//! macOS screenshot shortcuts (⇧⌘3/4/5), plus the Settings-tab helpers.

use tauri::{AppHandle, Emitter, State};

use crate::commands::AppState;
use crate::settings::Settings;
use crate::shortcuts::{self, ShortcutAction};

/// Open macOS System Settings on the Keyboard pane so the user can disable
/// the system screenshot shortcuts (Keyboard Shortcuts… → Screenshots — there
/// is no public deep link to that subpane, so UI copy carries the navigation).
#[tauri::command]
pub fn open_system_shortcut_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(
                "x-apple.systempreferences:com.apple.Keyboard-Settings.extension",
                None::<&str>,
            )
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(())
    }
}

/// Best-effort: is macOS itself still handling any of ⇧⌘3/4/5? Registering
/// those combos succeeds even while the system owns them (the keypress just
/// never reaches the app), so this is the only honest signal for the UI.
/// Reads the user's symbolic-hotkeys defaults; any failure reports `false`
/// (never nag on a parse problem). Always `false` off macOS.
#[tauri::command]
pub fn macos_screenshot_hotkeys_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("defaults")
            .args(["export", "com.apple.symbolichotkeys", "-"])
            .output()
        {
            Ok(out) if out.status.success() => {
                parse_symbolic_hotkeys(&String::from_utf8_lossy(&out.stdout))
            }
            _ => false,
        }
    }
    #[cfg(not(target_os = "macos"))]
    false
}

/// Assign the classic macOS combos in one go, mapped to system muscle memory:
/// ⇧⌘3 → fullscreen, ⇧⌘4 → area, ⇧⌘5 → window. Atomic (rebind_all rolls back
/// on failure); persists once, then refreshes the tray and broadcasts
/// `settings:changed` like `set_shortcut`.
#[tauri::command]
pub fn apply_macos_screenshot_shortcuts(
    app: AppHandle,
    state: State<AppState>,
) -> Result<Settings, String> {
    const TARGETS: [(ShortcutAction, &str); 3] = [
        (ShortcutAction::Fullscreen, "Cmd+Shift+3"),
        (ShortcutAction::Area, "Cmd+Shift+4"),
        (ShortcutAction::Window, "Cmd+Shift+5"),
    ];
    let mut settings = state.settings.get();
    let changes: Vec<(ShortcutAction, String, String)> = TARGETS
        .iter()
        .map(|(action, new)| (*action, settings.shortcut(*action).to_string(), (*new).to_string()))
        .collect();
    shortcuts::rebind_all(&app, &changes)?;
    for (action, _, new) in &changes {
        *settings.shortcut_mut(*action) = new.clone();
    }
    let saved = state.settings.set(settings).map_err(|e| e.to_string())?;
    // Menu APIs must run on the main thread on macOS.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(err) = crate::tray::refresh(&handle) {
            eprintln!("failed to rebuild tray menu: {err}");
        }
    });
    let _ = app.emit("settings:changed", &saved);
    Ok(saved)
}

/// AppleSymbolicHotKeys ids for the bare ⇧⌘3/4/5 actions: 28 = save screen,
/// 30 = save selection, 184 = screenshot options. The clipboard variants
/// (29/31) use ⌃⇧⌘ and don't collide with the bare combos.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const SCREENSHOT_HOTKEY_IDS: [u32; 3] = [28, 30, 184];

/// True when any bare screenshot hotkey is still enabled in the exported
/// `com.apple.symbolichotkeys` plist. A missing entry means the system
/// default applies, i.e. enabled. The plist format is undocumented, so this
/// stays a minimal text scan rather than a full parser.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_symbolic_hotkeys(xml: &str) -> bool {
    SCREENSHOT_HOTKEY_IDS.iter().any(|id| entry_enabled(xml, *id))
}

/// Find `<key>{id}</key>`, take its following balanced `<dict>` block, and
/// read the `enabled` flag inside it. Missing pieces count as enabled.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn entry_enabled(xml: &str, id: u32) -> bool {
    let marker = format!("<key>{id}</key>");
    let Some(pos) = xml.find(&marker) else {
        return true;
    };
    let rest = &xml[pos + marker.len()..];
    let Some(dict_start) = rest.find("<dict>") else {
        return true;
    };
    let mut depth = 0usize;
    let mut i = dict_start;
    let mut end = rest.len();
    while i < rest.len() {
        if rest[i..].starts_with("<dict>") {
            depth += 1;
            i += 6;
        } else if rest[i..].starts_with("</dict>") {
            depth -= 1;
            i += 7;
            if depth == 0 {
                end = i;
                break;
            }
        } else {
            i += 1;
        }
    }
    let body = &rest[dict_start..end];
    let Some(enabled_pos) = body.find("<key>enabled</key>") else {
        return true;
    };
    let after = &body[enabled_pos + "<key>enabled</key>".len()..];
    match (after.find("<true/>"), after.find("<false/>")) {
        (Some(t), Some(f)) => t < f,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32, enabled: bool) -> String {
        format!(
            r#"<key>{id}</key>
            <dict>
                <key>enabled</key>{}
                <key>value</key>
                <dict>
                    <key>parameters</key>
                    <array><integer>65535</integer><integer>51</integer><integer>1179648</integer></array>
                    <key>type</key>
                    <string>standard</string>
                </dict>
            </dict>"#,
            if enabled { "<true/>" } else { "<false/>" }
        )
    }

    fn plist(entries: &[String]) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>AppleSymbolicHotKeys</key>
    <dict>
    {}
    </dict>
</dict>
</plist>"#,
            entries.join("\n")
        )
    }

    #[test]
    fn all_disabled_reports_not_enabled() {
        let xml = plist(&[entry(28, false), entry(30, false), entry(184, false)]);
        assert!(!parse_symbolic_hotkeys(&xml));
    }

    #[test]
    fn any_enabled_entry_reports_enabled() {
        let xml = plist(&[entry(28, false), entry(30, true), entry(184, false)]);
        assert!(parse_symbolic_hotkeys(&xml));
    }

    #[test]
    fn missing_entries_count_as_enabled() {
        // A fresh system may have no explicit entry for a hotkey: the default
        // (enabled) applies.
        let xml = plist(&[entry(28, false)]);
        assert!(parse_symbolic_hotkeys(&xml));
    }

    #[test]
    fn clipboard_variants_are_ignored() {
        // 29/31 are the ⌃⇧⌘ clipboard combos; only 28/30/184 matter.
        let xml = plist(&[
            entry(28, false),
            entry(29, true),
            entry(30, false),
            entry(31, true),
            entry(184, false),
        ]);
        assert!(!parse_symbolic_hotkeys(&xml));
    }

    #[test]
    fn garbage_without_entries_counts_as_enabled() {
        // No screenshot entries at all → system defaults apply.
        assert!(parse_symbolic_hotkeys("<plist><dict></dict></plist>"));
    }
}
