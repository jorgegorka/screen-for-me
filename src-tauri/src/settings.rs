use std::path::PathBuf;
use std::sync::Mutex;

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

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
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
        }
    }
}

impl Settings {
    /// Reject out-of-range values coming over IPC or from a hand-edited file.
    pub fn sanitized(mut self) -> Self {
        self.overlay_size = self.overlay_size.clamp(0.75, 2.0);
        self.auto_close_seconds = self.auto_close_seconds.clamp(3, 600);
        self
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let current = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
            .map(Settings::sanitized)
            .unwrap_or_default();
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> Settings {
        *self.current.lock().unwrap()
    }

    pub fn set(&self, settings: Settings) -> std::io::Result<Settings> {
        let settings = settings.sanitized();
        *self.current.lock().unwrap() = settings;
        std::fs::write(&self.path, serde_json::to_vec_pretty(&settings)?)?;
        Ok(settings)
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
        store.set(s).unwrap();
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
    fn unknown_fields_and_partial_files_still_load() {
        let path = temp_path("partial");
        std::fs::write(&path, br#"{"position":"center","future_field":1}"#).unwrap();
        let s = SettingsStore::load(path.clone()).get();
        assert_eq!(s.position, OverlayPosition::Center);
        assert!(s.move_to_active_screen, "missing fields take defaults");
        let _ = std::fs::remove_file(&path);
    }
}
