//! System cursor style.
//!
//! Defines the platform cursor appearance that can be requested by UI elements
//! during hit registration. The application loop queries the cursor style for
//! the hovered element each frame and applies it via `window.set_cursor()`.

/// The cursor appearance when hovering over a UI element.
///
/// Elements declare their preferred cursor when registering for hit testing via
/// [`PaintContext::register_hit_with_cursor`]. The application loop reads the
/// cursor for the topmost hovered element and applies it to the window.
///
/// # Example
///
/// ```ignore
/// // In a button's paint() implementation:
/// cx.register_hit_with_cursor(self.id, bounds, CursorStyle::Pointer);
///
/// // In the window event handler (CursorMoved):
/// let cursor = hit_tree.cursor_at(pos).unwrap_or_default();
/// window.set_cursor(cursor.to_winit());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    /// Default arrow cursor. Used when no element or a non-interactive element is hovered.
    #[default]
    Default,
    /// Pointer hand for clickable elements (buttons, links, checkboxes).
    Pointer,
    /// Text I-beam for editable text fields and selectable text.
    Text,
    /// Crosshair for precision selection or drawing tools.
    Crosshair,
    /// Four-directional move cursor for draggable containers.
    Move,
    /// Not-allowed symbol for disabled elements.
    NotAllowed,
    /// Animated wait/loading cursor.
    Wait,
    /// Open hand for elements that can be grabbed to scroll or drag.
    Grab,
    /// Closed hand during an active drag operation.
    Grabbing,
}

impl CursorStyle {
    /// Convert to winit's [`CursorIcon`](winit::window::CursorIcon) for platform display.
    pub fn to_winit(self) -> winit::window::CursorIcon {
        match self {
            CursorStyle::Default => winit::window::CursorIcon::Default,
            CursorStyle::Pointer => winit::window::CursorIcon::Pointer,
            CursorStyle::Text => winit::window::CursorIcon::Text,
            CursorStyle::Crosshair => winit::window::CursorIcon::Crosshair,
            CursorStyle::Move => winit::window::CursorIcon::Move,
            CursorStyle::NotAllowed => winit::window::CursorIcon::NotAllowed,
            CursorStyle::Wait => winit::window::CursorIcon::Wait,
            CursorStyle::Grab => winit::window::CursorIcon::Grab,
            CursorStyle::Grabbing => winit::window::CursorIcon::Grabbing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cursor_is_default_variant() {
        assert_eq!(CursorStyle::default(), CursorStyle::Default);
    }

    #[test]
    fn to_winit_default() {
        assert_eq!(
            CursorStyle::Default.to_winit(),
            winit::window::CursorIcon::Default
        );
    }

    #[test]
    fn to_winit_pointer() {
        assert_eq!(
            CursorStyle::Pointer.to_winit(),
            winit::window::CursorIcon::Pointer
        );
    }

    #[test]
    fn to_winit_text() {
        assert_eq!(
            CursorStyle::Text.to_winit(),
            winit::window::CursorIcon::Text
        );
    }

    #[test]
    fn to_winit_crosshair() {
        assert_eq!(
            CursorStyle::Crosshair.to_winit(),
            winit::window::CursorIcon::Crosshair
        );
    }

    #[test]
    fn to_winit_move() {
        assert_eq!(
            CursorStyle::Move.to_winit(),
            winit::window::CursorIcon::Move
        );
    }

    #[test]
    fn to_winit_not_allowed() {
        assert_eq!(
            CursorStyle::NotAllowed.to_winit(),
            winit::window::CursorIcon::NotAllowed
        );
    }

    #[test]
    fn to_winit_wait() {
        assert_eq!(
            CursorStyle::Wait.to_winit(),
            winit::window::CursorIcon::Wait
        );
    }

    #[test]
    fn to_winit_grab() {
        assert_eq!(
            CursorStyle::Grab.to_winit(),
            winit::window::CursorIcon::Grab
        );
    }

    #[test]
    fn to_winit_grabbing() {
        assert_eq!(
            CursorStyle::Grabbing.to_winit(),
            winit::window::CursorIcon::Grabbing
        );
    }

    #[test]
    fn cursor_style_is_copy() {
        let a = CursorStyle::Pointer;
        let b = a; // Copy trait
        assert_eq!(a, b);
    }

    #[test]
    fn cursor_style_variants_are_distinct() {
        assert_ne!(CursorStyle::Default, CursorStyle::Pointer);
        assert_ne!(CursorStyle::Pointer, CursorStyle::Text);
        assert_ne!(CursorStyle::Text, CursorStyle::Grab);
        assert_ne!(CursorStyle::Grab, CursorStyle::Grabbing);
    }
}
