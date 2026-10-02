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
        reconcile_screenshot_hotkeys()
    }
    #[cfg(not(target_os = "macos"))]
    Vec::new()
}

#[cfg(target_os = "macos")]
fn reconcile_screenshot_hotkeys() -> Vec<String> {
    match std::process::Command::new("defaults")
        .args(["export", "com.apple.symbolichotkeys", "-"])
        .output()
    {
        Ok(out) if out.status.success() => {
            reconcile_owned_keys(&String::from_utf8_lossy(&out.stdout), &WindowServerHotKeys)
        }
        _ => Vec::new(),
    }
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
    #[cfg(target_os = "macos")]
    reconcile_screenshot_hotkeys();
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
trait LiveHotKeys {
    fn enabled(&self, id: u32) -> Option<bool>;
    fn disable(&self, id: u32);
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn reconcile_owned_keys(xml: &str, live: &impl LiveHotKeys) -> Vec<String> {
    SCREENSHOT_KEY_IDS
        .iter()
        .filter(|(_, id)| {
            if entry_enabled(xml, *id) {
                return true;
            }
            if live.enabled(*id) == Some(true) {
                live.disable(*id);
            }
            live.enabled(*id) == Some(true)
        })
        .map(|(key, _)| (*key).to_string())
        .collect()
}

#[cfg(target_os = "macos")]
struct WindowServerHotKeys;

#[cfg(target_os = "macos")]
impl LiveHotKeys for WindowServerHotKeys {
    fn enabled(&self, id: u32) -> Option<bool> {
        let symbol = skylight_symbol(c"CGSIsSymbolicHotKeyEnabled")?;
        let is_enabled: unsafe extern "C" fn(u32) -> u8 = unsafe { std::mem::transmute(symbol) };
        Some(unsafe { is_enabled(id) } != 0)
    }

    fn disable(&self, id: u32) {
        if let Some(symbol) = skylight_symbol(c"CGSSetSymbolicHotKeyEnabled") {
            let set_enabled: unsafe extern "C" fn(u32, bool) -> i32 =
                unsafe { std::mem::transmute(symbol) };
            unsafe { set_enabled(id, false) };
        }
    }
}

#[cfg(target_os = "macos")]
fn skylight_symbol(name: &std::ffi::CStr) -> Option<*mut std::ffi::c_void> {
    use std::ffi::{c_char, c_void};
    extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
    let symbol = unsafe { dlsym(RTLD_DEFAULT, name.as_ptr()) };
    (!symbol.is_null()).then_some(symbol)
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
    use std::cell::RefCell;

    struct Unavailable;

    impl LiveHotKeys for Unavailable {
        fn enabled(&self, _id: u32) -> Option<bool> {
            None
        }
        fn disable(&self, _id: u32) {}
    }

    struct FakeLive {
        enabled: RefCell<Vec<u32>>,
        honours_disable: bool,
        disabled: RefCell<Vec<u32>>,
    }

    impl FakeLive {
        fn new(enabled: &[u32], honours_disable: bool) -> Self {
            Self {
                enabled: RefCell::new(enabled.to_vec()),
                honours_disable,
                disabled: RefCell::new(Vec::new()),
            }
        }
    }

    impl LiveHotKeys for FakeLive {
        fn enabled(&self, id: u32) -> Option<bool> {
            Some(self.enabled.borrow().contains(&id))
        }
        fn disable(&self, id: u32) {
            self.disabled.borrow_mut().push(id);
            if self.honours_disable {
                self.enabled.borrow_mut().retain(|e| *e != id);
            }
        }
    }

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
        assert!(reconcile_owned_keys(&xml, &Unavailable).is_empty());
    }

    #[test]
    fn only_the_enabled_entry_key_is_owned() {
        let xml = plist(&[entry(28, false), entry(30, true), entry(184, false)]);
        assert_eq!(reconcile_owned_keys(&xml, &Unavailable), vec!["4"]);
    }

    #[test]
    fn only_the_freed_key_is_not_owned() {
        let xml = plist(&[entry(28, false), entry(30, true), entry(184, true)]);
        assert_eq!(reconcile_owned_keys(&xml, &Unavailable), vec!["4", "5"]);
    }

    #[test]
    fn missing_entries_count_as_owned() {
        let xml = plist(&[entry(28, false)]);
        assert_eq!(reconcile_owned_keys(&xml, &Unavailable), vec!["4", "5"]);
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
        assert!(reconcile_owned_keys(&xml, &Unavailable).is_empty());
    }

    #[test]
    fn garbage_without_entries_counts_as_all_owned() {
        assert_eq!(
            reconcile_owned_keys("<plist><dict></dict></plist>", &Unavailable),
            vec!["3", "4", "5"]
        );
    }

    #[test]
    fn stale_live_hotkey_is_released_when_prefs_disable_it() {
        let xml = plist(&[entry(28, false), entry(30, false), entry(184, false)]);
        let live = FakeLive::new(&[30], true);
        assert!(reconcile_owned_keys(&xml, &live).is_empty());
        assert_eq!(*live.disabled.borrow(), vec![30]);
    }

    #[test]
    fn live_hotkey_that_refuses_release_stays_owned() {
        let xml = plist(&[entry(28, false), entry(30, false), entry(184, false)]);
        let live = FakeLive::new(&[30], false);
        assert_eq!(reconcile_owned_keys(&xml, &live), vec!["4"]);
    }

    #[test]
    fn hotkeys_enabled_in_prefs_are_never_released() {
        let xml = plist(&[entry(28, false), entry(30, true), entry(184, false)]);
        let live = FakeLive::new(&[30], true);
        assert_eq!(reconcile_owned_keys(&xml, &live), vec!["4"]);
        assert!(live.disabled.borrow().is_empty());
    }
}
