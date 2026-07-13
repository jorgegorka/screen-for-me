# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Screen for me** — a CleanShot-style desktop screenshot app built with **Tauri v2** (Rust backend, TypeScript + Vite frontend, Konva canvas editor). v1 scope: area/window/fullscreen capture, a bottom-left quick-access overlay, an in-app annotation editor, drag-out, copy/save. **No video capture.** Targets: macOS (primary) and Linux; Windows later.

History: the repo started as a Native SDK (vercel-labs/native, Zig) app and was rewritten on Tauri because that SDK lacked screen capture, mouse-coordinate events, global hotkeys, and drag-out (see `docs/` plan history in git).

## Commands

```bash
npm run tauri dev        # run the app (Vite + cargo, hot reload both sides)
npm run tauri build      # release bundles (.app/.dmg on macOS)
npm run build            # tsc + vite build (frontend type-check)
npm test                 # vitest (pure editor modules)
cd src-tauri && cargo test   # Rust unit tests (capture validation, history)
npm run tauri icon assets/icon.png   # regenerate src-tauri/icons from source icon
```

Before calling a change done, run `npm run build`, `npm test`, and `cargo test`.

## Architecture

The app is a **menu-bar utility** (macOS ActivationPolicy::Accessory — no Dock icon). Everything starts from the tray menu or global shortcuts (Cmd/Ctrl+Shift+7/8/9 = area/window/fullscreen, defined in `src-tauri/src/shortcuts.rs`).

Capture flow: shortcut/tray → `commands::trigger_capture` (spawn_blocking) → `capture::capture(mode, dest)` (per-OS backend) → PNG in app-data `captures/` → `History::prune` → emit `capture:new` → overlay window positioned bottom-left of the primary monitor and shown.

- `src-tauri/src/capture/` — `CaptureBackend` per OS. macOS spawns `/usr/sbin/screencapture` (`-i` interactive crosshair). Linux uses the xdg-desktop-portal Screenshot API via `ashpd` (untested on this dev machine — verify on Linux before release). `validate_output` treats a missing file as user-cancelled and a <1 KiB file as a permission problem.
- `src-tauri/src/history.rs` — capture files named `capture-<unix-ms>.png`; ids are bare file names (path traversal rejected in `resolve`).
- `src-tauri/src/commands.rs` — all IPC commands + overlay positioning + `ExportAction` (copy/save_to/overwrite) for editor exports (base64 PNG over IPC).
- `src/overlay/` — quick-access panel (transparent, always-on-top window declared in `tauri.conf.json`); listens for `capture:new`; drag-out starts a native drag via `@crabnebula/tauri-plugin-drag` after a 5px move threshold.
- `src/editor/` — Konva editor. Stage is kept in **image coordinates** with `stage.scale(fitScale)`; export uses `pixelRatio: 1/scale` for native resolution. Undo/redo is a snapshot stack (`history.ts`) over a whitelisted-attrs serialization (`shapes.ts` — new shape types must be added to `ATTRS` or they won't survive undo). Pixelate bakes a data-URL into the node attr so undo can rehydrate it. `geometry.ts`/`history.ts` are deliberately Konva-free for vitest.
- Editor window is created once by `open_editor`, then **hidden** (not destroyed) on Done / close and re-shown on later opens (`on_window_event` in lib.rs handles `main` + `editor`). The target capture is set in `AppState.editor_target`; the editor **pulls** it via the `editor_target` command on every (re)load, and also listens for `editor:load` for reuse while already open. This pull model avoids the earlier bug where reopening showed a blank canvas (an `editor:load` event racing a torn-down/not-yet-ready webview).

## Gotchas

- **macOS Screen Recording permission**: without it, `screencapture` can exit 0 and write a wallpaper-only image. In dev, the TCC grant attaches to the *terminal* running the app; the packaged .app prompts once itself.
- Transparent overlay window requires `macOSPrivateApi: true` (tauri.conf.json) + the `macos-private-api` cargo feature.
- Capability file `src-tauri/capabilities/default.json` must list every window label (`main`, `overlay`, `editor`, `history`) — a new window with JS API calls needs its label added there.
- Windows that hide-instead-of-close (`main`, `editor`, `history`) are handled in the `on_window_event` match in lib.rs; `src-tauri/src/windows.rs` owns their open/show helpers plus the About and Check-for-Updates dialogs.
- The asset protocol scope is `$APPDATA/captures/*`; captures displayed in webviews go through `convertFileSrc`.
- macOS reserves Cmd+Shift+3/4/5, hence 7/8/9.
- Accessory activation policy means no app menu bar; don't rely on menu-role shortcuts (Cmd+C in webviews) — handle keys in JS.

## Updates

The tray's "Check for Updates…" uses `tauri-plugin-updater` (config under `plugins.updater` in tauri.conf.json). **The endpoint (`releases.screenforme.example`) and signing key are placeholders** — until a real release pipeline exists, a check fails gracefully with a "couldn't reach the update server" dialog. To make it real: (1) host update manifests at a real `endpoints` URL (Tauri static-JSON or dynamic format); (2) replace `plugins.updater.pubkey` with the public key whose **private** key you sign releases with (`npm run tauri signer generate`); (3) build with `TAURI_SIGNING_PRIVATE_KEY`/`_PASSWORD` set and `createUpdaterArtifacts: true` (already on). The dev keypair generated during scaffolding lives outside the repo (session scratchpad) and is throwaway — generate a real one for production and never commit the private key.
