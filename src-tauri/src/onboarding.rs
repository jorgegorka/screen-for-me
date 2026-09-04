use tauri::{AppHandle, State};

use crate::commands::{broadcast_settings, AppState};
use crate::settings::Settings;
use crate::shortcuts::{self, ShortcutAction};

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

#[tauri::command]
pub fn macos_screenshot_hotkeys_owned() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("defaults")
            .args(["export", "com.apple.symbolichotkeys", "-"])
            .output()
        {
            Ok(out) if out.status.success() => {
                owned_screenshot_keys(&String::from_utf8_lossy(&out.stdout))
            }
            _ => Vec::new(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    Vec::new()
}

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
    shortcuts::rebind_all(&app, &changes, "welcome.assign_failed")?;
    for (action, _, new) in &changes {
        *settings.shortcut_mut(*action) = new.clone();
    }
    let saved = state.settings.set(settings).map_err(|e| e.to_string())?;
    broadcast_settings(&app, &saved, true);
    Ok(saved)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const SCREENSHOT_KEY_IDS: [(&str, u32); 3] = [("3", 28), ("4", 30), ("5", 184)];

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn owned_screenshot_keys(xml: &str) -> Vec<String> {
    SCREENSHOT_KEY_IDS
        .iter()
        .filter(|(_, id)| entry_enabled(xml, *id))
        .map(|(key, _)| (*key).to_string())
        .collect()
}

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
    fn all_disabled_reports_nothing_owned() {
        let xml = plist(&[entry(28, false), entry(30, false), entry(184, false)]);
        assert!(owned_screenshot_keys(&xml).is_empty());
    }

    #[test]
    fn only_the_enabled_entry_key_is_owned() {
        let xml = plist(&[entry(28, false), entry(30, true), entry(184, false)]);
        assert_eq!(owned_screenshot_keys(&xml), vec!["4"]);
    }

    #[test]
    fn only_the_freed_key_is_not_owned() {
        let xml = plist(&[entry(28, false), entry(30, true), entry(184, true)]);
        assert_eq!(owned_screenshot_keys(&xml), vec!["4", "5"]);
    }

    #[test]
    fn missing_entries_count_as_owned() {
        let xml = plist(&[entry(28, false)]);
        assert_eq!(owned_screenshot_keys(&xml), vec!["4", "5"]);
    }

    #[test]
    fn clipboard_variants_are_ignored() {
        let xml = plist(&[
            entry(28, false),
            entry(29, true),
            entry(30, false),
            entry(31, true),
            entry(184, false),
        ]);
        assert!(owned_screenshot_keys(&xml).is_empty());
    }

    #[test]
    fn garbage_without_entries_counts_as_all_owned() {
        assert_eq!(
            owned_screenshot_keys("<plist><dict></dict></plist>"),
            vec!["3", "4", "5"]
        );
    }
}
