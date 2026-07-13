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
const SCROLL_LINES: i32 = 5;
/// One wheel line scrolls ≈ 10 points; only used when correlation fails.
const NOMINAL_POINTS_PER_LINE: f64 = 10.0;
const SETTLE: Duration = Duration::from_millis(350);

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
    let nominal_px = ((SCROLL_LINES as f64 * NOMINAL_POINTS_PER_LINE) * axis_px as f64
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
        let step = scroll_input::post_scroll(direction, SCROLL_LINES)
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
