# Self-timer uses last-used capture mode

**Date:** 2026-07-13
**Status:** Approved

## Problem

The tray Self-Timer (3/5/10 s) always fires a fullscreen capture —
`timed_capture_fire` in `src-tauri/src/commands.rs` hardcodes
`CaptureMode::Fullscreen`. Users expect the timer to repeat whichever
capture type they last chose (area / window / fullscreen).

## Decisions

- **Source of truth:** the mode of the most recent user-triggered capture
  (hotkey, tray item, or `capture_screen` IPC). In-memory only; not
  persisted across restarts.
- **Default:** `Fullscreen` when no capture has been taken yet this session
  (matches today's behavior).
- **Interactive modes:** if the last mode was Area or Window, the countdown
  ends and then the interactive `screencapture -i` crosshair/window-picker
  appears. The timer buys time to arrange the screen; the user selects at
  fire time. No pre-selection machinery.

## Design

1. Add `last_capture_mode: Mutex<CaptureMode>` to `AppState`
   (`commands.rs`), initialized to `CaptureMode::Fullscreen`.
2. `trigger_capture(app, mode)` — the single entry point for hotkeys, tray,
   and the `capture_screen` command — stores `mode` into that state before
   capturing.
3. `timed_capture_fire` reads the stored mode and passes it to
   `capture_and_publish` instead of the hardcoded `Fullscreen`.
4. `CaptureMode` derives `Clone`/`Copy` if it does not already.

Non-goals / unaffected paths:

- Timed fires call `capture_and_publish` directly, so a timer shot never
  overwrites the remembered mode.
- Scrolling capture uses its own path (`run_scrolling_capture`) and does
  not affect the remembered mode.
- No frontend changes: the timer window only counts down and invokes
  `timed_capture_fire`.
- No settings.json or EditorPrefs changes.

## Testing

- `cargo test`, `npm run build`, `npm test` remain green.
- Manual: take an area capture → start a 3 s timer → crosshair appears at
  zero. Fresh launch → timer → fullscreen capture.
