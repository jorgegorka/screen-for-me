# Scrolling Capture & Self-Timer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two capture modes: Self-Timer (3/5/10 s countdown, then fullscreen capture) and Scrolling Capture (macOS-only auto-scroll of a selected region, frames stitched into one PNG).

**Architecture:** Both features feed the existing pipeline (save to `captures/`, `History::prune`, `capture:new`, quick-access overlay). Self-Timer is a transient countdown window that fires the existing fullscreen path. Scrolling Capture is a transient full-screen selection window + a Rust loop: `screencapture -x -R` region grabs → CGEvent scroll steps → correlation-based stitching with the `image` crate.

**Tech Stack:** Tauri v2 (Rust backend, TS/Vite frontend), `core-graphics` (already a dep), new macOS-only deps `image` (png only) and `core-foundation`.

**Spec:** `docs/superpowers/specs/2026-07-13-scrolling-capture-self-timer-design.md`

## Global Constraints

- Scrolling Capture is **macOS-only**; its tray item and capture modules are `#[cfg(target_os = "macos")]`. Self-Timer works on macOS and Linux.
- Both new windows (`timer`, `scrollcap`) are transient: created on demand, **destroyed** when done — do NOT add them to the hide-instead-of-close list in `lib.rs`.
- Every new window label must be added to `src-tauri/capabilities/default.json` (`windows` array) or its JS API calls fail silently.
- Hard caps for scrolling: max 40 frames, composite scroll-axis dimension ≤ 20,000 px.
- Scroll events use LINE units (no trackpad inertia); ~350 ms settle between steps.
- Gates before calling the work done: `npm run build`, `npm test`, `cd src-tauri && cargo test` — all green.
- Commit after every task with a descriptive message.
- Tray menu order: Area / Window / Fullscreen / separator / Scrolling Capture / Self-Timer submenu / separator / History / …

---

### Task 1: Self-Timer — Rust plumbing (state, commands, window, tray submenu)

**Files:**
- Modify: `src-tauri/src/commands.rs` (AppState fields, `active_monitor` helper, three timer functions)
- Modify: `src-tauri/src/windows.rs` (add `open_timer`)
- Modify: `src-tauri/src/tray.rs` (Self-Timer submenu)
- Modify: `src-tauri/src/lib.rs` (AppState init, command registration)
- Modify: `src-tauri/capabilities/default.json` (add `timer` label)

**Interfaces:**
- Consumes: existing `capture_and_publish(app, CaptureMode::Fullscreen)`, `cursor_point(app)`.
- Produces: `commands::start_timed_capture(app: &AppHandle, seconds: u32)` (called from tray); IPC commands `timer_duration() -> u32` and `timed_capture_fire()`; `commands::active_monitor(app: &AppHandle) -> Option<tauri::Monitor>` (reused by Task 5); `windows::open_timer(app: &AppHandle) -> tauri::Result<()>`.

- [ ] **Step 1: Add AppState fields**

In `src-tauri/src/commands.rs`, extend `AppState` (the `scroll_stop` flag is added now so `lib.rs` is only touched once for state):

```rust
pub struct AppState {
    pub history: History,
    pub settings: SettingsStore,
    pub editor_prefs: EditorPrefsStore,
    /// Capture the editor window should be showing. The editor pulls this on
    /// load, so opening never depends on an event landing before the page's
    /// listener is ready.
    pub editor_target: std::sync::Mutex<Option<String>>,
    /// Seconds for the pending self-timer; the timer window pulls this on load.
    pub timer_seconds: std::sync::Mutex<u32>,
    /// Set by `stop_scrolling_capture` to end the scroll loop early.
    pub scroll_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
```

In `src-tauri/src/lib.rs`, extend the `app.manage(AppState { ... })` call:

```rust
            app.manage(AppState {
                history: History::new(data_dir.join("captures"))?,
                settings: SettingsStore::load(data_dir.join("settings.json")),
                editor_prefs: EditorPrefsStore::load(data_dir.join("editor_prefs.json")),
                editor_target: std::sync::Mutex::new(None),
                timer_seconds: std::sync::Mutex::new(5),
                scroll_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            });
```

- [ ] **Step 2: Extract the `active_monitor` helper**

In `src-tauri/src/commands.rs`, add below `cursor_point` (Task 5's scrollcap window reuses it):

```rust
/// The monitor under the cursor, falling back to the primary one.
pub(crate) fn active_monitor(app: &AppHandle) -> Option<tauri::Monitor> {
    cursor_point(app)
        .and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
}
```

Leave `show_overlay` unchanged — it gates the cursor lookup behind `settings.move_to_active_screen`, so it keeps its own logic. The helper is for the new transient windows, which always follow the cursor.

- [ ] **Step 3: Add the three timer functions**

In `src-tauri/src/commands.rs`:

```rust
/// Tray entry point: stage the duration and show the countdown window.
/// Starting a new timer replaces a running one.
pub fn start_timed_capture(app: &AppHandle, seconds: u32) {
    if let Some(existing) = app.get_webview_window("timer") {
        let _ = existing.destroy();
    }
    *app.state::<AppState>().timer_seconds.lock().unwrap() = seconds;
    if let Err(err) = crate::windows::open_timer(app) {
        eprintln!("failed to open timer window: {err}");
    }
}

/// Duration for the countdown window (pull model, like `editor_target`).
#[tauri::command]
pub fn timer_duration(state: State<AppState>) -> u32 {
    *state.timer_seconds.lock().unwrap()
}

/// Countdown reached zero: tear the window down, wait a beat so it cannot
/// appear in the shot, then run the normal fullscreen path.
#[tauri::command]
pub fn timed_capture_fire(app: AppHandle) {
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.destroy();
    }
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Err(err) = capture_and_publish(&app, CaptureMode::Fullscreen) {
            eprintln!("timed capture failed: {err}");
            let _ = app.emit("capture:error", err.to_string());
        }
    });
}
```

- [ ] **Step 4: Add `open_timer` to `src-tauri/src/windows.rs`**

```rust
/// Transient countdown window, centered on the active monitor. Destroyed (not
/// hidden) when the timer fires or is cancelled.
pub fn open_timer(app: &AppHandle) -> tauri::Result<()> {
    const SIZE: f64 = 180.0;
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "timer",
        tauri::WebviewUrl::App("timer.html".into()),
    )
    .title("Self-Timer")
    .inner_size(SIZE, SIZE)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .accept_first_mouse(true);
    if let Some(monitor) = crate::commands::active_monitor(app) {
        let scale = monitor.scale_factor();
        let pos = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        builder = builder.position(
            pos.x + (size.width - SIZE) / 2.0,
            pos.y + (size.height - SIZE) / 2.0,
        );
    }
    builder.build()?;
    Ok(())
}
```

Add `use tauri::Manager;` only if not already imported (it is — check the top of the file).

- [ ] **Step 5: Tray submenu**

In `src-tauri/src/tray.rs`, change the menu import and add the submenu. New import line:

```rust
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
```

After the `fullscreen` item, add:

```rust
    let timer_3 = MenuItem::with_id(app, "timer_3", "3 seconds", true, None::<&str>)?;
    let timer_5 = MenuItem::with_id(app, "timer_5", "5 seconds", true, None::<&str>)?;
    let timer_10 = MenuItem::with_id(app, "timer_10", "10 seconds", true, None::<&str>)?;
    let self_timer = Submenu::with_items(app, "Self-Timer", true, &[&timer_3, &timer_5, &timer_10])?;
```

Because Task 5 inserts a `#[cfg]`-gated item here, switch the menu construction from the array literal to a Vec now:

```rust
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let sep4 = PredefinedMenuItem::separator(app)?;
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        vec![&area, &window, &fullscreen, &sep1];
    items.push(&self_timer);
    items.extend_from_slice(&[
        &sep2, &history, &sep3, &about, &updates, &sep4, &settings, &quit,
    ]);
    let menu = Menu::with_items(app, &items)?;
```

(Delete the old `let sep = || ...` closure and `Menu::with_items(app, &[...])` block.)

In `on_menu_event`, add before the `_ => {}` arm:

```rust
            "timer_3" => crate::commands::start_timed_capture(app, 3),
            "timer_5" => crate::commands::start_timed_capture(app, 5),
            "timer_10" => crate::commands::start_timed_capture(app, 10),
```

- [ ] **Step 6: Register commands and capability**

In `src-tauri/src/lib.rs`, add to `generate_handler![]`:

```rust
            commands::timer_duration,
            commands::timed_capture_fire,
```

In `src-tauri/capabilities/default.json`, add `"timer"` to the `windows` array:

```json
  "windows": [
    "main",
    "overlay",
    "editor",
    "history",
    "timer"
  ],
```

- [ ] **Step 7: Verify it compiles and existing tests pass**

Run: `cd src-tauri && cargo test`
Expected: compiles; all existing tests PASS (capture validation, history). There is no new pure logic in this task — the deliverable is verified end-to-end in Task 2.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "Add Self-Timer backend: timer window plumbing, tray submenu, commands"
```

---

### Task 2: Self-Timer — countdown page

**Files:**
- Create: `timer.html` (repo root, beside `overlay.html`)
- Create: `src/timer/main.ts`
- Create: `src/timer/timer.css`
- Modify: `vite.config.ts` (add input)

**Interfaces:**
- Consumes: IPC `timer_duration() -> number`, `timed_capture_fire()` (Task 1).
- Produces: nothing consumed by later tasks.

- [ ] **Step 1: Create `timer.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Self-Timer</title>
    <link rel="stylesheet" href="/src/timer/timer.css" />
    <script type="module" src="/src/timer/main.ts" defer></script>
  </head>
  <body>
    <div class="disc" id="disc">
      <div id="count"></div>
      <div class="hint">Click to cancel</div>
    </div>
  </body>
</html>
```

- [ ] **Step 2: Create `src/timer/timer.css`**

```css
html,
body {
  margin: 0;
  height: 100%;
  background: transparent;
  overflow: hidden;
  user-select: none;
  -webkit-user-select: none;
}

.disc {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: rgba(24, 24, 26, 0.85);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

#count {
  font: 600 72px -apple-system, system-ui, sans-serif;
  font-variant-numeric: tabular-nums;
}

.hint {
  position: absolute;
  bottom: 16px;
  width: 100%;
  text-align: center;
  font: 12px -apple-system, system-ui, sans-serif;
  color: rgba(255, 255, 255, 0.55);
}
```

- [ ] **Step 3: Create `src/timer/main.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const count = document.getElementById("count") as HTMLDivElement;

async function main() {
  let remaining = await invoke<number>("timer_duration");
  count.textContent = String(remaining);

  const tick = window.setInterval(() => {
    remaining -= 1;
    if (remaining <= 0) {
      window.clearInterval(tick);
      // Rust destroys this window before capturing, so no cleanup here.
      void invoke("timed_capture_fire");
    } else {
      count.textContent = String(remaining);
    }
  }, 1000);

  const cancel = () => {
    window.clearInterval(tick);
    void appWindow.close();
  };
  document.addEventListener("click", cancel);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") cancel();
  });
}

void main();
```

- [ ] **Step 4: Register the Vite input**

In `vite.config.ts`, extend `build.rollupOptions.input`:

```ts
      input: {
        main: "index.html",
        overlay: "overlay.html",
        editor: "editor.html",
        history: "history.html",
        timer: "timer.html",
      },
```

- [ ] **Step 5: Verify frontend builds and tests pass**

Run: `npm run build && npm test`
Expected: tsc + vite succeed with `timer` in the emitted inputs; existing vitest suites PASS.

- [ ] **Step 6: Manual smoke test**

Run: `npm run tauri dev`, then tray → Self-Timer → 3 seconds.
Expected: a round countdown appears centered, counts 3-2-1, disappears, a fullscreen capture lands in the overlay. Repeat and click the disc mid-count: window closes, no capture. (Screen Recording TCC attaches to the terminal in dev — see CLAUDE.md if the capture is wallpaper-only.)

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "Add Self-Timer countdown page"
```

---

### Task 3: Stitching module (pure, TDD)

**Files:**
- Modify: `src-tauri/Cargo.toml` (macOS deps)
- Modify: `src-tauri/src/capture/mod.rs` (declare modules)
- Create: `src-tauri/src/capture/stitch.rs` (implementation + tests in one file, matching `mod.rs` style)

**Interfaces:**
- Consumes: `image` crate only — no Tauri, no I/O (unit-testable like `geometry.ts`).
- Produces (used by Task 5):
  - `pub enum ScrollDirection { Up, Down, Left, Right }` (serde `snake_case` Deserialize, `Clone, Copy, PartialEq, Debug`)
  - `pub(crate) fn normalize(frame: &RgbaImage, dir: ScrollDirection) -> RgbaImage`
  - `pub(crate) fn denormalize(composite: RgbaImage, dir: ScrollDirection) -> RgbaImage`
  - `pub(crate) fn find_scroll_offset(prev: &RgbaImage, next: &RgbaImage) -> Option<u32>` — `Some(0)` = no movement, `None` = no confident match
  - `pub(crate) fn frames_identical(a: &RgbaImage, b: &RgbaImage) -> bool`
  - `pub(crate) fn append_rows(composite: RgbaImage, next: &RgbaImage, new_rows: u32) -> RgbaImage`

- [ ] **Step 1: Add macOS dependencies**

In `src-tauri/Cargo.toml`, extend the macOS target section:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.24"
core-foundation = "0.10"
image = { version = "0.25", default-features = false, features = ["png"] }
```

- [ ] **Step 2: Declare the module (macOS-only)**

In `src-tauri/src/capture/mod.rs`, next to the existing platform mods:

```rust
#[cfg(target_os = "macos")]
pub mod stitch;
```

- [ ] **Step 3: Write the failing tests**

Create `src-tauri/src/capture/stitch.rs` containing ONLY the test module for now (plus the `use` line):

```rust
use image::{GenericImage, GenericImageView, RgbaImage};

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic per-pixel noise so strips match at exactly one offset.
    fn noise_image(w: u32, h: u32, seed: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            let mut v = x
                .wrapping_mul(374_761_393)
                ^ y.wrapping_mul(668_265_263)
                ^ seed.wrapping_mul(2_246_822_519);
            v ^= v >> 13;
            v = v.wrapping_mul(1_274_126_177);
            image::Rgba([(v & 0xff) as u8, ((v >> 8) & 0xff) as u8, ((v >> 16) & 0xff) as u8, 255])
        })
    }

    /// A viewport-sized window into a taller source, like one scroll frame.
    fn window(src: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        src.view(x, y, w, h).to_image()
    }

    #[test]
    fn exact_offset_detected() {
        let src = noise_image(160, 600, 7);
        let prev = window(&src, 0, 0, 160, 200);
        let next = window(&src, 0, 37, 160, 200);
        assert_eq!(find_scroll_offset(&prev, &next), Some(37));
    }

    #[test]
    fn identical_frames_offset_zero() {
        let frame = noise_image(160, 200, 3);
        assert_eq!(find_scroll_offset(&frame, &frame), Some(0));
        assert!(frames_identical(&frame, &frame));
    }

    #[test]
    fn unrelated_frames_no_match() {
        let a = noise_image(160, 200, 1);
        let b = noise_image(160, 200, 2);
        assert_eq!(find_scroll_offset(&a, &b), None);
        assert!(!frames_identical(&a, &b));
    }

    #[test]
    fn append_reconstructs_source() {
        let src = noise_image(160, 600, 9);
        let prev = window(&src, 0, 0, 160, 200);
        let next = window(&src, 0, 37, 160, 200);
        let composite = append_rows(prev, &next, 37);
        assert_eq!(composite.dimensions(), (160, 237));
        assert_eq!(composite, window(&src, 0, 0, 160, 237));
    }

    #[test]
    fn normalize_round_trips_every_direction() {
        let frame = noise_image(90, 60, 5);
        for dir in [
            ScrollDirection::Up,
            ScrollDirection::Down,
            ScrollDirection::Left,
            ScrollDirection::Right,
        ] {
            assert_eq!(denormalize(normalize(&frame, dir), dir), frame, "{dir:?}");
        }
    }

    /// End-to-end for a horizontal direction: proves the rotation mapping puts
    /// new content at the bottom of the normalized frames.
    #[test]
    fn right_scroll_stitches_through_normalize() {
        let src = noise_image(600, 160, 11);
        let prev = window(&src, 0, 0, 200, 160);
        let next = window(&src, 37, 0, 200, 160);
        let dir = ScrollDirection::Right;
        let prev_n = normalize(&prev, dir);
        let next_n = normalize(&next, dir);
        let offset = find_scroll_offset(&prev_n, &next_n).expect("confident match");
        assert_eq!(offset, 37);
        let composite = denormalize(append_rows(prev_n, &next_n, offset), dir);
        assert_eq!(composite, window(&src, 0, 0, 237, 160));
    }

    /// Same for Up: new content enters at the top, composite grows upward.
    #[test]
    fn up_scroll_stitches_through_normalize() {
        let src = noise_image(160, 600, 13);
        let prev = window(&src, 0, 400, 160, 200);
        let next = window(&src, 0, 363, 160, 200);
        let dir = ScrollDirection::Up;
        let prev_n = normalize(&prev, dir);
        let next_n = normalize(&next, dir);
        let offset = find_scroll_offset(&prev_n, &next_n).expect("confident match");
        assert_eq!(offset, 37);
        let composite = denormalize(append_rows(prev_n, &next_n, offset), dir);
        assert_eq!(composite, window(&src, 0, 363, 160, 237));
    }
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cd src-tauri && cargo test stitch`
Expected: FAIL to compile — `ScrollDirection`, `find_scroll_offset`, etc. not found.

- [ ] **Step 5: Write the implementation**

At the top of `src-tauri/src/capture/stitch.rs`, above the test module:

```rust
//! Pure stitching math for scrolling capture. Deliberately free of Tauri and
//! file I/O so `cargo test` covers it (same idea as the editor's geometry.ts).
//!
//! Everything works on "normalized" frames: `normalize` rotates/flips each
//! frame so the scroll direction becomes "down", stitching always appends rows
//! at the bottom, and `denormalize` undoes the transform on the final image.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

pub(crate) fn normalize(frame: &RgbaImage, dir: ScrollDirection) -> RgbaImage {
    use image::imageops::{flip_vertical, rotate270, rotate90};
    match dir {
        ScrollDirection::Down => frame.clone(),
        ScrollDirection::Up => flip_vertical(frame),
        // Content moving right maps to moving down under a clockwise rotation.
        ScrollDirection::Right => rotate90(frame),
        ScrollDirection::Left => rotate270(frame),
    }
}

pub(crate) fn denormalize(composite: RgbaImage, dir: ScrollDirection) -> RgbaImage {
    use image::imageops::{flip_vertical, rotate270, rotate90};
    match dir {
        ScrollDirection::Down => composite,
        ScrollDirection::Up => flip_vertical(&composite),
        ScrollDirection::Right => rotate270(&composite),
        ScrollDirection::Left => rotate90(&composite),
    }
}

const STRIP_ROWS: u32 = 32;
const SAMPLE_STEP: u32 = 4;
/// Mean per-channel abs difference below which a strip match is trusted.
const MAX_MEAN_DIFF: f64 = 6.0;

/// How many pixels the content moved between two normalized frames: slide the
/// top strip of `next` down `prev` and take the best (smallest-offset) match.
/// Browsers smooth-scroll, so the nominal scroll amount can't be trusted.
pub(crate) fn find_scroll_offset(prev: &RgbaImage, next: &RgbaImage) -> Option<u32> {
    let (w, h) = prev.dimensions();
    if next.dimensions() != (w, h) || h <= STRIP_ROWS || w == 0 {
        return None;
    }
    let mut best_offset = 0u32;
    let mut best_diff = f64::MAX;
    for offset in 0..=(h - STRIP_ROWS) {
        let mut sum = 0u64;
        let mut samples = 0u64;
        let mut y = 0;
        while y < STRIP_ROWS {
            let mut x = 0;
            while x < w {
                let a = prev.get_pixel(x, y + offset).0;
                let b = next.get_pixel(x, y).0;
                for c in 0..3 {
                    sum += (i32::from(a[c]) - i32::from(b[c])).unsigned_abs() as u64;
                }
                samples += 3;
                x += SAMPLE_STEP;
            }
            y += SAMPLE_STEP;
        }
        let diff = sum as f64 / samples as f64;
        if diff < best_diff {
            best_diff = diff;
            best_offset = offset;
        }
    }
    (best_diff <= MAX_MEAN_DIFF).then_some(best_offset)
}

/// Sampled full-frame equality — distinguishes "reached the end of the page"
/// from "a fixed header made offset 0 look like the best match".
pub(crate) fn frames_identical(a: &RgbaImage, b: &RgbaImage) -> bool {
    if a.dimensions() != b.dimensions() {
        return false;
    }
    let (w, h) = a.dimensions();
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            if a.get_pixel(x, y) != b.get_pixel(x, y) {
                return false;
            }
            x += SAMPLE_STEP;
        }
        y += SAMPLE_STEP;
    }
    true
}

/// Grow the composite by the `new_rows` bottom rows of `next`.
pub(crate) fn append_rows(composite: RgbaImage, next: &RgbaImage, new_rows: u32) -> RgbaImage {
    let (w, composite_h) = composite.dimensions();
    let (next_w, next_h) = next.dimensions();
    let new_rows = new_rows.min(next_h);
    let mut out = RgbaImage::new(w, composite_h + new_rows);
    out.copy_from(&composite, 0, 0).expect("composite fits");
    let strip = next.view(0, next_h - new_rows, next_w.min(w), new_rows).to_image();
    out.copy_from(&strip, 0, composite_h).expect("strip fits");
    out
}
```

Note: if `rotate90`/`rotate270` turn out to rotate the opposite way in this `image` version, `right_scroll_stitches_through_normalize` fails — fix by swapping the two calls in BOTH `normalize` and `denormalize` (they must stay inverses).

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd src-tauri && cargo test stitch`
Expected: 7 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "Add pure stitching module for scrolling capture (TDD)"
```

---

### Task 4: Scroll input module (Accessibility check, cursor warp, CGEvent scroll)

**Files:**
- Modify: `src-tauri/src/capture/mod.rs` (declare module)
- Create: `src-tauri/src/capture/scroll_input.rs`

**Interfaces:**
- Consumes: `ScrollDirection` from Task 3; `core-graphics`, `core-foundation`.
- Produces (used by Task 5): `pub fn ensure_accessibility() -> bool`, `pub fn warp_cursor(x: f64, y: f64)`, `pub fn post_scroll(direction: ScrollDirection, lines: i32) -> Result<(), String>`. Coordinates are global logical points (same space as `CGDisplayBounds` / `cursor_point`).

- [ ] **Step 1: Declare the module**

In `src-tauri/src/capture/mod.rs`:

```rust
#[cfg(target_os = "macos")]
pub mod scroll_input;
```

- [ ] **Step 2: Create `src-tauri/src/capture/scroll_input.rs`**

```rust
//! Synthetic scroll-wheel input and the Accessibility permission it needs.
//! Untestable side effects live here so the stitch module can stay pure.

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::CFString;
use core_graphics::display::CGDisplay;
use core_graphics::event::{CGEvent, CGEventTapLocation, ScrollEventUnit};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

use super::stitch::ScrollDirection;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

/// True when the app may post synthetic events. Passing the prompt option
/// also registers the app in System Settings → Accessibility on first ask.
pub fn ensure_accessibility() -> bool {
    let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let options =
        CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
}

/// Move the pointer so scroll events route to the window under the rect
/// (the system dispatches wheel events by cursor position, not event location).
pub fn warp_cursor(x: f64, y: f64) {
    let _ = CGDisplay::warp_mouse_cursor_position(CGPoint::new(x, y));
}

/// One scroll step. LINE units scroll discretely (no trackpad inertia), which
/// keeps the settle delay short and the frame offsets stitchable.
pub fn post_scroll(direction: ScrollDirection, lines: i32) -> Result<(), String> {
    // Positive wheel1 scrolls toward the top of the page; positive wheel2
    // toward the left edge. "Down" means revealing content below → negative.
    let (vertical, horizontal) = match direction {
        ScrollDirection::Down => (-lines, 0),
        ScrollDirection::Up => (lines, 0),
        ScrollDirection::Right => (0, -lines),
        ScrollDirection::Left => (0, lines),
    };
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "failed to create CGEventSource".to_string())?;
    let event = CGEvent::new_scroll_event(source, ScrollEventUnit::LINE, 2, vertical, horizontal, 0)
        .map_err(|_| "failed to create scroll event".to_string())?;
    event.post(CGEventTapLocation::HID);
    Ok(())
}
```

Note: if `core-graphics` 0.24 names differ slightly (e.g. `ScrollEventUnit` variant casing, `new_scroll_event` argument types), check `cargo doc -p core-graphics` / docs.rs and adapt — keep the semantics (line units, HID tap) identical.

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo test`
Expected: compiles and links against ApplicationServices; all tests still PASS (this module has no unit tests — it is pure side effects, exercised manually in Task 7).

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "Add macOS scroll input module (Accessibility check, cursor warp, CGEvent)"
```

---

### Task 5: Scrolling capture loop, commands, window, tray item

**Files:**
- Create: `src-tauri/src/capture/scrolling.rs`
- Modify: `src-tauri/src/capture/mod.rs` (declare module)
- Modify: `src-tauri/src/commands.rs` (`publish_capture` refactor, `SelectionRect`, two commands + macOS impl)
- Modify: `src-tauri/src/windows.rs` (add `open_scrollcap`)
- Modify: `src-tauri/src/tray.rs` (macOS-only item)
- Modify: `src-tauri/src/lib.rs` (register commands)
- Modify: `src-tauri/capabilities/default.json` (add `scrollcap` label)

**Interfaces:**
- Consumes: Task 3 stitch functions, Task 4 input functions, `History::new_capture_path`, `active_monitor` (Task 1).
- Produces: IPC commands `run_scrolling_capture(rect: {x,y,width,height}, direction: "up"|"down"|"left"|"right")` and `stop_scrolling_capture()`; events emitted to the `scrollcap` window: `scroll:running` (once, when the loop starts) and `scroll:progress` (frame count, each frame). The `scrollcap` window is destroyed by Rust when the run finishes.

- [ ] **Step 1: Create `src-tauri/src/capture/scrolling.rs`**

```rust
//! Auto-scroll capture loop: grab the region, scroll one step, stitch, repeat
//! until the frames stop changing, a cap is hit, or the user stops it.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use image::RgbaImage;

use super::stitch::{self, ScrollDirection};
use super::{scroll_input, CaptureError};

/// Region to grab, in global logical points (screencapture -R space).
#[derive(Clone, Copy)]
pub struct ScrollRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

const MAX_FRAMES: u32 = 40;
const MAX_COMPOSITE_PX: u32 = 20_000;
const SCROLL_LINES: i32 = 5;
/// One wheel line scrolls ≈ 10 points; only used when correlation fails.
const NOMINAL_POINTS_PER_LINE: f64 = 10.0;
const SETTLE: Duration = Duration::from_millis(350);

/// Run the loop and return the stitched image (already de-normalized).
/// A grab/scroll failure after ≥2 stitched frames returns the partial result —
/// a truncated scroll beats losing everything.
pub fn run(
    region: &ScrollRegion,
    direction: ScrollDirection,
    stop: &AtomicBool,
    work_dir: &Path,
    mut progress: impl FnMut(u32),
) -> Result<RgbaImage, CaptureError> {
    std::fs::create_dir_all(work_dir)?;
    let frame_path = work_dir.join("frame.png");

    scroll_input::warp_cursor(
        region.x + region.width / 2.0,
        region.y + region.height / 2.0,
    );
    // Let the warp land and the shrunken HUD window settle before frame one.
    std::thread::sleep(Duration::from_millis(200));

    let first = grab(region, &frame_path)?;
    // Fallback offset when correlation can't find one: the nominal scroll
    // distance converted from points to frame pixels along the scroll axis.
    let (axis_points, axis_px) = match direction {
        ScrollDirection::Up | ScrollDirection::Down => (region.height, first.height()),
        ScrollDirection::Left | ScrollDirection::Right => (region.width, first.width()),
    };
    let nominal_px = ((SCROLL_LINES as f64 * NOMINAL_POINTS_PER_LINE) * axis_px as f64
        / axis_points.max(1.0))
    .round() as u32;

    let mut prev_n = stitch::normalize(&first, direction);
    let mut composite = prev_n.clone();
    let mut frames = 1u32;
    progress(frames);

    while frames < MAX_FRAMES
        && composite.height() < MAX_COMPOSITE_PX
        && !stop.load(Ordering::Relaxed)
    {
        let step = scroll_input::post_scroll(direction, SCROLL_LINES)
            .map_err(CaptureError::Tool)
            .and_then(|()| {
                std::thread::sleep(SETTLE);
                grab(region, &frame_path)
            });
        let frame = match step {
            Ok(frame) => frame,
            // Keep the partial composite once it has real content.
            Err(err) if frames >= 2 => {
                eprintln!("scrolling capture step failed, keeping partial result: {err}");
                break;
            }
            Err(err) => return Err(err),
        };
        let frame_n = stitch::normalize(&frame, direction);
        let offset = match stitch::find_scroll_offset(&prev_n, &frame_n) {
            // No movement at all: the end of the scrollable content.
            Some(0) if stitch::frames_identical(&prev_n, &frame_n) => break,
            // Fixed header pinned the match at 0, or nothing matched: trust
            // the nominal scroll distance instead.
            Some(0) | None => nominal_px.clamp(1, frame_n.height().saturating_sub(1).max(1)),
            Some(offset) => offset,
        };
        composite = stitch::append_rows(composite, &frame_n, offset);
        prev_n = frame_n;
        frames += 1;
        progress(frames);
    }

    let _ = std::fs::remove_file(&frame_path);
    Ok(stitch::denormalize(composite, direction))
}

/// Silent region grab. Unlike interactive modes, -R always writes a file on
/// success, so a missing/broken file is an error, not a cancel.
fn grab(region: &ScrollRegion, dest: &Path) -> Result<RgbaImage, CaptureError> {
    let rect = format!(
        "{},{},{},{}",
        region.x, region.y, region.width, region.height
    );
    let output = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-t", "png", "-R", &rect])
        .arg(dest)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CaptureError::Tool(if stderr.is_empty() {
            format!("screencapture exited with {}", output.status)
        } else {
            stderr
        }));
    }
    image::open(dest)
        .map(|img| img.to_rgba8())
        .map_err(|err| CaptureError::Tool(format!("could not decode frame: {err}")))
}
```

In `src-tauri/src/capture/mod.rs`:

```rust
#[cfg(target_os = "macos")]
pub mod scrolling;
```

- [ ] **Step 2: Extract `publish_capture` in `src-tauri/src/commands.rs`**

Replace the `CaptureOutcome::Captured(path)` arm of `capture_and_publish` with a call to a new helper, so the scrolling path reuses it:

```rust
fn capture_and_publish(app: &AppHandle, mode: CaptureMode) -> Result<(), CaptureError> {
    // The overlay must not appear in the shot.
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
    let state = app.state::<AppState>();
    let dest = state.history.new_capture_path();
    match capture::capture(mode, &dest)? {
        CaptureOutcome::Cancelled => Ok(()),
        CaptureOutcome::Captured(path) => {
            publish_capture(app, &path);
            Ok(())
        }
    }
}

/// Prune history and announce a freshly written capture file.
fn publish_capture(app: &AppHandle, path: &std::path::Path) {
    let state = app.state::<AppState>();
    state.history.prune();
    let id = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    if let Some(entry) = state.history.resolve(&id) {
        let _ = app.emit("capture:new", &entry);
        show_overlay(app);
    }
}
```

- [ ] **Step 3: Add the scrolling commands to `src-tauri/src/commands.rs`**

The commands exist on every platform (single `generate_handler!` list); only the body is macOS-gated. Direction travels as a string so no macOS-only type leaks into the cross-platform signature.

```rust
/// Selection rect in scrollcap-window-local logical pixels (CSS px).
#[derive(serde::Deserialize, Clone, Copy)]
pub struct SelectionRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[tauri::command]
pub fn run_scrolling_capture(
    app: AppHandle,
    rect: SelectionRect,
    direction: String,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return run_scrolling_capture_macos(app, rect, direction);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, rect, direction);
        Err("scrolling capture is only available on macOS".into())
    }
}

#[tauri::command]
pub fn stop_scrolling_capture(state: State<AppState>) {
    state
        .scroll_stop
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(target_os = "macos")]
fn run_scrolling_capture_macos(
    app: AppHandle,
    rect: SelectionRect,
    direction: String,
) -> Result<(), String> {
    use crate::capture::scrolling::{self, ScrollRegion};
    use crate::capture::stitch::ScrollDirection;
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

    let direction = match direction.as_str() {
        "up" => ScrollDirection::Up,
        "down" => ScrollDirection::Down,
        "left" => ScrollDirection::Left,
        "right" => ScrollDirection::Right,
        other => return Err(format!("unknown scroll direction: {other}")),
    };
    let window = app
        .get_webview_window("scrollcap")
        .ok_or("scrollcap window is not open")?;

    // Window-local logical rect → global points. The window covers the whole
    // monitor, so window origin + local point = global point, in the same
    // logical space screencapture -R and CGDisplayBounds use.
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let origin = window
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let region = ScrollRegion {
        x: origin.x + rect.x,
        y: origin.y + rect.y,
        width: rect.width,
        height: rect.height,
    };

    if !crate::capture::scroll_input::ensure_accessibility() {
        let _ = window.destroy();
        app.dialog()
            .message(
                "Scrolling Capture needs Accessibility access to scroll the page for you.\n\n\
                 Enable \"Screen for me\" in System Settings → Privacy & Security → \
                 Accessibility, then try again.",
            )
            .title("Accessibility Permission Needed")
            .kind(MessageDialogKind::Warning)
            .show(|_| {});
        return Ok(());
    }

    // The overlay must not appear in frames either.
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }

    // Shrink the selection window to a Stop pill parked outside the rect so it
    // never appears in a frame. (If the rect spans the whole monitor there is
    // no outside; the pill may then overlap — accepted v1 limitation.)
    const PILL_W: f64 = 220.0;
    const PILL_H: f64 = 56.0;
    const GAP: f64 = 12.0;
    let monitor_bottom = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let s = m.scale_factor();
            m.position().to_logical::<f64>(s).y + m.size().to_logical::<f64>(s).height
        })
        .unwrap_or(f64::MAX);
    let below = region.y + region.height + GAP;
    let pill_y = if below + PILL_H <= monitor_bottom {
        below
    } else {
        (region.y - PILL_H - GAP).max(0.0)
    };
    let _ = window.set_size(tauri::LogicalSize::new(PILL_W, PILL_H));
    let _ = window.set_position(tauri::LogicalPosition::new(region.x, pill_y));
    let _ = app.emit_to("scrollcap", "scroll:running", ());

    let state = app.state::<AppState>();
    state
        .scroll_stop
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let stop = state.scroll_stop.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), CaptureError> {
            let work_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| CaptureError::Tool(e.to_string()))?
                .join("scroll-tmp");
            let image = scrolling::run(&region, direction, &stop, &work_dir, |frames| {
                let _ = app.emit_to("scrollcap", "scroll:progress", frames);
            })?;
            let state = app.state::<AppState>();
            let dest = state.history.new_capture_path();
            image
                .save(&dest)
                .map_err(|e| CaptureError::Tool(format!("could not save composite: {e}")))?;
            publish_capture(&app, &dest);
            Ok(())
        })();
        if let Some(window) = app.get_webview_window("scrollcap") {
            let _ = window.destroy();
        }
        if let Err(err) = result {
            eprintln!("scrolling capture failed: {err}");
            let _ = app.emit("capture:error", err.to_string());
        }
    });
    Ok(())
}
```

(`CaptureError` is already imported at the top of commands.rs.)

- [ ] **Step 4: Add `open_scrollcap` to `src-tauri/src/windows.rs`**

```rust
/// Transient full-screen selection window for scrolling capture, covering the
/// active monitor. Destroyed when the run finishes or is cancelled.
pub fn open_scrollcap(app: &AppHandle) -> tauri::Result<()> {
    if let Some(existing) = app.get_webview_window("scrollcap") {
        let _ = existing.destroy();
    }
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "scrollcap",
        tauri::WebviewUrl::App("scrollcap.html".into()),
    )
    .title("Scrolling Capture")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .accept_first_mouse(true)
    .focused(true);
    if let Some(monitor) = crate::commands::active_monitor(app) {
        let scale = monitor.scale_factor();
        let pos = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        builder = builder
            .position(pos.x, pos.y)
            .inner_size(size.width, size.height);
    }
    builder.build()?;
    Ok(())
}
```

- [ ] **Step 5: Tray item (macOS only)**

In `src-tauri/src/tray.rs`, after the `fullscreen` item:

```rust
    #[cfg(target_os = "macos")]
    let scrolling = MenuItem::with_id(app, "capture_scrolling", "Scrolling Capture", true, None::<&str>)?;
```

In the Vec built in Task 1 Step 5, insert between `&sep1` and `&self_timer`:

```rust
    #[cfg(target_os = "macos")]
    items.push(&scrolling);
```

In `on_menu_event`:

```rust
            "capture_scrolling" => {
                if let Err(err) = windows::open_scrollcap(app) {
                    eprintln!("failed to open scrolling capture: {err}");
                }
            }
```

- [ ] **Step 6: Register commands and capability**

`src-tauri/src/lib.rs` `generate_handler![]` — add:

```rust
            commands::run_scrolling_capture,
            commands::stop_scrolling_capture,
```

`src-tauri/capabilities/default.json` — add `"scrollcap"` to `windows`:

```json
  "windows": [
    "main",
    "overlay",
    "editor",
    "history",
    "timer",
    "scrollcap"
  ],
```

- [ ] **Step 7: Verify build + tests**

Run: `cd src-tauri && cargo test`
Expected: compiles; stitch tests + existing tests PASS.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "Add scrolling capture loop, commands, window, and tray item (macOS)"
```

---

### Task 6: Scrolling capture page (selection, direction HUD, stop pill)

**Files:**
- Create: `scrollcap.html` (repo root)
- Create: `src/scrollcap/geometry.ts` (pure, vitest-covered)
- Create: `src/scrollcap/geometry.test.ts`
- Create: `src/scrollcap/main.ts`
- Create: `src/scrollcap/scrollcap.css`
- Modify: `vite.config.ts` (add input)

**Interfaces:**
- Consumes: IPC `run_scrolling_capture({rect, direction})`, `stop_scrolling_capture()`; events `scroll:running`, `scroll:progress` (Task 5).
- Produces: nothing consumed by later tasks.

- [ ] **Step 1: Write the failing geometry tests**

Create `src/scrollcap/geometry.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { hudPosition, isSelectable, normalizeRect } from "./geometry";

describe("normalizeRect", () => {
  it("orders coordinates regardless of drag direction", () => {
    expect(normalizeRect(100, 80, 20, 200)).toEqual({
      x: 20,
      y: 80,
      width: 80,
      height: 120,
    });
  });
});

describe("isSelectable", () => {
  it("rejects rects under the minimum size", () => {
    expect(isSelectable({ x: 0, y: 0, width: 39, height: 400 })).toBe(false);
    expect(isSelectable({ x: 0, y: 0, width: 400, height: 39 })).toBe(false);
  });
  it("accepts rects at or over the minimum size", () => {
    expect(isSelectable({ x: 0, y: 0, width: 40, height: 40 })).toBe(true);
  });
});

describe("hudPosition", () => {
  const hud = { w: 260, h: 56 };
  it("sits below the rect when there is room", () => {
    const pos = hudPosition({ x: 100, y: 100, width: 300, height: 200 }, hud.w, hud.h, 1440, 900);
    expect(pos).toEqual({ x: 140, y: 312 });
  });
  it("flips above the rect near the bottom edge", () => {
    const pos = hudPosition({ x: 100, y: 600, width: 300, height: 260 }, hud.w, hud.h, 1440, 900);
    expect(pos.y).toBe(600 - 56 - 12);
  });
  it("clamps x inside the viewport", () => {
    const pos = hudPosition({ x: 1300, y: 100, width: 200, height: 100 }, hud.w, hud.h, 1440, 900);
    expect(pos.x).toBe(1440 - 260 - 12);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test`
Expected: FAIL — `./geometry` module not found.

- [ ] **Step 3: Create `src/scrollcap/geometry.ts`**

```ts
// Pure selection math, DOM-free so vitest covers it (same idea as the
// editor's geometry.ts).

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Selections smaller than this are accidental clicks, not scroll areas. */
export const MIN_SELECTION = 40;

export function normalizeRect(x1: number, y1: number, x2: number, y2: number): Rect {
  return {
    x: Math.min(x1, x2),
    y: Math.min(y1, y2),
    width: Math.abs(x2 - x1),
    height: Math.abs(y2 - y1),
  };
}

export function isSelectable(rect: Rect): boolean {
  return rect.width >= MIN_SELECTION && rect.height >= MIN_SELECTION;
}

/**
 * Place the HUD under the rect's bottom-right corner, flipping above when the
 * screen edge is too close, always clamped inside the viewport.
 */
export function hudPosition(
  rect: Rect,
  hudW: number,
  hudH: number,
  viewportW: number,
  viewportH: number,
): { x: number; y: number } {
  const gap = 12;
  let x = Math.min(rect.x + rect.width - hudW, viewportW - hudW - gap);
  x = Math.max(gap, x);
  let y = rect.y + rect.height + gap;
  if (y + hudH > viewportH - gap) {
    y = Math.max(gap, rect.y - hudH - gap);
  }
  return { x, y };
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test`
Expected: geometry tests PASS (existing suites still green). Note: `hudPosition` "sits below" expectation is `x: 140` because `rect.x + width - hudW = 100 + 300 - 260 = 140` and `y: 312 = 100 + 200 + 12`.

- [ ] **Step 5: Create `scrollcap.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Scrolling Capture</title>
    <link rel="stylesheet" href="/src/scrollcap/scrollcap.css" />
    <script type="module" src="/src/scrollcap/main.ts" defer></script>
  </head>
  <body>
    <div id="hint" class="hint">Drag to select the area to scroll-capture — Esc to cancel</div>
    <div id="selection" class="selection hidden"></div>
    <div id="hud" class="hud hidden">
      <div class="directions">
        <button data-direction="up" title="Scroll up">↑</button>
        <button data-direction="down" class="active" title="Scroll down">↓</button>
        <button data-direction="left" title="Scroll left">←</button>
        <button data-direction="right" title="Scroll right">→</button>
      </div>
      <button id="start" class="primary">Start</button>
      <button id="cancel">Cancel</button>
    </div>
    <div id="pill" class="pill hidden">
      <span id="progress">Capturing…</span>
      <button id="stop" class="primary">Stop</button>
    </div>
  </body>
</html>
```

- [ ] **Step 6: Create `src/scrollcap/scrollcap.css`**

```css
html,
body {
  margin: 0;
  height: 100%;
  overflow: hidden;
  user-select: none;
  -webkit-user-select: none;
  cursor: crosshair;
  background: rgba(0, 0, 0, 0.12);
  font: 13px -apple-system, system-ui, sans-serif;
}

.hidden {
  display: none !important;
}

.hint {
  position: fixed;
  top: 24px;
  left: 50%;
  transform: translateX(-50%);
  padding: 6px 14px;
  border-radius: 999px;
  background: rgba(24, 24, 26, 0.85);
  color: rgba(255, 255, 255, 0.85);
  pointer-events: none;
}

.selection {
  position: fixed;
  border: 1px solid #4a9eff;
  /* Punches a "hole": everything outside the rect gets the extra dim. */
  box-shadow: 0 0 0 100000px rgba(0, 0, 0, 0.25);
}

.hud,
.pill {
  position: fixed;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px;
  border-radius: 10px;
  background: rgba(24, 24, 26, 0.92);
  color: #fff;
  cursor: default;
}

.directions {
  display: flex;
  gap: 4px;
}

.hud button,
.pill button {
  font: inherit;
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
  border: none;
  border-radius: 6px;
  padding: 6px 10px;
  cursor: pointer;
}

.directions button.active {
  background: #4a9eff;
}

button.primary {
  background: #4a9eff;
}

/* Pill mode: Rust has shrunk the window to just the pill. */
body.running {
  background: transparent;
  cursor: default;
}

body.running .pill {
  display: flex !important;
  inset: 0;
  border-radius: 12px;
  justify-content: space-between;
}
```

- [ ] **Step 7: Create `src/scrollcap/main.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hudPosition, isSelectable, normalizeRect, type Rect } from "./geometry";

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const appWindow = getCurrentWindow();
const hint = el<HTMLDivElement>("hint");
const selection = el<HTMLDivElement>("selection");
const hud = el<HTMLDivElement>("hud");
const pill = el<HTMLDivElement>("pill");
const progress = el<HTMLSpanElement>("progress");

type Phase = "select" | "staged" | "running";
let phase: Phase = "select";
let dragStart: { x: number; y: number } | null = null;
let rect: Rect | null = null;
let direction = "down";

function renderSelection(r: Rect) {
  selection.style.left = `${r.x}px`;
  selection.style.top = `${r.y}px`;
  selection.style.width = `${r.width}px`;
  selection.style.height = `${r.height}px`;
  selection.classList.remove("hidden");
}

function showHud(r: Rect) {
  hud.classList.remove("hidden");
  const { x, y } = hudPosition(
    r,
    hud.offsetWidth,
    hud.offsetHeight,
    window.innerWidth,
    window.innerHeight,
  );
  hud.style.left = `${x}px`;
  hud.style.top = `${y}px`;
}

function resetToSelect() {
  phase = "select";
  rect = null;
  selection.classList.add("hidden");
  hud.classList.add("hidden");
  hint.classList.remove("hidden");
}

document.addEventListener("mousedown", (event) => {
  if (phase === "running" || (event.target as HTMLElement).closest(".hud, .pill")) return;
  phase = "select";
  hud.classList.add("hidden");
  hint.classList.add("hidden");
  dragStart = { x: event.clientX, y: event.clientY };
});

document.addEventListener("mousemove", (event) => {
  if (!dragStart) return;
  renderSelection(normalizeRect(dragStart.x, dragStart.y, event.clientX, event.clientY));
});

document.addEventListener("mouseup", (event) => {
  if (!dragStart) return;
  const candidate = normalizeRect(dragStart.x, dragStart.y, event.clientX, event.clientY);
  dragStart = null;
  if (!isSelectable(candidate)) {
    resetToSelect();
    return;
  }
  rect = candidate;
  phase = "staged";
  renderSelection(candidate);
  showHud(candidate);
});

for (const button of hud.querySelectorAll<HTMLButtonElement>(".directions button")) {
  button.addEventListener("click", () => {
    direction = button.dataset.direction ?? "down";
    hud.querySelectorAll(".directions button").forEach((b) => b.classList.remove("active"));
    button.classList.add("active");
  });
}

el<HTMLButtonElement>("start").addEventListener("click", () => {
  if (phase !== "staged" || !rect) return;
  phase = "running";
  // Rust shrinks this window to the pill; swap the page to pill-only mode.
  selection.classList.add("hidden");
  hud.classList.add("hidden");
  document.body.classList.add("running");
  void invoke("run_scrolling_capture", { rect, direction }).catch((err) => {
    progress.textContent = String(err);
  });
});

el<HTMLButtonElement>("cancel").addEventListener("click", () => void appWindow.close());
el<HTMLButtonElement>("stop").addEventListener("click", () => void invoke("stop_scrolling_capture"));

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (phase === "running") {
    void invoke("stop_scrolling_capture");
  } else {
    void appWindow.close();
  }
});

void listen<number>("scroll:progress", (event) => {
  progress.textContent = `${event.payload} frames`;
});
```

- [ ] **Step 8: Register the Vite input**

In `vite.config.ts`:

```ts
      input: {
        main: "index.html",
        overlay: "overlay.html",
        editor: "editor.html",
        history: "history.html",
        timer: "timer.html",
        scrollcap: "scrollcap.html",
      },
```

- [ ] **Step 9: Verify build + tests**

Run: `npm run build && npm test`
Expected: build succeeds with the `scrollcap` input; all vitest suites PASS.

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "Add scrolling capture page: selection, direction HUD, stop pill"
```

---

### Task 7: End-to-end verification and docs

**Files:**
- Modify: `CLAUDE.md` (new gotchas)
- No code changes expected unless verification finds bugs.

**Interfaces:**
- Consumes: everything above.
- Produces: verified feature + updated docs.

- [ ] **Step 1: Run all gates**

Run: `npm run build && npm test && (cd src-tauri && cargo test)`
Expected: all PASS.

- [ ] **Step 2: Manual verification (dev build)**

Run `npm run tauri dev` (check `pgrep -fl screenforme` first — kill any stale `/Applications/Screen for me.app` instance per CLAUDE.md). Then verify each:

1. Self-Timer 3 s: countdown appears centered, fires, fullscreen capture shows in overlay.
2. Self-Timer cancel: click the disc mid-count → window closes, no capture. Esc also cancels.
3. Self-Timer restart: start a 10 s timer, then immediately start a 3 s one → only one countdown window, 3 s wins.
4. Scrolling Capture ↓ on a long web page: select an area inside the browser viewport, Start → Accessibility prompt on first run (grant to the terminal in dev); re-run → page auto-scrolls, pill shows frame count, result is one tall stitched PNG in the overlay/editor with no duplicated or missing bands at seams.
5. Scrolling Capture auto-stop: let it reach the page bottom → capture ends on its own.
6. Stop button and Esc during a run → partial capture is saved.
7. Esc before Start → window closes, nothing captured.
8. Directions ↑ ← → each produce a correctly extended image (e.g. ← / → on a wide horizontally scrollable page).
9. Linux-facing sanity: `capture_scrolling` tray item is cfg-gated; Self-Timer path builds without macOS-only code (confirmed by the cfg gates; actual Linux run stays untested per CLAUDE.md).

- [ ] **Step 3: Update CLAUDE.md**

Add to the Gotchas section:

```markdown
- **macOS Accessibility permission (Scrolling Capture)**: posting synthetic scroll
  events needs Accessibility (separate from Screen Recording). The first run
  prompts and registers the app; in dev the grant attaches to the *terminal*.
  Without it the capture aborts with an explanatory dialog.
```

Add to the Architecture section (after the editor-window bullet):

```markdown
- `timer` and `scrollcap` are transient windows: created on demand
  (`windows.rs::open_timer` / `open_scrollcap`), **destroyed** on close — they are
  deliberately NOT in the hide-instead-of-close list in lib.rs. Scrolling capture
  (macOS only): `capture/scrolling.rs` loops `screencapture -x -R` grabs with
  CGEvent line-scroll steps and stitches via the pure `capture/stitch.rs`
  (correlation-based offset detection; unit-tested with synthetic noise images).
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "Verify scrolling capture + self-timer; document new gotchas"
```
