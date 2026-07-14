---
name: verify
description: Runtime-verify editor/UI changes in the real app — build, launch, drive the GUI with CGEvents, capture window screenshots by ID, and check exported PNGs
---

# Verifying Screen for me changes in the running app

The app is a macOS menu-bar (Accessory) Tauri app — no Dock icon, everything
starts from the tray, so the editor can't be reached by clicking around.
Recipe that works:

## Launch with a target capture

1. Generate a test capture directly into
   `~/Library/Application Support/com.screenforme.app/captures/capture-<unix-ms>.png`
   (Pillow in a scratch venv works; PNG only — `validate_output` treats <1 KiB as broken).
2. Add a **temporary** `#[cfg(debug_assertions)]` block in `lib.rs` `setup()` that
   sleeps ~2s then calls `commands::open_editor(handle, state, "<capture-id>")`
   via `run_on_main_thread`. Remove it before reporting; delete the test capture after.
3. `npm run tauri dev` in the background; wait for `pgrep -f target/debug/screenforme`.

## Drive and observe

- Screenshot a single window without focus: find the window ID with a swift
  one-liner over `CGWindowListCopyWindowInfo`, then `screencapture -x -o -l <id>`.
- Synthetic input: CGEvent swift scripts (click/drag/scroll). **Before every
  event batch, check the topmost layer-0 window at each target point** — the
  app's windows sit behind whatever Jorge is using, and events go to the top
  window (once they landed in his browser/terminal). Raise first:
  `osascript … set frontmost of (first process whose unix id is <pid>) to true`.
  `AXRaise` fails for this accessory app; frontmost + a click works.
- Don't send keyboard shortcuts by ANSI keycode — the keyboard layout may not
  be US; click the toolbar buttons instead.
- Editor window is 1200×800 by default at (264,104) here; toolbar buttons move —
  measure from a fresh screenshot (screenshots are Retina 2x; `sips -Z 1400`
  then multiply coords by 1200/1400).
- Verify exports by reading the overwritten PNG from the captures dir (Pillow
  pixel probes for dimensions + annotation pixels at expected image coords).

## Gotchas

- WKWebView kills its WebContent process (window goes solid white) if any
  canvas exceeds ~32767px in one dimension — the editor virtualizes the Konva
  stage for this reason; watch for white windows as a crash signal.
- Konva `batchDraw` is rAF-batched: screenshot immediately after an action can
  race the draw; sleep ~0.5s.
- A fallback when driving the GUI is unsafe (user active): Playwright WebKit
  harness mirroring `src/editor/main.ts` view code (see memory
  `verify-editor-in-webkit-harness`), asserting via `getImageData` probes.
