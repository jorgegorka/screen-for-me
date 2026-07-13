# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Screen for me** — a native desktop screenshot application built with the [Native SDK](https://native-sdk.dev/introduction). Version 1 mimics [CleanShot](https://cleanshot.com)'s screen *capture* features (fullscreen, window, and area capture, plus post-capture UX). **No video/screen recording in v1.**

The app uses the zero-config Zig-core layout (`app.zon` + `src/main.zig` + `src/app.native` + `assets/`) — the CLI generates the build graph; there are no build files to edit. The current code is still the `native init` counter starter awaiting the first capture features.

## Native SDK: read the skills first

Do NOT rely on general model knowledge of the Native SDK — it will be wrong. The SDK ships its own agent skills via the installed CLI. Load the relevant one before implementing or explaining anything:

```bash
native skills list
native skills get core --full     # foundation: app.zon, App/Runtime, bridge, packaging
native skills get native-ui       # authoring .native markup views + Model/Msg/update
native skills get automation      # testing/driving the running app, snapshots, screenshots
native skills get zig             # when `zig build` fails with "no member named" std errors
native skills get ts-core         # only if a TypeScript src/core.ts app core is used
```

This app is **native-rendered** (Native markup views drawn by the SDK's own engine — no WebView frontend), so `native-ui` is the primary skill for UI work.

## Commands

```bash
zig build run                                  # build and launch the app
zig build dev                                  # dev mode (markup hot reload via watch_path)
zig build test                                 # Zig tests
native validate app.zon                        # validate the manifest
native doctor --manifest app.zon --strict      # environment/manifest health check
native build && native package --target macos  # package (works without ejecting)
```

Always run **both** `zig build` and `zig build test` before calling a change done — Zig's lazy analysis means code only one of them references can sit broken under the other.

GUI smoke tests (the automation server is built into every app):

```bash
zig build run -Dplatform=macos -Dautomation=true
zig-out/bin/native automate snapshot
```

**Zig version: 0.16.0 required.** If a build fails on std APIs (`std.fs.cwd`, `ArrayList.init`, `std.io`, `GeneralPurposeAllocator`), the code uses pre-0.16 idioms — run `native skills get zig` for the error-to-idiom map.

## Architecture

A native-rendered Native SDK app is an Elm-style loop:

- `src/<view>.native` — the entire UI as declarative Native markup: elements, layout, bindings, message dispatch. Markup never mutates state; it binds values and dispatches messages.
- `src/main.zig` — `Model` (plain struct, every field needs a default), `Msg` (tagged union), `update(model, msg)`, wired into `native_sdk.UiApp(Model, Msg)`.
- `app.zon` — the manifest and source of truth for identity, windows, permissions, capabilities, and packaging.

Key patterns (details in the `native-ui` skill):

- Use `App.create(allocator, options)` / `destroy`, never by-value `init` — the multi-MB app struct and Model must not ride the stack. Assign boot state through the returned pointer.
- Dev builds use runtime markup with `watch_path` for ~2s hot reload; release builds compile the markup at comptime via `canvas.CompiledMarkupView` (markup errors become compile errors). Gate on `@import("builtin").mode`.
- Menu-bar presence (CleanShot-style status item) uses `Options.status_item` / `status_item_fn`; its items dispatch through the same `on_command` mapping as menus and toolbars.
- Editors: associate `*.native` with HTML for highlighting (`.vscode/settings.json` → `"files.associations": {"*.native": "html"}`).

## Product scope (v1)

Capture features modeled on CleanShot: capture area / window / fullscreen, and post-capture handling (e.g. quick-access overlay, copy/save/annotate flows). macOS is the primary target and the SDK's deepest-supported platform. Screen capture itself will need macOS screen-recording permission and likely native capture APIs beyond stock widgets — check `native skills get core --full` (permissions/capabilities in `app.zon`) before designing that layer. Explicitly out of scope for v1: video recording of any kind.
