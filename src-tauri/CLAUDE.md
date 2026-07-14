
## Settings

User settings live in `$APPDATA/settings.json` (`src-tauri/src/settings.rs`, `SettingsStore` with serde defaults — unknown/missing fields fall back safely, values are clamped in `sanitized`). The hidden `main` window is the Settings UI (`src/settings/`), opened from the tray; closing it hides it (`on_window_event` in lib.rs). Changes save immediately over IPC and broadcast via the `settings:changed` event; the overlay re-arms its auto-close timer on it, and `show_overlay` reads position / size / active-monitor placement (`cursor_position` + `monitor_from_point`) per capture. Overlay size slider steps map to multipliers in `src/settings/main.ts` (`SIZE_STEPS`) against the base size constants in `commands.rs`.
