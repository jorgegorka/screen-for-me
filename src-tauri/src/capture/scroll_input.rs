//! Synthetic scroll-wheel input and the Accessibility permission it needs.
//! Untestable side effects live here so the stitch module can stay pure.

use core_foundation::base::{Boolean, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::CFString;
use core_graphics::display::CGDisplay;
use core_graphics::event::{CGEvent, CGEventTapLocation, ScrollEventUnit};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

use super::stitch::ScrollDirection;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    // Apple declares this as returning `Boolean` (unsigned char, nonzero =
    // true); reading it through Rust `bool` would be UB for values other
    // than 0/1, so take the raw byte and compare.
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> Boolean;
}

/// True when the app may post synthetic events. Passing the prompt option
/// also registers the app in System Settings → Accessibility on first ask.
pub fn ensure_accessibility() -> bool {
    let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let options =
        CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) != 0 }
}

/// Move the pointer so scroll events route to the window under the rect
/// (the system dispatches wheel events by cursor position, not event location).
pub fn warp_cursor(x: f64, y: f64) {
    let _ = CGDisplay::warp_mouse_cursor_position(CGPoint::new(x, y));
}

/// One scroll step. LINE units scroll discretely (no trackpad inertia), which
/// keeps the settle delay short and the frame offsets stitchable.
pub fn post_scroll(direction: ScrollDirection, lines: i32) -> Result<(), String> {
    // Positive wheel1 scrolls toward the top of the page; positive wheel2
    // toward the left edge. "Down" means revealing content below → negative.
    let (vertical, horizontal) = match direction {
        ScrollDirection::Down => (-lines, 0),
        ScrollDirection::Up => (lines, 0),
        ScrollDirection::Right => (0, -lines),
        ScrollDirection::Left => (0, lines),
    };
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "failed to create CGEventSource".to_string())?;
    let event = CGEvent::new_scroll_event(source, ScrollEventUnit::LINE, 2, vertical, horizontal, 0)
        .map_err(|_| "failed to create scroll event".to_string())?;
    event.post(CGEventTapLocation::HID);
    Ok(())
}
