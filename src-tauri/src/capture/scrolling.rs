//! Auto-scroll capture loop: grab the region, scroll one step, stitch, repeat
//! until the frames stop changing, a cap is hit, or the user stops it.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use image::RgbaImage;

use super::{scroll_input, stitch, CaptureError, ScrollDirection};

/// Region to grab, in global logical points (screencapture -R space).
#[derive(Clone, Copy)]
pub struct ScrollRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

const MAX_FRAMES: u32 = 40;
const MAX_COMPOSITE_PX: u32 = 20_000;
/// One wheel line scrolls ≈ 10 points; used for step sizing and the nominal
/// fallback offset when correlation fails.
const NOMINAL_POINTS_PER_LINE: f64 = 10.0;
/// Per-step scroll cap: 8 lines ≈ 80 points.
const MAX_STEP_LINES: i32 = 8;
/// Fraction of the region's scroll-axis extent one step aims to cover.
/// `step_lines` additionally caps the commanded step against
/// `stitch::max_detectable_offset`, so it can never outrun the stitcher's
/// offset search window regardless of this value.
const STEP_EXTENT_FRACTION: f64 = 0.4;
const SETTLE: Duration = Duration::from_millis(200);

/// Nominal pixel distance `lines` wheel lines move on a frame extending
/// `axis_px` pixels over `axis_points` points along the scroll axis.
pub(crate) fn nominal_step_px(lines: i32, axis_points: f64, axis_px: u32) -> u32 {
    ((lines as f64 * NOMINAL_POINTS_PER_LINE) * f64::from(axis_px) / axis_points.max(1.0)).round()
        as u32
}

/// Lines to scroll per frame for a region extending `axis_points` along the
/// scroll axis, grabbed as frames `axis_px` tall along it. Capped so the
/// commanded pixel extent stays within `stitch::max_detectable_offset` — a
/// bigger step would be undetectable and degrade every frame to the nominal
/// fallback offset.
pub(crate) fn step_lines(axis_points: f64, axis_px: u32) -> i32 {
    let mut lines =
        ((axis_points * STEP_EXTENT_FRACTION / NOMINAL_POINTS_PER_LINE) as i32).clamp(1, MAX_STEP_LINES);
    while lines > 1
        && nominal_step_px(lines, axis_points, axis_px) > stitch::max_detectable_offset(axis_px)
    {
        lines -= 1;
    }
    lines
}

/// Run the loop and return the stitched image (already de-normalized).
/// A grab/scroll failure after ≥2 stitched frames returns the partial result —
/// a truncated scroll beats losing everything.
pub fn run(
    region: &ScrollRegion,
    direction: ScrollDirection,
    stop: &AtomicBool,
    work_dir: &Path,
    mut progress: impl FnMut(u32),
) -> Result<RgbaImage, CaptureError> {
    std::fs::create_dir_all(work_dir)?;
    let frame_path = work_dir.join("frame.png");

    scroll_input::warp_cursor(
        region.x + region.width / 2.0,
        region.y + region.height / 2.0,
    )
    .map_err(CaptureError::Tool)?;
    // Let the warp land and the shrunken HUD window settle before frame one.
    std::thread::sleep(Duration::from_millis(200));

    let first = grab(region, &frame_path).map_err(|err| {
        let _ = std::fs::remove_file(&frame_path);
        err
    })?;
    // Fallback offset when correlation can't find one: the nominal scroll
    // distance converted from points to frame pixels along the scroll axis.
    let (axis_points, axis_px) = match direction {
        ScrollDirection::Up | ScrollDirection::Down => (region.height, first.height()),
        ScrollDirection::Left | ScrollDirection::Right => (region.width, first.width()),
    };
    let lines = step_lines(axis_points, axis_px);
    let nominal_px =
        nominal_step_px(lines, axis_points, axis_px).clamp(1, stitch::max_detectable_offset(axis_px).max(1));

    let mut prev_n = stitch::normalize(first, direction);
    let mut composite = stitch::Composite::new(&prev_n);
    let mut frames = 1u32;
    progress(frames);

    while frames < MAX_FRAMES
        && composite.height() < MAX_COMPOSITE_PX
        && !stop.load(Ordering::Relaxed)
    {
        let step = scroll_input::post_scroll_smooth(direction, lines)
            .map_err(CaptureError::Tool)
            .and_then(|()| {
                std::thread::sleep(SETTLE);
                grab(region, &frame_path)
            });
        let frame = match step {
            Ok(frame) => frame,
            // Keep the partial composite once it has real content.
            Err(err) if frames >= 2 => {
                eprintln!("scrolling capture step failed, keeping partial result: {err}");
                break;
            }
            Err(err) => {
                let _ = std::fs::remove_file(&frame_path);
                return Err(err);
            }
        };
        let frame_n = stitch::normalize(frame, direction);
        let offset = match stitch::find_scroll_offset(&prev_n, &frame_n) {
            // No real movement (only scrollbar-fade/caret noise): the end
            // of the scrollable content.
            Some(0) if stitch::frames_similar(&prev_n, &frame_n) => break,
            // Fixed header pinned the match at 0, or nothing matched: trust
            // the nominal scroll distance instead.
            Some(0) | None => nominal_px,
            Some(offset) => offset,
        };
        composite.append_rows(&frame_n, offset);
        prev_n = frame_n;
        frames += 1;
        progress(frames);
    }

    let _ = std::fs::remove_file(&frame_path);
    Ok(stitch::denormalize(composite.into_image(), direction))
}

/// Silent region grab. Unlike interactive modes, -R always writes a file on
/// success, so a missing/broken file is an error, not a cancel.
fn grab(region: &ScrollRegion, dest: &Path) -> Result<RgbaImage, CaptureError> {
    // screencapture -R wants integer values; pass the rect rounded.
    let rect = format!(
        "{},{},{},{}",
        region.x.round() as i64,
        region.y.round() as i64,
        region.width.round() as i64,
        region.height.round() as i64
    );
    super::run_screencapture(&["-x", "-t", "png", "-R", &rect], dest)?;
    image::open(dest)
        .map(|img| img.to_rgba8())
        .map_err(|err| CaptureError::Tool(format!("could not decode frame: {err}")))
}

#[cfg(test)]
mod tests {
    use super::super::stitch;
    use super::{nominal_step_px, step_lines};

    #[test]
    fn step_lines_scales_and_clamps() {
        assert_eq!(step_lines(400.0, 400), 8); // 40% of 400 pt / 10 pt-per-line, capped
        assert_eq!(step_lines(150.0, 150), 4); // 6 targeted, capped to the search window
        assert_eq!(step_lines(20.0, 20), 1); // floor(0.8) clamped up
        assert_eq!(step_lines(5.0, 5), 1); // floor(0.2) clamped up to 1
        assert_eq!(step_lines(1000.0, 1000), 8); // MAX_STEP_LINES cap
        assert_eq!(step_lines(1000.0, 2000), 8); // retina px don't change the cap
    }

    /// The invariant that broke once (commit b2734ee): the commanded step
    /// must never exceed what `stitch::find_scroll_offset` can detect, or
    /// every frame silently falls back to the nominal offset.
    #[test]
    fn commanded_step_stays_detectable() {
        // Representative (region points, frame px) pairs at 1x and 2x scale.
        for (points, px) in [
            (150.0, 150),
            (200.0, 200),
            (400.0, 400),
            (900.0, 900),
            (200.0, 400),
            (400.0, 800),
            (900.0, 1800),
        ] {
            let lines = step_lines(points, px);
            assert!(
                nominal_step_px(lines, points, px) <= stitch::max_detectable_offset(px),
                "step for {points} pt / {px} px outruns the offset search window"
            );
        }
    }
}
