# Design: "Launch on start" setting

Date: 2026-07-14
Status: approved

## Purpose

Add a "Launch on start" checkbox to the Settings window that registers the app
to launch at login (macOS LaunchAgent, Linux XDG autostart).

## Approach

Use the official `tauri-plugin-autostart` (Tauri v2). The OS registration is
the **single source of truth** — no new field in `Settings`/settings.json.
This can never drift from reality (e.g. the user removes the login item in
System Settings); the checkbox always shows the actual state.

Rejected alternative: storing `launch_on_start` in settings.json and syncing
the OS login item on every save/app start — consistent with existing settings
plumbing but can drift and re-registers on every launch.

## Details

- **Rust**: register the plugin in `lib.rs` with `MacosLauncher::LaunchAgent`
  and no extra launch args. Add `tauri-plugin-autostart` to
  `src-tauri/Cargo.toml`.
- **Frontend**: add `@tauri-apps/plugin-autostart` npm package. The settings
  window (`src/settings/`) calls `isEnabled()` on load to set the checkbox and
  `enable()`/`disable()` on toggle. On failure, revert the checkbox and log
  the error. The checkbox is excluded from the generic `readForm`/`fillForm`
  settings plumbing since it is not part of `Settings`.
- **UI**: new "General:" row with a "Launch on start" checkbox at the top of
  the settings window, above "Position on screen:", separated with the
  existing divider style. Same markup/CSS as existing checkboxes.
- **Permissions**: add `autostart:allow-enable`, `autostart:allow-disable`,
  `autostart:allow-is-enabled` to `src-tauri/capabilities/default.json`
  (applies to all windows).
- **Default**: unregistered (off) — matches current behavior.

## Verification

- `npm run build`, `npm test`, `cargo test` still pass.
- Manual: toggling the checkbox creates/removes
  `~/Library/LaunchAgents/com.screenforme.app.plist`; after removing the login
  item externally, reopening Settings shows the checkbox unchecked.

## Caveats

- In `npm run tauri dev` the login item points at the dev binary; the feature
  is only meaningful for the packaged .app, but the toggle still works.
