# Scrolling Capture & Self-Timer — Design

**Date:** 2026-07-13
**Status:** Approved

Two new capture modes for Screen for me, modeled on CleanShot: **Scrolling Capture**
(auto-scroll a selected region and stitch the frames into one tall/wide image) and
**Self-Timer** (capture the full screen after a chosen delay).

## Goals

- Scrolling Capture: capture content longer than the screen (mostly web pages) by
  auto-scrolling in a user-selected direction (up / down / left / right) and
  stitching frames into a single PNG that lands in the normal capture pipeline.
- Self-Timer: capture the full screen after a 3 / 5 / 10 second countdown, with a
  visible, cancellable countdown.
- Both results are ordinary captures: saved to `captures/`, pruned by `History`,
  announced via `capture:new`, shown in the quick-access overlay, editable.

## Non-goals (v1)

- Scrolling Capture on Linux/Windows (macOS only; tray item not added elsewhere).
- Handling fixed/sticky page headers perfectly — minor seams accepted.
- Custom timer durations or a global shortcut for either feature.
- Video or animated output.

## Tray menu

Two new entries after "Capture Fullscreen", before the History separator:

- **Scrolling Capture** (`capture_scrolling`) — macOS only (`#[cfg(target_os = "macos")]`).
- **Self-Timer** — a `Submenu` with items **3 seconds / 5 seconds / 10 seconds**
  (`timer_3`, `timer_5`, `timer_10`). Both platforms.

## Feature 1 — Self-Timer

### Flow

1. Tray preset clicked → `commands::start_timed_capture(app, secs)`.
2. A small transparent, always-on-top, non-resizable countdown window
   (label `timer`, page `timer.html`, `src/timer/`) is created on demand (destroyed
   when finished), centered on the active monitor (reuse `cursor_point` +
   `monitor_from_point` logic, falling back to primary).
3. The page receives the duration (pull model: a `timer_duration` command it calls
   on load — same pattern as `editor_target`, avoiding event races) and renders a
   large countdown number, ticking locally with `setInterval`.
4. **Cancel:** clicking the window or pressing Esc closes the window; no capture.
5. **Fire:** at zero the page invokes a `timed_capture_fire` command, which
   destroys the timer window, sleeps ~150 ms so it cannot appear in the shot, then
   calls the existing `capture_and_publish(app, CaptureMode::Fullscreen)` path on
   `spawn_blocking`.

### Notes

- Works on macOS and Linux — it is just a delayed fullscreen capture.
- Starting a new timer while one is running replaces it (destroy old window first).
- Capability file gains the `timer` label; Vite gains a `timer.html` entry.

## Feature 2 — Scrolling Capture (macOS only)

### UX flow

1. Tray item → `start_scrolling_capture` creates a full-screen, transparent,
   always-on-top window (label `scrollcap`, page `scrollcap.html`, `src/scrollcap/`)
   sized to the active monitor. Backdrop dimmed; crosshair drag-to-select rect.
   Esc at any point before Start cancels (window destroyed).
2. After the rect is drawn, a HUD panel appears beside it: four direction buttons
   (↑ ↓ ← →, **↓ preselected**), **Start**, **Cancel**. The rect can be redrawn.
3. On Start the page sends `{rect (logical, global coords), direction}` to the
   `run_scrolling_capture` command. The window is resized/repositioned by Rust to a
   small "Stop" pill placed outside the rect (so it never appears in frames) showing
   frame progress. **Stop** (or Esc) ends the run early and keeps what was stitched.

The rect the page reports is in window-local logical coordinates; Rust converts to
global screen points using the window's outer position (the window spans the whole
monitor, so window origin + local point = global point).

### Permissions

Synthetic scroll events require **Accessibility** (in addition to the existing
Screen Recording grant). Before the loop, call `AXIsProcessTrustedWithOptions`
with the prompt option (via `extern "C"` to ApplicationServices). If untrusted:
show a dialog explaining System Settings → Privacy & Security → Accessibility,
destroy the scrollcap window, abort. Same dev-vs-packaged TCC caveat as Screen
Recording applies (grant attaches to the terminal in dev).

### Capture loop (`src-tauri/src/capture/scrolling.rs`, on `spawn_blocking`)

1. Warp the cursor to the rect center (CoreGraphics) so scroll events route to the
   window under the rect.
2. Repeat (hard caps: ~40 frames, composite dimension ≤ 20,000 px):
   a. Grab the region silently: `/usr/sbin/screencapture -x -R x,y,w,h <tmp>` —
      frames land in a temp dir inside app-data, cleaned up afterwards.
   b. If the new frame is pixel-identical to the previous one → end of scrollable
      content → stop.
   c. Stitch (below), then post one scroll-wheel `CGEvent` in the chosen direction
      using **line units** (avoids trackpad-style inertia), then sleep ~350 ms for
      smooth-scroll settle.
   d. Check the stop flag (`AtomicBool` in `AppState`, set by a
      `stop_scrolling_capture` command wired to the Stop pill / Esc).
3. Save the composite PNG to `History::new_capture_path()`, prune, emit
   `capture:new`, show the overlay — identical publish path to other modes.

### Stitching

New macOS-only dependency: `image` crate (PNG codec only). Pure functions, unit
tested:

- **Offset detection:** browsers smooth-scroll and rubber-band, so the nominal
  scroll amount cannot be trusted. Take a reference strip from the new frame's
  leading edge (top rows for ↓, etc.) and slide it along the scroll axis over the
  previous frame, minimizing sum-of-absolute-differences, to find the true pixel
  offset. If no confident match (e.g. fixed headers dominating), fall back to the
  nominal scroll distance in pixels.
- **Append:** only the non-overlapping region of each new frame is appended to the
  composite (rows for vertical, columns for horizontal).
- Frames are at physical (Retina) resolution; all stitching math is in pixels, so
  no scale-factor conversions are needed. The `-R` rect is in logical points.

### Error handling

- Accessibility missing → explanatory dialog, clean abort.
- A grab or stitch failure mid-run: if ≥ 2 frames were stitched, save the
  composite so far (partial result beats nothing); otherwise emit the existing
  `capture:error` event.
- Cancelled before Start → no file, no event (like an Esc'd area capture).

## Plumbing checklist

- `src-tauri/capabilities/default.json`: add `timer` and `scrollcap` labels.
- `vite.config.ts`: add `timer.html` and `scrollcap.html` inputs; new pages under
  `src/timer/` and `src/scrollcap/`.
- Both new windows are created on demand and **destroyed** (not hidden) when done —
  they are transient, unlike `main`/`editor`/`history`.
- Tray wiring in `tray.rs`; new commands registered in `lib.rs`.
- `Cargo.toml`: `image` under the macOS target dependencies.

## Testing

- Rust unit tests for the pure stitching module: offset detection on synthetic
  images (exact scroll, smooth-scroll fractional offset, no-match fallback),
  identical-frame end detection, append geometry for all four directions, caps.
- Vitest for any pure TS selection-rect math kept Konva/DOM-free.
- Gates before done: `npm run build`, `npm test`, `cd src-tauri && cargo test`.
- Manual verification on this machine: scrolling capture of a long web page in all
  four directions; self-timer cancel and fire paths.
