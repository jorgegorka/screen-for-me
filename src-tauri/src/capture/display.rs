use core_graphics::display::CGDisplay;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl DisplayBounds {
    fn contains(&self, (px, py): (f64, f64)) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    pub fn screencapture_rect(&self) -> String {
        format!(
            "{},{},{},{}",
            self.x.round() as i64,
            self.y.round() as i64,
            self.width.round() as i64,
            self.height.round() as i64
        )
    }
}

pub fn pick_display(displays: &[DisplayBounds], cursor: Option<(f64, f64)>) -> Option<usize> {
    cursor
        .and_then(|point| displays.iter().position(|d| d.contains(point)))
        .or(if displays.is_empty() { None } else { Some(0) })
}

pub fn cursor_location() -> Option<(f64, f64)> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let point = CGEvent::new(source).ok()?.location();
    Some((point.x, point.y))
}

pub fn active_displays() -> Vec<DisplayBounds> {
    CGDisplay::active_displays()
        .unwrap_or_default()
        .into_iter()
        .map(|id| {
            let rect = CGDisplay::new(id).bounds();
            DisplayBounds {
                x: rect.origin.x,
                y: rect.origin.y,
                width: rect.size.width,
                height: rect.size.height,
            }
        })
        .collect()
}

pub fn display_under_cursor() -> Option<DisplayBounds> {
    let displays = active_displays();
    pick_display(&displays, cursor_location()).map(|index| displays[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(x: f64, y: f64, width: f64, height: f64) -> DisplayBounds {
        DisplayBounds { x, y, width, height }
    }

    #[test]
    fn pick_display_prefers_the_one_under_the_cursor() {
        let displays = [display(0.0, 0.0, 1728.0, 1117.0), display(1728.0, -200.0, 2560.0, 1440.0)];
        assert_eq!(pick_display(&displays, Some((2000.0, 100.0))), Some(1));
        assert_eq!(pick_display(&displays, Some((10.0, 10.0))), Some(0));
    }

    #[test]
    fn pick_display_falls_back_to_the_first() {
        let displays = [display(0.0, 0.0, 1728.0, 1117.0), display(1728.0, 0.0, 2560.0, 1440.0)];
        assert_eq!(pick_display(&displays, Some((-50.0, 5000.0))), Some(0));
        assert_eq!(pick_display(&displays, None), Some(0));
        assert_eq!(pick_display(&[], Some((1.0, 1.0))), None);
    }

    #[test]
    fn screencapture_rect_is_rounded_global_points() {
        assert_eq!(
            display(-1920.0, 37.4, 1920.0, 1079.6).screencapture_rect(),
            "-1920,37,1920,1080"
        );
    }
}
