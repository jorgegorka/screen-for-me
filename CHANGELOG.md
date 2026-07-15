# Changelog

All notable changes to Screen for me are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.3] - 2026-07-15

### Fixed

- Changing a General setting no longer reverts shortcuts assigned elsewhere
  (e.g. via the Welcome window's ⌘⇧3/4/5 button): the Settings window kept a
  stale copy of the shortcuts and echoed it back on every save, desyncing the
  saved settings from the live registrations.
- The Welcome window no longer shows the macOS-only shortcuts card (with
  non-working buttons) on Linux — a `display: flex` rule overrode the `hidden`
  attribute, the same class of bug as the Settings tabs fix in 1.2.0.
- On Linux, the quick-access overlay no longer jumps to another monitor in the
  middle of a drag-out when "Move to the active screen" is enabled.
- The Welcome window's success message now only appears when ⌘⇧3/4/5 are
  assigned to their matching actions (3 = full screen, 4 = area, 5 = window),
  not any permutation of them.
- The "macOS is still handling this shortcut" warning in Settings is now
  per-key: freeing only one of the system's ⌘⇧3/4/5 shortcuts no longer
  warns when you assign that freed combo.

## [1.2.2] - 2026-07-14

### Added

- Zoom controls in the editor.

### Fixed

- Annotating very long captures (scrolling captures): the editor stage is now
  virtualized so tall images stay responsive.

## [1.2.1] - 2026-07-14

### Changed

- In-app updates are served from GitHub Releases.

## [1.2.0] - 2026-07-14

### Changed

- Editor arrows are now solid and tapered (thin tail widening into the
  head, Skitch-style) and scale with their length, so they grow naturally
  out of the tail point while dragging.

### Fixed

- Settings window tabs (General / Shortcuts / About) now switch their content.
  A `display: flex` rule on the panels overrode the `hidden` attribute, so
  every panel stayed visible regardless of the selected tab.

## [1.1.0] - 2026-07-14

### Added

- Configurable global shortcuts in Settings (Shortcuts tab).
- Static marketing site pages (`site/`).

## [1.0.0] - 2026-07-14

### Added

- Area, window, and full-screen capture with global shortcuts
  (Cmd/Ctrl+Shift+7/8/9).
- Quick-access overlay with drag-out, copy, and save.
- Konva-based annotation editor with undo/redo, pixelate, and counter tool.
- Scrolling capture with automatic stitching and page-end auto-stop (macOS).
- Self-timer that repeats the last-used capture mode.
- Localisation: English, Spanish, French, German, Italian.
- Launch-on-start setting backed by OS login-item state.
- Capture history with pruning.

[Unreleased]: https://github.com/jorgegorka/screen-for-me/compare/v1.2.3...HEAD
[1.2.3]: https://github.com/jorgegorka/screen-for-me/compare/v1.2.2...v1.2.3
[1.2.2]: https://github.com/jorgegorka/screen-for-me/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/jorgegorka/screen-for-me/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/jorgegorka/screen-for-me/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/jorgegorka/screen-for-me/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/jorgegorka/screen-for-me/releases/tag/v1.0.0
