# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Screen for me** — a desktop screenshot app built with **Tauri v2** (Rust backend, TypeScript + Vite frontend, Konva canvas editor). v1 scope: area/window/fullscreen capture, a bottom-left quick-access overlay, an in-app annotation editor, drag-out, copy/save. **No video capture.** Targets: macOS (primary) and Linux; Windows later.

History: the repo started as a Native SDK (vercel-labs/native, Zig) app and was rewritten on Tauri because that SDK lacked screen capture, mouse-coordinate events, global hotkeys, and drag-out (see `docs/` plan history in git).

## Commands

```bash
npm run tauri dev        # run the app (Vite + cargo, hot reload both sides)
npm run tauri build      # release bundles (.app/.dmg on macOS)
npm run bundle           # tauri build + rename artifacts to underscores (Screen_for_me_*.dmg)
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
- `src/editor/` — Konva editor. **Load the background from raw bytes via `read_capture_bytes` → `blob:` URL, never `asset://`/`convertFileSrc`** — an asset-protocol image taints the canvas and makes `toDataURL()` throw/return empty, silently breaking export. `renderPng` retries at lower resolution and validates output; `export_png` rejects any non-PNG bytes so a bad export can never overwrite (zero out) a capture. Last-used tool/color/stroke persist to `editor_prefs.json` (`get/set_editor_prefs`, separate from overlay Settings so the Settings window can't clobber them) and are restored by `applyPrefs` on open. Stage is kept in **image coordinates** with `stage.scale(fitScale)`; export uses `pixelRatio: 1/scale` for native resolution. Undo/redo is a snapshot stack (`history.ts`) over a whitelisted-attrs serialization (`shapes.ts` — new shape types must be added to `ATTRS` or they won't survive undo). Pixelate bakes a data-URL into the node attr so undo can rehydrate it. `geometry.ts`/`history.ts` are deliberately Konva-free for vitest.
- Editor window is created once by `open_editor`, then **hidden** (not destroyed) on Done / close and re-shown on later opens (`on_window_event` in lib.rs handles `main` + `editor`). The target capture is set in `AppState.editor_target`; the editor **pulls** it via the `editor_target` command on every (re)load, and also listens for `editor:load` for reuse while already open. This pull model avoids the earlier bug where reopening showed a blank canvas (an `editor:load` event racing a torn-down/not-yet-ready webview).
- **i18n**: all user-facing strings live in flat JSON catalogs `locales/{en-GB,es,fr,de,it}.json` (en-GB is the fallback; keys must exist in every file — parity is unit-tested on both sides). Rust embeds them via `include_str!` in `src-tauri/src/i18n.rs` (`t`/`t_with`, resolved language in a global `RwLock`; `"system"` resolves via `sys-locale`). Frontend loads the same files via `import.meta.glob` in `src/shared/i18n.ts` (`t`/`tn`, `applyTranslations` swaps `data-i18n`/`data-i18n-title`/`data-i18n-alt` annotated HTML; `initI18n()` runs first in every window and re-applies on `settings:changed`). The language lives in `settings.language` (`"system"` default); changing it live rebuilds the tray (`tray::refresh`) and retitles open windows in `set_settings`. Never localise menu-item IDs, accelerators, or settings enum values. New user-visible strings go in all five catalogs, never inline.
- `timer` and `scrollcap` are transient windows: created on demand
  (`windows.rs::open_timer` / `open_scrollcap`), **destroyed** on close — they are
  deliberately NOT in the hide-instead-of-close list in lib.rs. Scrolling capture
  (macOS only): `capture/scrolling.rs` loops `screencapture -x -R` grabs with
  CGEvent line-scroll steps and stitches via the pure `capture/stitch.rs`
  (correlation-based offset detection; unit-tested with synthetic noise images).

## Gotchas

- **Stale installed app vs dev build**: `npm run tauri build` installs nothing, but if a `.app` was copied to `/Applications`, launching it runs that *frozen* binary — its tray menu reflects whenever it was packaged, not the current source. Two instances (installed + `npm run tauri dev`) also both register the global shortcuts and both show a tray icon. When the menu looks out of date, check `pgrep -fl screenforme` for an `/Applications/Screen for me.app` process and rebuild/reinstall the bundle.

- **macOS Screen Recording permission**: without it, `screencapture` can exit 0 and write a wallpaper-only image. In dev, the TCC grant attaches to the *terminal* running the app; the packaged .app prompts once itself.
- Transparent overlay window requires `macOSPrivateApi: true` (tauri.conf.json) + the `macos-private-api` cargo feature.
- Capability file `src-tauri/capabilities/default.json` applies to `"windows": ["*"]`, so new windows get JS API permissions automatically; split the capability only if a window ever needs a narrower set.
- Windows that hide-instead-of-close are listed in `windows.rs::HIDE_ON_CLOSE` (consumed by the `on_window_event` handler in lib.rs); `src-tauri/src/windows.rs` owns their open/show helpers plus the About and Check-for-Updates dialogs.
- The asset protocol scope is `$APPDATA/captures/*`; captures displayed in webviews go through `convertFileSrc`.
- macOS reserves Cmd+Shift+3/4/5, hence 7/8/9.
- **Cursor→monitor on macOS**: don't use `AppHandle::cursor_position()` for active-screen detection — tao (0.35.x) returns it in physical pixels scaled by the *primary* monitor and mixes units in the Y-flip, so on scaled/Retina displays the point misses every monitor and `monitor_from_point` returns `None` (silently falling back to primary). `commands.rs::cursor_point` reads the cursor from CoreGraphics (`CGEvent::location`), which is in the same logical-point space as `CGDisplayBounds`/`monitor_from_point`. Non-macOS falls back to `cursor_position()`.
- Accessory activation policy means no app menu bar; don't rely on menu-role shortcuts (Cmd+C in webviews) — handle keys in JS.
- **macOS Accessibility permission (Scrolling Capture)**: posting synthetic scroll
  events needs Accessibility (separate from Screen Recording). The first run
  prompts and registers the app; in dev the grant attaches to the *terminal*.
  Without it the capture aborts with an explanatory dialog.

## Updates

Releases are published to GitHub Releases on `jorgegorka/screen-for-me`; the
updater endpoint is `https://github.com/jorgegorka/screen-for-me/releases/latest/download/latest.json`
(`plugins.updater` in tauri.conf.json). `npm run release` does everything:
verifies version consistency (package.json / tauri.conf.json / Cargo.toml) and
that the `v<version>` release doesn't exist, builds a Developer ID-signed and
notarized bundle, emits minisign-signed updater artifacts
(`createUpdaterArtifacts`), generates `latest.json`
(`scripts/latest-json.mjs`, unit-tested), and uploads the .dmg, .app.tar.gz
and manifest with `gh`. Secrets are exported in the shell environment (never
committed): `TAURI_SIGNING_PRIVATE_KEY` (path to `~/.tauri/screenforme.key`),
its `_PASSWORD`, and `APPLE_SIGNING_IDENTITY` / `APPLE_ID` / `APPLE_PASSWORD`
(app-specific) / `APPLE_TEAM_ID` for notarization. Never commit the private
key; the pubkey in tauri.conf.json must
match it or every update check fails signature verification. In-app:
`windows.rs::check_for_updates(app, silent)` — the tray item is the loud path,
and lib.rs auto-checks silently 10 s after launch then daily (release builds
only). Bumping a version means updating package.json, tauri.conf.json **and**
src-tauri/Cargo.toml together (`npm run release` refuses on mismatch).

## Design Context

`PRODUCT.md` (strategic) and `DESIGN.md` (visual system) at the repo root guide all UI work — read them before designing or restyling anything. Register: product ("invisible, native, fast"; a future landing page is brand-register per-task). Positioning: the fastest capture-to-share loop. Key rules: one accent violet #7c5ce6 (Signal Violet, from the brand mark; hover/outlines #9172e7 Violet Lift; legacy blues #4f8ef7/#4a9eff migrate on contact — #4f8ef7 survives only as an annotation ink), the brand spectrum (blue→violet→magenta→red) only as a thin neon line at three sanctioned moments (overlay badge, countdown ring, scrollcap recording pill — DESIGN.md Spectrum Rule), translucent dark glass for transient HUDs vs. opaque native grays for windows, shadows only on floating panels (plus the Signal Glow halo on active states), no decorative motion. Anti-references: Electron heaviness, feature-bloat pro tools, startup-SaaS styling.
