# Auto-copy new captures to the clipboard

**Date:** 2026-07-15
**Status:** Approved

## Goal

Many users capture and immediately Cmd+V into the destination app. After every
successful capture, put the PNG on the system clipboard automatically,
controlled by a user setting that defaults to ON.

## Design

### Setting

- Add `copy_to_clipboard: bool` to `Settings` (`src-tauri/src/settings.rs`)
  with a serde default of `true`, so existing `settings.json` files without the
  field resolve to ON. No sanitisation/clamping needed.

### Behavior

- In `commands.rs::publish_capture` — the single choke point every successful
  capture goes through (area/window/fullscreen, timer, scrolling capture) —
  when the setting is on: read the capture file's bytes and call the existing
  `copy_png_to_clipboard`.
- A clipboard failure is logged to stderr and never blocks emitting
  `capture:new` or showing the overlay.
- Editor `Overwrite` exports (annotated image saved back over a capture) do
  **not** auto-copy; the editor already has an explicit Copy action. Auto-copy
  applies only to fresh captures.

### Settings UI

- New checkbox "Copy new captures to the clipboard" in the Settings window
  (`src/settings/`), wired exactly like the existing boolean settings
  (e.g. `close_after_drag`).
- New i18n key in all five locale catalogs (`locales/*.json`); key parity is
  already unit-tested on both sides.

### Testing

- Rust: settings deserialisation test — a settings JSON without the new field
  defaults to `true`; round-trip keeps an explicit `false`.
- Existing i18n parity tests cover the new key.
- Clipboard write itself is exercised manually (needs a real pasteboard).
