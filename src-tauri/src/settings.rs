use std::path::PathBuf;
use std::sync::Mutex;

use crate::shortcuts::{self, ShortcutAction};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayPosition {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoCloseAction {
    Close,
    SaveAndClose,
}

/// UI languages selectable in Settings; anything else is reset to "system".
pub const LANGUAGES: &[&str] = &["system", "en-GB", "es", "fr", "de", "it"];

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub position: OverlayPosition,
    pub move_to_active_screen: bool,
    /// Overlay size multiplier over the base panel size.
    pub overlay_size: f64,
    pub auto_close_enabled: bool,
    pub auto_close_action: AutoCloseAction,
    pub auto_close_seconds: u32,
    pub close_after_drag: bool,
    /// "system" (follow the OS locale) or one of the supported tags.
    pub language: String,
    /// Global-shortcut accelerator strings, one per capture action; parseable
    /// by both the global-shortcut plugin and the tray menu (shortcuts.rs).
    pub shortcut_area: String,
    pub shortcut_window: String,
    pub shortcut_fullscreen: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            position: OverlayPosition::Left,
            move_to_active_screen: true,
            overlay_size: 1.0,
            auto_close_enabled: false,
            auto_close_action: AutoCloseAction::Close,
            auto_close_seconds: 30,
            close_after_drag: true,
            language: "system".into(),
            shortcut_area: ShortcutAction::Area.default_accel().into(),
            shortcut_window: ShortcutAction::Window.default_accel().into(),
            shortcut_fullscreen: ShortcutAction::Fullscreen.default_accel().into(),
        }
    }
}

impl Settings {
    /// Reject out-of-range values coming over IPC or from a hand-edited file.
    pub fn sanitized(mut self) -> Self {
        self.overlay_size = self.overlay_size.clamp(0.75, 2.0);
        self.auto_close_seconds = self.auto_close_seconds.clamp(3, 600);
        if !LANGUAGES.contains(&self.language.as_str()) {
            self.language = "system".into();
        }
        // Shortcuts: invalid entries reset to their defaults, and a combo
        // colliding with an earlier action loses to it (area → window →
        // fullscreen order keeps the outcome deterministic).
        let mut seen = Vec::new();
        for action in shortcuts::ACTIONS {
            let parsed = shortcuts::validate(self.shortcut(action)).ok();
            let parsed = match parsed {
                Some(p) if !seen.contains(&p) => p,
                _ => {
                    *self.shortcut_mut(action) = action.default_accel().into();
                    shortcuts::validate(action.default_accel()).expect("defaults are valid")
                }
            };
            seen.push(parsed);
        }
        self
    }

    pub fn shortcut(&self, action: ShortcutAction) -> &str {
        match action {
            ShortcutAction::Area => &self.shortcut_area,
            ShortcutAction::Window => &self.shortcut_window,
            ShortcutAction::Fullscreen => &self.shortcut_fullscreen,
        }
    }

    pub fn shortcut_mut(&mut self, action: ShortcutAction) -> &mut String {
        match action {
            ShortcutAction::Area => &mut self.shortcut_area,
            ShortcutAction::Window => &mut self.shortcut_window,
            ShortcutAction::Fullscreen => &mut self.shortcut_fullscreen,
        }
    }
}

/// Last-used annotation tool/color/stroke, restored when the editor opens.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EditorPrefs {
    pub tool: String,
    pub color: String,
    pub stroke_width: u32,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            tool: "arrow".into(),
            color: "#ff3b30".into(),
            stroke_width: 4,
        }
    }
}

/// JSON-file-backed store: a missing or corrupt file yields `T::default()`,
/// and `sanitize` normalizes values on load and set (identity when a type has
/// no invariants to enforce).
pub struct JsonStore<T> {
    path: PathBuf,
    sanitize: fn(T) -> T,
    current: Mutex<T>,
}

impl<T> JsonStore<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Default + Clone,
{
    fn load_with(path: PathBuf, sanitize: fn(T) -> T) -> Self {
        let current = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<T>(&bytes).ok())
            .map(sanitize)
            .unwrap_or_default();
        Self {
            path,
            sanitize,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> T {
        self.current.lock().unwrap().clone()
    }

    pub fn set(&self, value: T) -> std::io::Result<T> {
        let value = (self.sanitize)(value);
        *self.current.lock().unwrap() = value.clone();
        std::fs::write(&self.path, serde_json::to_vec_pretty(&value)?)?;
        Ok(value)
    }
}

pub type SettingsStore = JsonStore<Settings>;
pub type EditorPrefsStore = JsonStore<EditorPrefs>;

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        Self::load_with(path, Settings::sanitized)
    }
}

impl EditorPrefsStore {
    pub fn load(path: PathBuf) -> Self {
        Self::load_with(path, |prefs| prefs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sfm-settings-{}-{name}.json", std::process::id()))
    }

    #[test]
    fn missing_or_corrupt_file_yields_defaults() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);
        assert_eq!(SettingsStore::load(path.clone()).get(), Settings::default());
        std::fs::write(&path, b"{not json").unwrap();
        assert_eq!(SettingsStore::load(path.clone()).get(), Settings::default());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_persists_and_reloads() {
        let path = temp_path("roundtrip");
        let store = SettingsStore::load(path.clone());
        let mut s = Settings::default();
        s.position = OverlayPosition::Right;
        s.auto_close_enabled = true;
        s.auto_close_seconds = 10;
        store.set(s.clone()).unwrap();
        let reloaded = SettingsStore::load(path.clone()).get();
        assert_eq!(reloaded, s);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sanitize_clamps_ranges() {
        let s = Settings {
            overlay_size: 99.0,
            auto_close_seconds: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.overlay_size, 2.0);
        assert_eq!(s.auto_close_seconds, 3);
    }

    #[test]
    fn sanitize_resets_unknown_language() {
        let s = Settings {
            language: "zz".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.language, "system");
        for tag in LANGUAGES {
            let s = Settings {
                language: (*tag).into(),
                ..Default::default()
            }
            .sanitized();
            assert_eq!(s.language, *tag);
        }
    }

    #[test]
    fn sanitize_resets_invalid_shortcuts() {
        let s = Settings {
            shortcut_area: "garbage".into(),
            shortcut_window: "Shift+8".into(), // no non-Shift modifier
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.shortcut_area, ShortcutAction::Area.default_accel());
        assert_eq!(s.shortcut_window, ShortcutAction::Window.default_accel());
    }

    #[test]
    fn sanitize_resets_duplicate_shortcuts_keeping_the_earlier_action() {
        let s = Settings {
            shortcut_window: "Alt+Shift+K".into(),
            shortcut_fullscreen: "Alt+Shift+K".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.shortcut_window, "Alt+Shift+K");
        assert_eq!(s.shortcut_fullscreen, ShortcutAction::Fullscreen.default_accel());
    }

    #[test]
    fn sanitize_preserves_macos_screenshot_shortcuts() {
        // Cmd+Shift+3/4/5 are assignable (the user may have freed them in
        // System Settings); ownership is checked at assign time, not here.
        let s = Settings {
            shortcut_area: "Cmd+Shift+3".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.shortcut_area, "Cmd+Shift+3");
    }

    #[test]
    fn custom_shortcuts_roundtrip() {
        let path = temp_path("shortcuts");
        let store = SettingsStore::load(path.clone());
        let mut s = Settings::default();
        s.shortcut_area = "Alt+Shift+1".into();
        s.shortcut_window = "Ctrl+Alt+W".into();
        store.set(s.clone()).unwrap();
        let reloaded = SettingsStore::load(path.clone()).get();
        assert_eq!(reloaded, s);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_fields_and_partial_files_still_load() {
        let path = temp_path("partial");
        std::fs::write(&path, br#"{"position":"center","future_field":1}"#).unwrap();
        let s = SettingsStore::load(path.clone()).get();
        assert_eq!(s.position, OverlayPosition::Center);
        assert!(s.move_to_active_screen, "missing fields take defaults");
        let _ = std::fs::remove_file(&path);
    }
}
