# Scrolling Capture Auto-Stop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scrolling capture stops by itself when the page reaches its end, instead of appending duplicate frames until the user presses Stop.

**Architecture:** Replace the exact-equality end-detection (`stitch.rs::frames_identical`) with a tolerant `frames_similar` (mean per-channel abs diff over the existing sample grid, threshold `MAX_MEAN_DIFF = 6.0`). `scrolling.rs` breaks out of the loop when the best offset is 0 AND the frames are similar; the fixed-header case still produces a large full-frame diff and falls through to the nominal fallback unchanged.

**Tech Stack:** Rust (`image` crate), pure functions in `src-tauri/src/capture/stitch.rs` unit-tested with the existing synthetic-noise helpers.

## Global Constraints

- Similarity threshold: reuse the existing `MAX_MEAN_DIFF: f64 = 6.0` constant (`stitch.rs:43`) — do not introduce a new constant.
- Sampling: the existing `SAMPLE_STEP: u32 = 4` grid, RGB channels only (alpha ignored), matching `find_scroll_offset`.
- Dimension mismatch → not similar (false).
- On end detection the loop breaks WITHOUT appending the final frame (same as current behavior).
- No change to Stop button, `MAX_FRAMES`, `MAX_COMPOSITE_PX`, scroll input, or settle timing.
- Gates before done: `cd src-tauri && cargo test`, `npm run build`, `npm test` (CLAUDE.md).

---

### Task 1: Tolerant end-of-page detection

**Files:**
- Modify: `src-tauri/src/capture/stitch.rs:81-100` (replace `frames_identical` with `frames_similar`), tests at `stitch.rs:144-157`
- Modify: `src-tauri/src/capture/scrolling.rs:93-95` (call site)

**Interfaces:**
- Consumes: `RgbaImage` (`image` crate), existing `SAMPLE_STEP`, `MAX_MEAN_DIFF` constants in `stitch.rs`.
- Produces: `pub(crate) fn frames_similar(a: &RgbaImage, b: &RgbaImage) -> bool` in `stitch.rs`. `frames_identical` is deleted (its only caller is `scrolling.rs:95` plus two test assertions, all updated in this task).

- [ ] **Step 1: Write the failing tests**

In `src-tauri/src/capture/stitch.rs` tests module:

Update the two existing assertions that use `frames_identical`:

```rust
    #[test]
    fn identical_frames_offset_zero() {
        let frame = noise_image(160, 200, 3);
        assert_eq!(find_scroll_offset(&frame, &frame), Some(0));
        assert!(frames_similar(&frame, &frame));
    }

    #[test]
    fn unrelated_frames_no_match() {
        let a = noise_image(160, 200, 1);
        let b = noise_image(160, 200, 2);
        assert_eq!(find_scroll_offset(&a, &b), None);
        assert!(!frames_similar(&a, &b));
    }
```

Add two new tests after `unrelated_frames_no_match`:

```rust
    /// End-of-page frames are never pixel-identical (scrollbar fade, caret,
    /// bounce settling); a small uniform shift must still count as "no
    /// movement".
    #[test]
    fn slightly_noisy_frames_similar() {
        let a = noise_image(160, 200, 3);
        let b = RgbaImage::from_fn(160, 200, |x, y| {
            let p = a.get_pixel(x, y).0;
            image::Rgba([
                p[0].saturating_add(3),
                p[1].saturating_add(3),
                p[2].saturating_add(3),
                p[3],
            ])
        });
        assert!(frames_similar(&a, &b));
    }

    #[test]
    fn dimension_mismatch_not_similar() {
        let a = noise_image(160, 200, 3);
        let b = noise_image(160, 199, 3);
        assert!(!frames_similar(&a, &b));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test stitch`
Expected: compile error — `cannot find function frames_similar in this scope` (this is the RED state; the function doesn't exist yet).

- [ ] **Step 3: Replace `frames_identical` with `frames_similar`**

In `src-tauri/src/capture/stitch.rs`, replace the whole `frames_identical` function (lines 81-100) with:

```rust
/// Sampled full-frame similarity — distinguishes "reached the end of the
/// page" from "a fixed header made offset 0 look like the best match".
/// Tolerant of end-of-scroll noise (scrollbar fade, caret blink, elastic
/// bounce): mean per-channel abs diff at or below `MAX_MEAN_DIFF` counts
/// as no movement. A truly moved frame (fixed-header case) differs across
/// the whole viewport and lands far above the threshold.
pub(crate) fn frames_similar(a: &RgbaImage, b: &RgbaImage) -> bool {
    if a.dimensions() != b.dimensions() {
        return false;
    }
    let (w, h) = a.dimensions();
    if w == 0 || h == 0 {
        return false;
    }
    let mut sum = 0u64;
    let mut samples = 0u64;
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let pa = a.get_pixel(x, y).0;
            let pb = b.get_pixel(x, y).0;
            for c in 0..3 {
                sum += (i32::from(pa[c]) - i32::from(pb[c])).unsigned_abs() as u64;
            }
            samples += 3;
            x += SAMPLE_STEP;
        }
        y += SAMPLE_STEP;
    }
    (sum as f64 / samples as f64) <= MAX_MEAN_DIFF
}
```

- [ ] **Step 4: Update the call site in `scrolling.rs`**

In `src-tauri/src/capture/scrolling.rs`, the offset match (currently lines 93-99) becomes:

```rust
        let offset = match stitch::find_scroll_offset(&prev_n, &frame_n) {
            // No real movement (only scrollbar-fade/caret noise): the end
            // of the scrollable content.
            Some(0) if stitch::frames_similar(&prev_n, &frame_n) => break,
            // Fixed header pinned the match at 0, or nothing matched: trust
            // the nominal scroll distance instead.
            Some(0) | None => nominal_px.clamp(1, frame_n.height().saturating_sub(1).max(1)),
            Some(offset) => offset,
        };
```

(Only the guard function name and the first comment change; the rest is verbatim.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test`
Expected: all tests PASS (21 existing + 2 new = 23), no warnings about unused `frames_identical` (it was deleted, not kept).

- [ ] **Step 6: Run the remaining gates**

Run from repo root: `npm run build` (expected: tsc + vite succeed) and `npm test` (expected: 17 passing — frontend untouched, this is a regression gate).

Manual (deferred to the user if no GUI access): scroll-capture a page to its bottom; the capture should stop on its own within one scroll step of reaching the end, and the composite should have no duplicated tail.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/capture/stitch.rs src-tauri/src/capture/scrolling.rs
git commit -m "Auto-stop scrolling capture at page end via tolerant frame similarity"
```
