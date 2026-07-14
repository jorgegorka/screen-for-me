# Changelog

All notable changes to Screen for me are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/jorgegorka/screen-for-me/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/jorgegorka/screen-for-me/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/jorgegorka/screen-for-me/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/jorgegorka/screen-for-me/releases/tag/v1.0.0
