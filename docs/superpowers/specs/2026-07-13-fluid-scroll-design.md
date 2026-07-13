# Scrolling capture: fluid scroll steps

**Date:** 2026-07-13
**Status:** Approved

## Problem

Each capture step posts a single 5-line wheel event (≈50 pt jump) and then
sleeps 350 ms. The page visibly teleports between freezes, and overall
progress is ≈0.14 pt/ms.

## Decision

Glide, scroll further per step, and settle for less:

- New `scroll_input::post_scroll_smooth(direction: ScrollDirection,
  lines: i32) -> Result<(), String>`: posts `lines` individual 1-line
  wheel events spaced 25 ms apart (LINE units — no trackpad inertia).
  Replaces the single `post_scroll(direction, SCROLL_LINES)` call in
  `scrolling.rs::run`. `post_scroll` stays as the single-event primitive
  that `post_scroll_smooth` calls per line.
- Adaptive step size, new pure function in `scrolling.rs`:
  `step_lines(region_height_pt: f64) -> i32` =
  `clamp(floor(region_height_pt * 0.4 / 10.0), 1, 8)` — up to 8 lines
  (≈80 pt) per frame, never more than ~40% of the region height so
  consecutive frames keep enough overlap for offset matching
  (`NOMINAL_POINTS_PER_LINE = 10.0` is the existing constant).
  `SCROLL_LINES` (5) is replaced by the computed value; the nominal
  fallback offset derives from the same computed `lines`, staying
  consistent with what was actually posted.
  For horizontal directions the relevant extent is the region width —
  `step_lines` receives the extent along the scroll axis (the existing
  `axis_points` in `run`).
- `SETTLE` drops 350 ms → 200 ms: the 25 ms-spaced glide already gives the
  page ~200 ms of render time per step before the settle starts.
- Net: ≈0.2 pt/ms — ≈40% faster capture with visibly smooth motion.

## Risk

Slow-rendering pages get less settle time. Mitigations: the textured-strip
matcher (2026-07-13-textured-strip-matching spec) makes offset detection
robust to partially-settled frames, and a failed match falls back to the
nominal offset for that frame instead of corrupting the composite. If
manual testing shows mid-render seams, `SETTLE` is the single knob to
raise.

## Non-goals

- No PIXEL-unit or inertial scrolling (breaks the discrete-step model).
- No change to end-detection, thresholds, frame caps, or the pill.

## Testing

- Unit: `step_lines` — small region (100 pt → 4), tiny region (20 pt → 1),
  large region (1000 pt → 8, the cap), plus exact 40% boundary
  (125 pt → 5; 50 pt → 2).
- Existing stitch/scrolling tests unchanged.
- Gates: `cargo test`, `npm run build`, `npm test`.
- Manual: capture the same form page — motion glides, capture completes
  noticeably faster, seams stay clean.

*Amended 2026-07-13:* The fraction was lowered from 0.6 to 0.4 because the
textured-strip matcher (`stitch::most_textured_strip`) may pick a strip at
h/2, leaving a worst-case search window of h/2 − 32 px; the step must stay
inside it to remain detectable.
