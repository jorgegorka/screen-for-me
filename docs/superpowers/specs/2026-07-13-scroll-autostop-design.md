# Scrolling capture: auto-stop at the end of the page

**Date:** 2026-07-13
**Status:** Approved

## Problem

Scrolling capture keeps appending frames after the page has reached the
bottom, until the user presses Stop or the 40-frame cap hits. The tail of
the composite fills with duplicated content.

Root cause: the loop's end-detection (`src-tauri/src/capture/scrolling.rs:95`)
only fires when consecutive frames are **exactly** pixel-identical
(`stitch.rs::frames_identical`, sampled strict equality). At the real end of
a page, frames differ slightly — macOS overlay scrollbar fading, elastic
bounce settling, blinking caret, antialiasing noise. `find_scroll_offset`
returns `Some(0)`, the identity check fails, and the `Some(0) | None`
fallback appends a nominal-offset strip of duplicate content every
iteration.

## Decision

Approach A: make the end-detection tolerant instead of exact.

- Replace `frames_identical(a, b) -> bool` with `frames_similar(a, b) -> bool`
  in `src-tauri/src/capture/stitch.rs`: mean per-channel absolute difference
  over the existing `SAMPLE_STEP` grid (RGB channels, alpha ignored —
  matching `find_scroll_offset`), returning true when the mean is
  `<= MAX_MEAN_DIFF` (the existing 6.0 constant the strip matcher already
  trusts). Dimension mismatch → false.
- `scrolling.rs` end-detection becomes
  `Some(0) if stitch::frames_similar(&prev_n, &frame_n) => break`.
- Disambiguation stays sound: a fixed header pinning the best offset at 0
  while content actually moved produces a large full-frame mean diff, so
  the nominal fallback still applies to that case.
- On end detection the loop breaks **without** appending the final frame
  (same as the current identical case) — no duplicate tail rows.

## Non-goals

- No change to the Stop button, frame cap (`MAX_FRAMES`), composite size
  cap, scroll input, or settle timing.
- Animated content at the page end (video, carousel) may still defeat
  detection; the existing 40-frame cap remains the backstop.

## Testing

Unit tests in `stitch.rs` using the existing `noise_image` helper:
- identical frames → similar (existing test updated to the new name)
- frames with small uniform noise (per-channel delta well under the
  threshold, e.g. ±3) → similar
- unrelated noise frames → not similar
- dimension mismatch → not similar

Gates: `cargo test`, `npm run build`, `npm test`. Manual: scroll-capture a
page to its bottom and confirm capture stops by itself within one step of
reaching the end.
