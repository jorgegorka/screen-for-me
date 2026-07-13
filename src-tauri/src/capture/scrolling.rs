//! Auto-scroll capture loop: grab the region, scroll one step, stitch, repeat
//! until the frames stop changing, a cap is hit, or the user stops it.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use image::RgbaImage;

use super::stitch::{self, ScrollDirection};
use super::{scroll_input, CaptureError};

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
/// Fraction of the region's scroll-axis extent one step may cover. Must stay
/// below 0.5: `stitch::most_textured_strip` may pick a strip starting at h/2,
/// leaving only `h/2 - STRIP_ROWS` of search range for the offset — a step
/// larger than that window becomes undetectable and falls back to nominal.
const STEP_EXTENT_FRACTION: f64 = 0.4;
const SETTLE: Duration = Duration::from_millis(200);

/// Lines to scroll per frame for a region extending `axis_points` along the
/// scroll axis.
pub(crate) fn step_lines(axis_points: f64) -> i32 {
    ((axis_points * STEP_EXTENT_FRACTION / NOMINAL_POINTS_PER_LINE) as i32).clamp(1, MAX_STEP_LINES)
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
    let lines = step_lines(axis_points);
    let nominal_px = ((lines as f64 * NOMINAL_POINTS_PER_LINE) * axis_px as f64
        / axis_points.max(1.0))
    .round() as u32;

    let mut prev_n = stitch::normalize(&first, direction);
    let mut composite = prev_n.clone();
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
        let frame_n = stitch::normalize(&frame, direction);
        let offset = match stitch::find_scroll_offset(&prev_n, &frame_n) {
            // No real movement (only scrollbar-fade/caret noise): the end
            // of the scrollable content.
            Some(0) if stitch::frames_similar(&prev_n, &frame_n) => break,
            // Fixed header pinned the match at 0, or nothing matched: trust
            // the nominal scroll distance instead.
            Some(0) | None => nominal_px.clamp(1, frame_n.height().saturating_sub(1).max(1)),
            Some(offset) => offset,
        };
        composite = stitch::append_rows(composite, &frame_n, offset);
        prev_n = frame_n;
        frames += 1;
        progress(frames);
    }

    let _ = std::fs::remove_file(&frame_path);
    Ok(stitch::denormalize(composite, direction))
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
    let output = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-t", "png", "-R", &rect])
        .arg(dest)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CaptureError::Tool(if stderr.is_empty() {
            format!("screencapture exited with {}", output.status)
        } else {
            stderr
        }));
    }
    image::open(dest)
        .map(|img| img.to_rgba8())
        .map_err(|err| CaptureError::Tool(format!("could not decode frame: {err}")))
}

#[cfg(test)]
mod tests {
    use super::step_lines;

    #[test]
    fn step_lines_scales_and_clamps() {
        assert_eq!(step_lines(100.0), 4); // 40% of 100 pt / 10 pt-per-line
        assert_eq!(step_lines(50.0), 2);
        assert_eq!(step_lines(20.0), 1); // floor(0.8) clamped up
        assert_eq!(step_lines(5.0), 1); // floor(0.2) clamped up to 1
        assert_eq!(step_lines(1000.0), 8); // cap
    }
}
