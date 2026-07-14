# Self-Timer Uses Last-Used Capture Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The tray Self-Timer fires the capture mode the user last triggered (area/window/fullscreen) instead of always fullscreen.

**Architecture:** Add an in-memory `last_capture_mode: Mutex<CaptureMode>` to `AppState`. `trigger_capture` (single entry point for hotkeys, tray, and the `capture_screen` IPC command) records the mode; `timed_capture_fire` reads it instead of hardcoding `CaptureMode::Fullscreen`. Timed fires call `capture_and_publish` directly, so they never overwrite the remembered mode.

**Tech Stack:** Rust (Tauri v2 backend). No frontend changes.

## Global Constraints

- Default mode when no capture has happened this session: `CaptureMode::Fullscreen` (spec).
- In-memory only — no settings.json / EditorPrefs changes (spec).
- Scrolling capture must not affect the remembered mode (it already uses a separate path; do not touch it).
- Before calling the change done: `npm run build`, `npm test`, and `cd src-tauri && cargo test` must pass (CLAUDE.md).

**Testing note:** `CaptureMode` already derives `Clone, Copy` (`src-tauri/src/capture/mod.rs:14`), so the spec's derive step is already satisfied. The change is Mutex store/read wired to Tauri's `AppHandle`-managed state; there is no pure function to unit-test without constructing a full Tauri `App` (which the existing test suite never does). Verification is via the existing suites (regression) plus the manual checks in Task 1 Step 5.

---

### Task 1: Remember and reuse the last capture mode

**Files:**
- Modify: `src-tauri/src/commands.rs:8-23` (AppState), `:27-35` (trigger_capture), `:343-355` (timed_capture_fire)
- Modify: `src-tauri/src/lib.rs:30-38` (AppState initializer)

**Interfaces:**
- Consumes: `CaptureMode` (`crate::capture`, derives `Clone, Copy`), existing `AppState`, `trigger_capture`, `timed_capture_fire`, `capture_and_publish`.
- Produces: `AppState.last_capture_mode: std::sync::Mutex<CaptureMode>` — read/written only inside `commands.rs`; no new public API.

- [ ] **Step 1: Add the field to `AppState`**

In `src-tauri/src/commands.rs`, after the `timer_seconds` field:

```rust
    /// Seconds for the pending self-timer; the timer window pulls this on load.
    pub timer_seconds: std::sync::Mutex<u32>,
    /// Mode of the most recent user-triggered capture; the self-timer fires
    /// this mode. Defaults to Fullscreen until a capture is taken.
    pub last_capture_mode: std::sync::Mutex<CaptureMode>,
```

- [ ] **Step 2: Initialize it in `lib.rs`**

In `src-tauri/src/lib.rs`, in the `app.manage(AppState { ... })` block, after `timer_seconds`:

```rust
                timer_seconds: std::sync::Mutex::new(5),
                last_capture_mode: std::sync::Mutex::new(capture::CaptureMode::Fullscreen),
```

If `capture::CaptureMode` is not in scope in lib.rs, use the full path `crate::capture::CaptureMode::Fullscreen` (do not add a new `use` line just for this).

- [ ] **Step 3: Record the mode in `trigger_capture`**

In `src-tauri/src/commands.rs`, at the top of `trigger_capture` (before the `spawn_blocking`):

```rust
pub fn trigger_capture(app: &AppHandle, mode: CaptureMode) {
    *app.state::<AppState>().last_capture_mode.lock().unwrap() = mode;
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(err) = capture_and_publish(&app, mode) {
            eprintln!("capture failed: {err}");
            let _ = app.emit("capture:error", err.to_string());
        }
    });
}
```

- [ ] **Step 4: Read it in `timed_capture_fire`**

Replace the hardcoded `CaptureMode::Fullscreen` in `timed_capture_fire`:

```rust
#[tauri::command]
pub fn timed_capture_fire(app: AppHandle) {
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.destroy();
    }
    let mode = *app.state::<AppState>().last_capture_mode.lock().unwrap();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Err(err) = capture_and_publish(&app, mode) {
            eprintln!("timed capture failed: {err}");
            let _ = app.emit("capture:error", err.to_string());
        }
    });
}
```

- [ ] **Step 5: Verify**

Run (each from repo root unless noted):

```bash
cd src-tauri && cargo test        # Expected: all existing tests PASS
npm run build                     # Expected: tsc + vite succeed
npm test                          # Expected: vitest PASS
```

Manual check with `npm run tauri dev`:
1. Tray → Self-Timer → 3 seconds on a fresh launch → fullscreen capture fires at zero (default).
2. Take an Area capture (Cmd+Shift+7), then tray → Self-Timer → 3 seconds → interactive crosshair appears when the countdown ends.
3. After the timed area capture, run the timer again without any new capture → still area (the timed fire didn't overwrite the mode — it never goes through `trigger_capture`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "Self-timer fires the last-used capture mode instead of always fullscreen"
```
