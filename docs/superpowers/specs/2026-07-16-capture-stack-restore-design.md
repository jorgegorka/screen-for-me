# Capture Stack Overlay + Restore from History

**Date:** 2026-07-16
**Status:** Approved

## Goal

Two changes that turn the quick-access overlay into a stack of capture panels:

1. **History window**: each capture card has exactly two buttons — **Copy**
   (unchanged) and **Restore**. Restore reopens the overlay panel with that
   image. The History window stays open after Restore.
2. **Overlay**: instead of one card showing the latest capture, the overlay
   holds an ordered **stack of panels** (newest on top, growing upward from
   the bottom corner). Both a fresh capture and a Restore push a panel onto
   the stack.

## Behaviour

- **New capture** → pushes a panel on top of any panels already open.
- **Restore** → pushes a panel on top; if that image is already in the stack,
  the existing panel **moves to the top** instead (no duplicates). No
  clipboard side effects.
- **Dismiss**: each panel has its own ×, removing just that panel. When the
  last panel is dismissed, the overlay window hides and the stack clears.
- **Auto-close** (`auto_close_enabled` etc.): per panel. Each panel has its
  own timer started when it appears; hovering a panel pauses that panel's
  timer; `save_and_close` saves that panel's capture before removing it.
  Removal by timer behaves like its × (window hides when the stack empties).
- **Drag-out**: works per panel (drag data is the entry path).
  `close_after_drag` removes the dragged panel only.
- **Stack cap**: the stack is clamped so the window fits the monitor's
  height; when full, the oldest (bottom) panels are dropped.
- **"+N" badge**: removed — the stack itself shows what's open.
- **Persistence**: the stack lives in the overlay webview's memory. The
  window is hidden, never destroyed, so the stack survives hide/show —
  including the temporary hide during capture (`trigger_capture`), after
  which the new shot lands on top of the surviving stack. The stack does not
  survive app restart.

## Architecture

Single overlay window that grows into a column of panels (chosen over
one-window-per-panel: the existing `overlay` window already provides
transparency, always-on-top, content protection, drag-out, and the
follow-the-cursor loop; multiple windows would multiply Rust window
management for the same visual result).

### Rust (`src-tauri/src/commands.rs`)

- **`restore_capture(id)`** (new command): resolves via `History::resolve`
  (path traversal already rejected there), emits a new **`capture:restore`**
  event with the `CaptureEntry`, then calls `show_overlay`. Unknown id → Err.
- **`set_overlay_panels(count) -> usize`** (new command): called by the
  overlay webview whenever the stack size changes. Computes the panel height
  (`OVERLAY_BASE_HEIGHT × overlay_size`), clamps `count` so the total stack
  height fits the current monitor's logical height (minus margins), resizes
  the window to `count × panel height` (bottom-anchored, so it grows upward
  via re-position), and returns the clamped count. The webview trims its
  stack to the returned value, dropping the bottom-most panels.
- `show_overlay` / `place_overlay` account for the current panel count when
  sizing/positioning (single owner of window size stays the backend).
- The height-clamp / origin math is a pure function with a Rust unit test.

### Overlay frontend (`src/overlay/`)

- Stack state and operations (push, move-to-top, remove, trim-to) live in a
  new pure module (no Konva/Tauri imports) so vitest covers the ordering
  logic.
- `main.ts` renders the stack as a column of cards, newest on top. Each card
  carries the full current action set: thumbnail with drag-out, Copy, Save…,
  Reveal, Annotate, ×, its own auto-close timer.
- Listens for `capture:new` (push) and `capture:restore` (push or
  move-to-top). On every stack change: `set_overlay_panels(stack.length)`,
  then trim to the returned count. On empty: hide the window.
- The catch-up path on `DOMContentLoaded` (`list_captures`) seeds a
  single-panel stack with the latest capture, as today.

### History frontend (`src/history/main.ts`)

- Card actions reduced to **Copy** + **Restore**. Restore invokes
  `restore_capture(id)`; the window is not hidden.

### i18n

- New key `history.restore` added to all five catalogs
  (`locales/{en-GB,es,fr,de,it}.json`).
- Unused keys `history.annotate`, `history.save`, `history.reveal` removed
  from all five (they are only referenced by the history window; parity
  tests stay green).

## Error handling

- `restore_capture` with a missing/pruned id returns an error; the history
  card surfaces nothing special (the file list re-renders on `capture:new`
  only, so a stale card is possible after external deletion — matches
  current behaviour of Copy).
- `set_overlay_panels` failures (no window/monitor) are non-fatal: the
  webview keeps its stack; sizing falls back to whatever the window has.

## Testing

- **Rust**: unit tests for the clamp/height math in `commands.rs`.
- **Vitest**: unit tests for the pure stack module (push, restore-existing
  moves to top, per-panel remove, trim drops oldest).
- Gate: `npm run build`, `npm test`, `cargo test` all pass.

## Out of scope

- Persisting the stack across app restarts.
- Any change to capture flow, clipboard auto-copy, or editor behaviour.
- Windows platform work.
