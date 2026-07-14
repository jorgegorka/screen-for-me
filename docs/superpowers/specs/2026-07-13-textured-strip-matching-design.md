# Scroll stitching: textured-strip offset matching

**Date:** 2026-07-13
**Status:** Approved

## Problem

`stitch.rs::find_scroll_offset` always matches the **top** 32-row strip of
the new frame against the previous frame. When that strip is blank or
low-texture (whitespace between form sections), it matches at many offsets
almost equally well; the smallest-offset tie-break returns a wrong or
unconfident result, the loop falls back to the nominal scroll estimate,
and seams come out slightly misaligned (rows clipped or duplicated —
visible around "Horas estimadas" / "Resultado de la eficacia" in the
2026-07-13 verification capture).

## Decision

Approach A: pick the most textured strip of the new frame for matching.

- New pure helper in `src-tauri/src/capture/stitch.rs`:
  `most_textured_strip(img: &RgbaImage) -> u32` — returns the strip start
  row `s` with the highest texture score.
  - Candidates: `s = 0, 16, 32, …` (stride `STRIP_ROWS / 2 = 16`) with
    `s <= min(h / 2, h - STRIP_ROWS)`; `s = 0` is always a candidate.
  - Score: variance of the sampled pixels in rows `[s, s + STRIP_ROWS)` —
    the existing `SAMPLE_STEP` grid, per-RGB-channel variance summed
    (alpha ignored). Ties keep the smallest `s`.
- `find_scroll_offset(prev, next)` (signature unchanged) matches the strip
  of `next` starting at `s` against `prev` rows `[s + o, s + o + STRIP_ROWS)`
  for `o in 0..=(h - STRIP_ROWS - s)`, and returns the best `o`.
  - Capping `s` at `h / 2` guarantees detectable offsets of at least
    `h / 2 - STRIP_ROWS` — several times a real scroll step (~100 px on a
    ≥ 800 px frame).
  - Match threshold (`MAX_MEAN_DIFF = 6.0`), sampled diff computation, and
    smallest-offset-wins tie behavior are unchanged.
- A fixed header no longer pins matches at offset 0 when there is texture
  below it, since the chosen strip moves below the header. The existing
  `Some(0)` + `frames_similar` end-detection and nominal fallback in
  `scrolling.rs` stay exactly as they are.

## Non-goals

- No multi-strip voting, no full-frame correlation (revisit only if this
  proves insufficient).
- No changes to `scrolling.rs`, end-detection, thresholds, or constants
  other than the new candidate stride derived from `STRIP_ROWS`.

## Testing

Unit tests in `stitch.rs` (existing `noise_image`/`window` helpers):
- **Regression:** frame pair whose top ~120 rows are uniform white with
  noise texture below, shifted by a known offset — must return exactly
  that offset (today's top-strip matching cannot).
- `most_textured_strip` on such a frame returns a strip start below the
  blank region; on an all-noise frame any candidate is valid (assert the
  returned start is a legal candidate).
- All existing stitch tests pass unchanged (they use full-noise frames, so
  exact offsets must still be found regardless of which strip is chosen).

Gates: `cargo test`, `npm run build`, `npm test`. Manual: repeat the same
form-page scrolling capture; seams that previously clipped "Horas
estimadas" should be clean.
