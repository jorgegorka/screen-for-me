//! Pure stitching math for scrolling capture. Deliberately free of Tauri and
//! file I/O so `cargo test` covers it (same idea as the editor's geometry.ts).
//!
//! Everything works on "normalized" frames: `normalize` rotates/flips each
//! frame so the scroll direction becomes "down", stitching always appends rows
//! at the bottom, and `denormalize` undoes the transform on the final image.

use image::{GenericImage, GenericImageView, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

pub(crate) fn normalize(frame: &RgbaImage, dir: ScrollDirection) -> RgbaImage {
    use image::imageops::{flip_vertical, rotate270, rotate90};
    match dir {
        ScrollDirection::Down => frame.clone(),
        ScrollDirection::Up => flip_vertical(frame),
        // Content moving right maps to moving down under a clockwise rotation.
        ScrollDirection::Right => rotate90(frame),
        ScrollDirection::Left => rotate270(frame),
    }
}

pub(crate) fn denormalize(composite: RgbaImage, dir: ScrollDirection) -> RgbaImage {
    use image::imageops::{flip_vertical, rotate270, rotate90};
    match dir {
        ScrollDirection::Down => composite,
        ScrollDirection::Up => flip_vertical(&composite),
        ScrollDirection::Right => rotate270(&composite),
        ScrollDirection::Left => rotate90(&composite),
    }
}

const STRIP_ROWS: u32 = 32;
const SAMPLE_STEP: u32 = 4;
/// Mean per-channel abs difference below which a strip match is trusted.
const MAX_MEAN_DIFF: f64 = 6.0;

/// How many pixels the content moved between two normalized frames: slide the
/// top strip of `next` down `prev` and take the best (smallest-offset) match.
/// Browsers smooth-scroll, so the nominal scroll amount can't be trusted.
pub(crate) fn find_scroll_offset(prev: &RgbaImage, next: &RgbaImage) -> Option<u32> {
    let (w, h) = prev.dimensions();
    if next.dimensions() != (w, h) || h <= STRIP_ROWS || w == 0 {
        return None;
    }
    let mut best_offset = 0u32;
    let mut best_diff = f64::MAX;
    for offset in 0..=(h - STRIP_ROWS) {
        let mut sum = 0u64;
        let mut samples = 0u64;
        let mut y = 0;
        while y < STRIP_ROWS {
            let mut x = 0;
            while x < w {
                let a = prev.get_pixel(x, y + offset).0;
                let b = next.get_pixel(x, y).0;
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

/// Sampled full-frame equality — distinguishes "reached the end of the page"
/// from "a fixed header made offset 0 look like the best match".
pub(crate) fn frames_identical(a: &RgbaImage, b: &RgbaImage) -> bool {
    if a.dimensions() != b.dimensions() {
        return false;
    }
    let (w, h) = a.dimensions();
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            if a.get_pixel(x, y) != b.get_pixel(x, y) {
                return false;
            }
            x += SAMPLE_STEP;
        }
        y += SAMPLE_STEP;
    }
    true
}

/// Grow the composite by the `new_rows` bottom rows of `next`.
pub(crate) fn append_rows(composite: RgbaImage, next: &RgbaImage, new_rows: u32) -> RgbaImage {
    let (w, composite_h) = composite.dimensions();
    let (next_w, next_h) = next.dimensions();
    let new_rows = new_rows.min(next_h);
    let mut out = RgbaImage::new(w, composite_h + new_rows);
    out.copy_from(&composite, 0, 0).expect("composite fits");
    let strip = next.view(0, next_h - new_rows, next_w.min(w), new_rows).to_image();
    out.copy_from(&strip, 0, composite_h).expect("strip fits");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic per-pixel noise so strips match at exactly one offset.
    fn noise_image(w: u32, h: u32, seed: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            let mut v = x
                .wrapping_mul(374_761_393)
                ^ y.wrapping_mul(668_265_263)
                ^ seed.wrapping_mul(2_246_822_519);
            v ^= v >> 13;
            v = v.wrapping_mul(1_274_126_177);
            image::Rgba([(v & 0xff) as u8, ((v >> 8) & 0xff) as u8, ((v >> 16) & 0xff) as u8, 255])
        })
    }

    /// A viewport-sized window into a taller source, like one scroll frame.
    fn window(src: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        src.view(x, y, w, h).to_image()
    }

    #[test]
    fn exact_offset_detected() {
        let src = noise_image(160, 600, 7);
        let prev = window(&src, 0, 0, 160, 200);
        let next = window(&src, 0, 37, 160, 200);
        assert_eq!(find_scroll_offset(&prev, &next), Some(37));
    }

    #[test]
    fn identical_frames_offset_zero() {
        let frame = noise_image(160, 200, 3);
        assert_eq!(find_scroll_offset(&frame, &frame), Some(0));
        assert!(frames_identical(&frame, &frame));
    }

    #[test]
    fn unrelated_frames_no_match() {
        let a = noise_image(160, 200, 1);
        let b = noise_image(160, 200, 2);
        assert_eq!(find_scroll_offset(&a, &b), None);
        assert!(!frames_identical(&a, &b));
    }

    #[test]
    fn append_reconstructs_source() {
        let src = noise_image(160, 600, 9);
        let prev = window(&src, 0, 0, 160, 200);
        let next = window(&src, 0, 37, 160, 200);
        let composite = append_rows(prev, &next, 37);
        assert_eq!(composite.dimensions(), (160, 237));
        assert_eq!(composite, window(&src, 0, 0, 160, 237));
    }

    #[test]
    fn normalize_round_trips_every_direction() {
        let frame = noise_image(90, 60, 5);
        for dir in [
            ScrollDirection::Up,
            ScrollDirection::Down,
            ScrollDirection::Left,
            ScrollDirection::Right,
        ] {
            assert_eq!(denormalize(normalize(&frame, dir), dir), frame, "{dir:?}");
        }
    }

    /// End-to-end for a horizontal direction: proves the rotation mapping puts
    /// new content at the bottom of the normalized frames.
    #[test]
    fn right_scroll_stitches_through_normalize() {
        let src = noise_image(600, 160, 11);
        let prev = window(&src, 0, 0, 200, 160);
        let next = window(&src, 37, 0, 200, 160);
        let dir = ScrollDirection::Right;
        let prev_n = normalize(&prev, dir);
        let next_n = normalize(&next, dir);
        let offset = find_scroll_offset(&prev_n, &next_n).expect("confident match");
        assert_eq!(offset, 37);
        let composite = denormalize(append_rows(prev_n, &next_n, offset), dir);
        assert_eq!(composite, window(&src, 0, 0, 237, 160));
    }

    /// Same for Up: new content enters at the top, composite grows upward.
    #[test]
    fn up_scroll_stitches_through_normalize() {
        let src = noise_image(160, 600, 13);
        let prev = window(&src, 0, 400, 160, 200);
        let next = window(&src, 0, 363, 160, 200);
        let dir = ScrollDirection::Up;
        let prev_n = normalize(&prev, dir);
        let next_n = normalize(&next, dir);
        let offset = find_scroll_offset(&prev_n, &next_n).expect("confident match");
        assert_eq!(offset, 37);
        let composite = denormalize(append_rows(prev_n, &next_n, offset), dir);
        assert_eq!(composite, window(&src, 0, 363, 160, 237));
    }
}
