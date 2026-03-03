//! OS-level input simulation using macOS CGEvent APIs.
//!
//! Simulates real mouse events that flow through the normal OS → window → app pipeline.

#[cfg(target_os = "macos")]
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
#[cfg(target_os = "macos")]
use core_graphics::geometry::CGPoint;

/// Result of an input simulation operation.
#[derive(Debug, Clone)]
pub struct SimResult {
    pub success: bool,
    pub message: String,
}

impl SimResult {
    pub fn ok(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            message: msg.into(),
        }
    }
}

/// Window position info needed for coordinate translation.
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowPosition {
    /// Window's top-left corner in screen coordinates.
    pub x: f32,
    pub y: f32,
    /// Scale factor for retina displays.
    pub scale: f32,
}

impl WindowPosition {
    /// Convert window-local logical coordinates to screen coordinates.
    pub fn to_screen(&self, local_x: f32, local_y: f32) -> (f64, f64) {
        // Window-local logical coords → screen coords
        // Note: macOS screen origin is top-left of primary display
        let screen_x = self.x + local_x;
        let screen_y = self.y + local_y;
        (screen_x as f64, screen_y as f64)
    }
}

/// Move the mouse cursor to a screen position.
#[cfg(target_os = "macos")]
pub fn move_mouse_to(screen_x: f64, screen_y: f64) -> SimResult {
    let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        Ok(s) => s,
        Err(_) => return SimResult::err("Failed to create event source"),
    };

    let point = CGPoint::new(screen_x, screen_y);

    let event = match CGEvent::new_mouse_event(
        source,
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Left, // Ignored for move events
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create mouse move event"),
    };

    event.post(CGEventTapLocation::HID);

    SimResult::ok(format!("Moved to ({:.1}, {:.1})", screen_x, screen_y))
}

/// Click (press and release) at a screen position.
#[cfg(target_os = "macos")]
pub fn click_at(screen_x: f64, screen_y: f64) -> SimResult {
    let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        Ok(s) => s,
        Err(_) => return SimResult::err("Failed to create event source"),
    };

    let point = CGPoint::new(screen_x, screen_y);

    // Mouse down
    let down_event = match CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDown,
        point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create mouse down event"),
    };

    // Mouse up
    let up_event = match CGEvent::new_mouse_event(
        source,
        CGEventType::LeftMouseUp,
        point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create mouse up event"),
    };

    // Post events
    down_event.post(CGEventTapLocation::HID);
    up_event.post(CGEventTapLocation::HID);

    SimResult::ok(format!("Clicked at ({:.1}, {:.1})", screen_x, screen_y))
}

/// Press mouse button at a screen position (without release).
#[cfg(target_os = "macos")]
pub fn mouse_down_at(screen_x: f64, screen_y: f64) -> SimResult {
    let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        Ok(s) => s,
        Err(_) => return SimResult::err("Failed to create event source"),
    };

    let point = CGPoint::new(screen_x, screen_y);

    let event = match CGEvent::new_mouse_event(
        source,
        CGEventType::LeftMouseDown,
        point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create mouse down event"),
    };

    event.post(CGEventTapLocation::HID);

    SimResult::ok(format!("Mouse down at ({:.1}, {:.1})", screen_x, screen_y))
}

/// Release mouse button at a screen position.
#[cfg(target_os = "macos")]
pub fn mouse_up_at(screen_x: f64, screen_y: f64) -> SimResult {
    let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        Ok(s) => s,
        Err(_) => return SimResult::err("Failed to create event source"),
    };

    let point = CGPoint::new(screen_x, screen_y);

    let event = match CGEvent::new_mouse_event(
        source,
        CGEventType::LeftMouseUp,
        point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create mouse up event"),
    };

    event.post(CGEventTapLocation::HID);

    SimResult::ok(format!("Mouse up at ({:.1}, {:.1})", screen_x, screen_y))
}

/// Drag from one position to another.
#[cfg(target_os = "macos")]
pub fn drag(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> SimResult {
    let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        Ok(s) => s,
        Err(_) => return SimResult::err("Failed to create event source"),
    };

    // Move to start
    let start_point = CGPoint::new(from_x, from_y);
    let move_event = match CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::MouseMoved,
        start_point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create move event"),
    };
    move_event.post(CGEventTapLocation::HID);

    // Mouse down
    let down_event = match CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDown,
        start_point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create mouse down event"),
    };
    down_event.post(CGEventTapLocation::HID);

    // Drag to end
    let end_point = CGPoint::new(to_x, to_y);
    let drag_event = match CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDragged,
        end_point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create drag event"),
    };
    drag_event.post(CGEventTapLocation::HID);

    // Mouse up
    let up_event = match CGEvent::new_mouse_event(
        source,
        CGEventType::LeftMouseUp,
        end_point,
        CGMouseButton::Left,
    ) {
        Ok(e) => e,
        Err(_) => return SimResult::err("Failed to create mouse up event"),
    };
    up_event.post(CGEventTapLocation::HID);

    SimResult::ok(format!(
        "Dragged from ({:.1}, {:.1}) to ({:.1}, {:.1})",
        from_x, from_y, to_x, to_y
    ))
}

// Stub implementations for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn move_mouse_to(_screen_x: f64, _screen_y: f64) -> SimResult {
    SimResult::err("Input simulation only supported on macOS")
}

#[cfg(not(target_os = "macos"))]
pub fn click_at(_screen_x: f64, _screen_y: f64) -> SimResult {
    SimResult::err("Input simulation only supported on macOS")
}

#[cfg(not(target_os = "macos"))]
pub fn mouse_down_at(_screen_x: f64, _screen_y: f64) -> SimResult {
    SimResult::err("Input simulation only supported on macOS")
}

#[cfg(not(target_os = "macos"))]
pub fn mouse_up_at(_screen_x: f64, _screen_y: f64) -> SimResult {
    SimResult::err("Input simulation only supported on macOS")
}

#[cfg(not(target_os = "macos"))]
pub fn drag(_from_x: f64, _from_y: f64, _to_x: f64, _to_y: f64) -> SimResult {
    SimResult::err("Input simulation only supported on macOS")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_position_to_screen() {
        let pos = WindowPosition {
            x: 100.0,
            y: 50.0,
            scale: 2.0,
        };

        let (sx, sy) = pos.to_screen(20.0, 30.0);
        assert_eq!(sx, 120.0);
        assert_eq!(sy, 80.0);
    }

    #[test]
    fn sim_result_ok() {
        let r = SimResult::ok("test");
        assert!(r.success);
        assert_eq!(r.message, "test");
    }

    #[test]
    fn sim_result_err() {
        let r = SimResult::err("failed");
        assert!(!r.success);
        assert_eq!(r.message, "failed");
    }
}
