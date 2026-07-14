# Textured-Strip Matching + Fluid Scroll Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scroll stitching matches on the most textured strip of each frame (clean seams on sparse pages), and capture scrolling glides in 1-line events with larger adaptive steps and a shorter settle (smoother and ≈40% faster).

**Architecture:** Task 1 adds a pure `most_textured_strip` helper to `stitch.rs` and makes `find_scroll_offset` match that strip instead of rows 0–32; the offset math generalizes (strip at row `s` matching prev at `p` ⇒ offset `p − s`). Task 2 adds `post_scroll_smooth` (1-line events, 25 ms apart) to `scroll_input.rs` and a pure `step_lines` sizing function to `scrolling.rs`, replacing the fixed 5-line step and 350 ms settle.

**Tech Stack:** Rust; `image` crate for the pure stitching math; CoreGraphics events for scroll input (macOS-only file).

## Global Constraints

- Strip candidates: stride `STRIP_ROWS / 2 = 16`, capped at `min(h / 2, h - STRIP_ROWS)`; `s = 0` always a candidate; ties keep the smallest `s` (spec: textured-strip).
- Texture score: per-RGB-channel variance over the existing `SAMPLE_STEP` grid, summed; alpha ignored (spec: textured-strip).
- `MAX_MEAN_DIFF = 6.0` match threshold, sampled diff computation, and smallest-offset-wins tie behavior unchanged (spec: textured-strip).
- Glide: `lines` individual 1-line LINE-unit wheel events spaced 25 ms apart; no PIXEL/inertial scrolling (spec: fluid-scroll).
- Step size: `step_lines(axis_points) = clamp(floor(axis_points * 0.6 / 10.0), 1, 8)`; the nominal fallback derives from the same computed `lines`; horizontal directions pass the region width (the existing `axis_points`) (spec: fluid-scroll).
- `SETTLE` becomes 200 ms (was 350 ms) (spec: fluid-scroll).
- No changes to end-detection (`Some(0)` + `frames_similar`), `MAX_FRAMES`, `MAX_COMPOSITE_PX`, or the pill.
- Gates before done: `cd src-tauri && cargo test`, `npm run build`, `npm test` (CLAUDE.md).

---

### Task 1: Textured-strip offset matching

**Files:**
- Modify: `src-tauri/src/capture/stitch.rs:40-79` (constants + `find_scroll_offset`), tests module at end of file
- No other files (call sites in `scrolling.rs` are signature-compatible)

**Interfaces:**
- Consumes: existing `STRIP_ROWS`, `SAMPLE_STEP`, `MAX_MEAN_DIFF` constants; `RgbaImage`.
- Produces: `pub(crate) fn most_textured_strip(img: &RgbaImage) -> u32` (strip start row); `find_scroll_offset(prev, next) -> Option<u32>` keeps its exact signature and semantics (offset in pixels), so `scrolling.rs` needs no changes in this task.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src-tauri/src/capture/stitch.rs`:

```rust
    /// Regression: whitespace between form sections used to blank the top
    /// strip, matching every offset and losing the real one. The textured
    /// strip below must recover the exact offset.
    #[test]
    fn blank_top_strip_still_matches_exact_offset() {
        let mut src = noise_image(160, 600, 21);
        // Uniform band covering both frames' top strips.
        for y in 0..100 {
            for x in 0..160 {
                src.put_pixel(x, y, image::Rgba([250, 250, 250, 255]));
            }
        }
        let prev = window(&src, 0, 0, 160, 200);
        let next = window(&src, 0, 37, 160, 200);
        assert_eq!(find_scroll_offset(&prev, &next), Some(37));
    }

    /// The chosen strip must not be one that lies entirely inside a blank
    /// region (strips fully inside rows 0..100 start at s <= 68).
    #[test]
    fn textured_strip_skips_blank_region() {
        let mut img = noise_image(160, 200, 23);
        for y in 0..100 {
            for x in 0..160 {
                img.put_pixel(x, y, image::Rgba([250, 250, 250, 255]));
            }
        }
        assert!(most_textured_strip(&img) > 68);
    }

    /// On an all-noise frame any candidate is fine; the start must be legal.
    #[test]
    fn textured_strip_is_legal_candidate() {
        let img = noise_image(160, 200, 25);
        let s = most_textured_strip(&img);
        assert!(s <= 100, "start {s} beyond h/2");
        assert_eq!(s % 16, 0, "start {s} not on the candidate stride");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test stitch`
Expected: compile error — `cannot find function most_textured_strip` (RED; the helper doesn't exist). After Step 3 partially lands you may also see `blank_top_strip_still_matches_exact_offset` fail with `Some(0) != Some(37)` if run against the old matcher — that is the bug being fixed.

- [ ] **Step 3: Add `most_textured_strip` and rework `find_scroll_offset`**

In `src-tauri/src/capture/stitch.rs`, after the `MAX_MEAN_DIFF` constant, add:

```rust
/// Strip-start candidates are examined at this stride; starts are capped at
/// h/2 so at least half the frame remains below the strip for the offset
/// search range.
const STRIP_CANDIDATE_STRIDE: u32 = STRIP_ROWS / 2;

/// Start row of the most textured `STRIP_ROWS`-tall strip in the top half of
/// `img`: per-channel variance over the sampled grid, summed across RGB.
/// A blank strip (whitespace between form sections) matches at every offset;
/// the busiest strip keeps the match unambiguous. Ties keep the smallest
/// start.
pub(crate) fn most_textured_strip(img: &RgbaImage) -> u32 {
    let (w, h) = img.dimensions();
    if h <= STRIP_ROWS || w == 0 {
        return 0;
    }
    let max_start = (h / 2).min(h - STRIP_ROWS);
    let mut best_start = 0u32;
    let mut best_score = f64::MIN;
    let mut s = 0;
    while s <= max_start {
        let mut sum = [0u64; 3];
        let mut sum_sq = [0u64; 3];
        let mut n = 0u64;
        let mut y = 0;
        while y < STRIP_ROWS {
            let mut x = 0;
            while x < w {
                let p = img.get_pixel(x, s + y).0;
                for c in 0..3 {
                    sum[c] += u64::from(p[c]);
                    sum_sq[c] += u64::from(p[c]) * u64::from(p[c]);
                }
                n += 1;
                x += SAMPLE_STEP;
            }
            y += SAMPLE_STEP;
        }
        let nf = n as f64;
        let score: f64 = (0..3)
            .map(|c| sum_sq[c] as f64 / nf - (sum[c] as f64 / nf).powi(2))
            .sum();
        if score > best_score {
            best_score = score;
            best_start = s;
        }
        s += STRIP_CANDIDATE_STRIDE;
    }
    best_start
}
```

Then replace the body of `find_scroll_offset` with:

```rust
pub(crate) fn find_scroll_offset(prev: &RgbaImage, next: &RgbaImage) -> Option<u32> {
    let (w, h) = prev.dimensions();
    if next.dimensions() != (w, h) || h <= STRIP_ROWS || w == 0 {
        return None;
    }
    let strip_start = most_textured_strip(next);
    let mut best_offset = 0u32;
    let mut best_diff = f64::MAX;
    for offset in 0..=(h - STRIP_ROWS - strip_start) {
        let mut sum = 0u64;
        let mut samples = 0u64;
        let mut y = 0;
        while y < STRIP_ROWS {
            let mut x = 0;
            while x < w {
                let a = prev.get_pixel(x, strip_start + y + offset).0;
                let b = next.get_pixel(x, strip_start + y).0;
                for c in 0..3 {
                    sum += (i32::from(a[c]) - i32::from(b[c])).unsigned_abs() as u64;
                }
                samples += 3;
                x += SAMPLE_STEP;
            }
            y += SAMPLE_STEP;
        }
        let diff = sum as f64 / samples as f64;
        if diff < best_diff {
            best_diff = diff;
            best_offset = offset;
        }
    }
    (best_diff <= MAX_MEAN_DIFF).then_some(best_offset)
}
```

Update the doc comment above it: the strip slid down `prev` is now the most
textured strip of `next` (found by `most_textured_strip`), not the top strip.

- [ ] **Step 4: Run the Rust suite**

Run: `cd src-tauri && cargo test`
Expected: all PASS (25 existing + 3 new = 28). The pre-existing directional tests (`exact_offset_detected`, `up/left/right_scroll_stitches_through_normalize`) must pass unchanged — full-noise frames give exact matches for any chosen strip.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/capture/stitch.rs
git commit -m "Match scroll offsets on the most textured strip, not the top strip"
```

---

### Task 2: Fluid scroll steps

**Files:**
- Modify: `src-tauri/src/capture/scroll_input.rs` (add `post_scroll_smooth`)
- Modify: `src-tauri/src/capture/scrolling.rs:23-28` (constants), `:56-63` (nominal computation), `:74` (scroll call), new tests module at end

**Interfaces:**
- Consumes: existing `post_scroll(direction, lines)` in `scroll_input.rs`; existing `NOMINAL_POINTS_PER_LINE = 10.0` and `axis_points` in `scrolling.rs::run`.
- Produces: `pub fn post_scroll_smooth(direction: ScrollDirection, lines: i32) -> Result<(), String>` in `scroll_input.rs`; `pub(crate) fn step_lines(axis_points: f64) -> i32` in `scrolling.rs`. Nothing outside these two files changes.

- [ ] **Step 1: Write the failing test**

Add at the end of `src-tauri/src/capture/scrolling.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::step_lines;

    #[test]
    fn step_lines_scales_and_clamps() {
        assert_eq!(step_lines(100.0), 6); // 60% of 100 pt / 10 pt-per-line
        assert_eq!(step_lines(50.0), 3);
        assert_eq!(step_lines(20.0), 1); // floor(1.2)
        assert_eq!(step_lines(5.0), 1); // floor(0.3) clamped up to 1
        assert_eq!(step_lines(1000.0), 8); // cap
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test step_lines`
Expected: compile error — `cannot find function step_lines` (RED).

- [ ] **Step 3: Implement `step_lines` and the new constants in `scrolling.rs`**

Replace the constants block (currently `SCROLL_LINES` and `SETTLE`; keep `MAX_FRAMES`, `MAX_COMPOSITE_PX`, `NOMINAL_POINTS_PER_LINE` as they are):

```rust
const MAX_FRAMES: u32 = 40;
const MAX_COMPOSITE_PX: u32 = 20_000;
/// One wheel line scrolls ≈ 10 points; used for step sizing and the nominal
/// fallback offset when correlation fails.
const NOMINAL_POINTS_PER_LINE: f64 = 10.0;
/// Per-step cap (≈80 points) and the fraction of the region's scroll-axis
/// extent one step may cover — consecutive frames must keep enough overlap
/// for offset matching.
const MAX_STEP_LINES: i32 = 8;
const STEP_EXTENT_FRACTION: f64 = 0.6;
const SETTLE: Duration = Duration::from_millis(200);

/// Lines to scroll per frame for a region extending `axis_points` along the
/// scroll axis.
pub(crate) fn step_lines(axis_points: f64) -> i32 {
    ((axis_points * STEP_EXTENT_FRACTION / NOMINAL_POINTS_PER_LINE) as i32).clamp(1, MAX_STEP_LINES)
}
```

In `run()`, after `axis_points`/`axis_px` are computed, derive the step and use it for the nominal fallback (replacing `SCROLL_LINES` in the `nominal_px` expression):

```rust
    let lines = step_lines(axis_points);
    // Fallback offset when correlation can't find one: the nominal scroll
    // distance converted from points to frame pixels along the scroll axis.
    let nominal_px = ((lines as f64 * NOMINAL_POINTS_PER_LINE) * axis_px as f64
        / axis_points.max(1.0))
    .round() as u32;
```

And change the scroll call inside the loop from
`scroll_input::post_scroll(direction, SCROLL_LINES)` to:

```rust
        let step = scroll_input::post_scroll_smooth(direction, lines)
```

(the rest of that expression — `.map_err(CaptureError::Tool).and_then(...)` — is unchanged).

- [ ] **Step 4: Implement `post_scroll_smooth` in `scroll_input.rs`**

Add after `post_scroll`:

```rust
/// Gap between the 1-line events of a smooth scroll step.
const GLIDE_STEP_GAP: std::time::Duration = std::time::Duration::from_millis(25);

/// One capture step as a glide: `lines` individual 1-line events spaced
/// `GLIDE_STEP_GAP` apart, so the page rolls between grabs instead of
/// teleporting. LINE units, like `post_scroll` — no trackpad inertia.
pub fn post_scroll_smooth(direction: ScrollDirection, lines: i32) -> Result<(), String> {
    for i in 0..lines.max(1) {
        if i > 0 {
            std::thread::sleep(GLIDE_STEP_GAP);
        }
        post_scroll(direction, 1)?;
    }
    Ok(())
}
```

- [ ] **Step 5: Run the Rust suite and frontend gates**

Run: `cd src-tauri && cargo test`
Expected: all PASS (28 from Task 1 + 1 new = 29), no `unused` warnings (`SCROLL_LINES` was removed, `post_scroll` is still called by `post_scroll_smooth`).

Run from repo root: `npm run build` (expected: succeeds) and `npm test` (expected: 17 passing — regression gate, frontend untouched).

Manual (deferred to the user if no GUI access): a scrolling capture should glide smoothly, finish noticeably faster, and stitch cleanly.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/capture/scrolling.rs src-tauri/src/capture/scroll_input.rs
git commit -m "Glide scroll steps: 1-line events, adaptive step size, shorter settle"
```
