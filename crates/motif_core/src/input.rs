//! Input event types and state tracking.
//!
//! Provides motif-native input types with logical coordinates,
//! translating from winit's physical-pixel events.

use crate::Point;
use std::collections::HashSet;

// Re-export winit keyboard types (well-designed, handles international layouts)
pub use winit::event::ElementState;
pub use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// Scroll delta from mouse wheel or trackpad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    /// Discrete lines (mouse wheel clicks).
    Lines { x: f32, y: f32 },
    /// Continuous pixels (trackpad gestures).
    Pixels { x: f32, y: f32 },
}

/// What kind of mouse event occurred.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEventKind {
    /// Cursor moved to a new position.
    Move,
    /// Mouse button pressed.
    Down,
    /// Mouse button released.
    Up,
    /// Scroll wheel or trackpad scroll.
    Scroll { delta: ScrollDelta },
    /// Cursor entered the window.
    Enter,
    /// Cursor left the window.
    Leave,
}

/// A mouse event with logical coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseEvent {
    /// What happened.
    pub kind: MouseEventKind,
    /// Cursor position in logical pixels (None for Enter/Leave).
    pub position: Option<Point>,
    /// Which button (for Down/Up events).
    pub button: Option<MouseButton>,
    /// Modifier keys held during this event.
    pub modifiers: ModifiersState,
}

/// A keyboard event.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyEvent {
    /// Logical key (what the user typed, layout-dependent).
    pub key: Key,
    /// Physical key (keyboard position, layout-independent).
    pub physical_key: PhysicalKey,
    /// Press or release.
    pub state: ElementState,
    /// Modifier keys held during this event.
    pub modifiers: ModifiersState,
}

/// Unified input event.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    Mouse(MouseEvent),
    Key(KeyEvent),
    /// Modifier keys changed (without a key press).
    ModifiersChanged(ModifiersState),
}

/// Tracks current input state for a window.
#[derive(Debug, Default)]
pub struct InputState {
    /// Current cursor position in logical pixels. None if outside window.
    pub cursor_position: Option<Point>,
    /// Currently pressed mouse buttons.
    pub mouse_buttons: HashSet<MouseButton>,
    /// Current modifier key state.
    pub modifiers: ModifiersState,
    /// Events queued this frame.
    events: Vec<InputEvent>,
}

impl InputState {
    /// Create new input state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue an event.
    pub fn push_event(&mut self, event: InputEvent) {
        self.events.push(event);
    }

    /// Drain all queued events.
    pub fn take_events(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.events)
    }

    /// Number of queued events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Handle cursor moved. Takes physical coordinates and scale factor.
    pub fn handle_cursor_moved(&mut self, physical_x: f64, physical_y: f64, scale: f32) {
        let logical = Point::new(
            physical_x as f32 / scale,
            physical_y as f32 / scale,
        );
        self.cursor_position = Some(logical);
        self.push_event(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Move,
            position: Some(logical),
            button: None,
            modifiers: self.modifiers,
        }));
    }

    /// Handle cursor entering the window.
    pub fn handle_cursor_entered(&mut self) {
        self.push_event(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Enter,
            position: self.cursor_position,
            button: None,
            modifiers: self.modifiers,
        }));
    }

    /// Handle cursor leaving the window.
    pub fn handle_cursor_left(&mut self) {
        self.cursor_position = None;
        self.push_event(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Leave,
            position: None,
            button: None,
            modifiers: self.modifiers,
        }));
    }

    /// Handle mouse button press/release.
    pub fn handle_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        if pressed {
            self.mouse_buttons.insert(button);
        } else {
            self.mouse_buttons.remove(&button);
        }

        let kind = if pressed { MouseEventKind::Down } else { MouseEventKind::Up };
        self.push_event(InputEvent::Mouse(MouseEvent {
            kind,
            position: self.cursor_position,
            button: Some(button),
            modifiers: self.modifiers,
        }));
    }

    /// Handle scroll wheel or trackpad scroll.
    pub fn handle_scroll(&mut self, delta: ScrollDelta) {
        self.push_event(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Scroll { delta },
            position: self.cursor_position,
            button: None,
            modifiers: self.modifiers,
        }));
    }

    /// Handle modifier keys changed.
    pub fn handle_modifiers_changed(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
        self.push_event(InputEvent::ModifiersChanged(modifiers));
    }

    /// Handle a keyboard event.
    pub fn handle_key(&mut self, key: Key, physical_key: PhysicalKey, state: ElementState) {
        self.push_event(InputEvent::Key(KeyEvent {
            key,
            physical_key,
            state,
            modifiers: self.modifiers,
        }));
    }
}

impl MouseButton {
    /// Convert from winit mouse button.
    pub fn from_winit(button: winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward,
            winit::event::MouseButton::Other(id) => MouseButton::Other(id),
        }
    }
}

impl ScrollDelta {
    /// Convert from winit scroll delta, scaling to logical pixels.
    pub fn from_winit(delta: winit::event::MouseScrollDelta, scale: f32) -> Self {
        match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                ScrollDelta::Lines { x, y }
            }
            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                // Convert physical pixels to logical
                ScrollDelta::Pixels {
                    x: pos.x as f32 / scale,
                    y: pos.y as f32 / scale,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_state_starts_empty() {
        let state = InputState::new();
        assert!(state.cursor_position.is_none());
        assert!(state.mouse_buttons.is_empty());
        assert_eq!(state.modifiers, ModifiersState::empty());
        assert_eq!(state.event_count(), 0);
    }

    #[test]
    fn push_and_take_events() {
        let mut state = InputState::new();

        let event = InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Move,
            position: Some(Point::new(100.0, 200.0)),
            button: None,
            modifiers: ModifiersState::empty(),
        });

        state.push_event(event.clone());
        assert_eq!(state.event_count(), 1);

        let events = state.take_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
        assert_eq!(state.event_count(), 0);
    }

    #[test]
    fn mouse_button_equality() {
        assert_eq!(MouseButton::Left, MouseButton::Left);
        assert_ne!(MouseButton::Left, MouseButton::Right);
        assert_eq!(MouseButton::Other(4), MouseButton::Other(4));
        assert_ne!(MouseButton::Other(4), MouseButton::Other(5));
    }

    #[test]
    fn scroll_delta_variants() {
        let lines = ScrollDelta::Lines { x: 0.0, y: -3.0 };
        let pixels = ScrollDelta::Pixels { x: 0.0, y: -120.0 };

        match lines {
            ScrollDelta::Lines { y, .. } => assert_eq!(y, -3.0),
            _ => panic!("expected Lines"),
        }

        match pixels {
            ScrollDelta::Pixels { y, .. } => assert_eq!(y, -120.0),
            _ => panic!("expected Pixels"),
        }
    }

    #[test]
    fn mouse_buttons_hashable() {
        let mut buttons = HashSet::new();
        buttons.insert(MouseButton::Left);
        buttons.insert(MouseButton::Right);
        buttons.insert(MouseButton::Left); // duplicate

        assert_eq!(buttons.len(), 2);
        assert!(buttons.contains(&MouseButton::Left));
        assert!(buttons.contains(&MouseButton::Right));
        assert!(!buttons.contains(&MouseButton::Middle));
    }

    #[test]
    fn mouse_button_from_winit() {
        use winit::event::MouseButton as WinitButton;

        assert_eq!(MouseButton::from_winit(WinitButton::Left), MouseButton::Left);
        assert_eq!(MouseButton::from_winit(WinitButton::Right), MouseButton::Right);
        assert_eq!(MouseButton::from_winit(WinitButton::Middle), MouseButton::Middle);
        assert_eq!(MouseButton::from_winit(WinitButton::Back), MouseButton::Back);
        assert_eq!(MouseButton::from_winit(WinitButton::Forward), MouseButton::Forward);
        assert_eq!(MouseButton::from_winit(WinitButton::Other(42)), MouseButton::Other(42));
    }

    #[test]
    fn scroll_delta_from_winit_lines() {
        use winit::event::MouseScrollDelta;

        let winit_delta = MouseScrollDelta::LineDelta(1.0, -2.0);
        let delta = ScrollDelta::from_winit(winit_delta, 2.0); // scale doesn't affect lines

        match delta {
            ScrollDelta::Lines { x, y } => {
                assert_eq!(x, 1.0);
                assert_eq!(y, -2.0);
            }
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn scroll_delta_from_winit_pixels_scales() {
        use winit::dpi::PhysicalPosition;
        use winit::event::MouseScrollDelta;

        // Physical pixels at 2x scale
        let winit_delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(100.0, 200.0));
        let delta = ScrollDelta::from_winit(winit_delta, 2.0);

        match delta {
            ScrollDelta::Pixels { x, y } => {
                // Should be converted to logical pixels
                assert_eq!(x, 50.0);
                assert_eq!(y, 100.0);
            }
            _ => panic!("expected Pixels"),
        }
    }

    #[test]
    fn handle_cursor_moved_updates_position() {
        let mut state = InputState::new();

        // Physical position 200, 400 at 2x scale = logical 100, 200
        state.handle_cursor_moved(200.0, 400.0, 2.0);

        assert_eq!(state.cursor_position, Some(Point::new(100.0, 200.0)));
        assert_eq!(state.event_count(), 1);

        let events = state.take_events();
        match &events[0] {
            InputEvent::Mouse(MouseEvent { kind: MouseEventKind::Move, position, .. }) => {
                assert_eq!(*position, Some(Point::new(100.0, 200.0)));
            }
            _ => panic!("expected mouse move event"),
        }
    }

    #[test]
    fn handle_cursor_left_clears_position() {
        let mut state = InputState::new();
        state.cursor_position = Some(Point::new(100.0, 100.0));

        state.handle_cursor_left();

        assert!(state.cursor_position.is_none());
        assert_eq!(state.event_count(), 1);

        let events = state.take_events();
        match &events[0] {
            InputEvent::Mouse(MouseEvent { kind: MouseEventKind::Leave, .. }) => {}
            _ => panic!("expected mouse leave event"),
        }
    }

    #[test]
    fn handle_mouse_button_tracks_pressed() {
        let mut state = InputState::new();
        state.cursor_position = Some(Point::new(50.0, 50.0));

        // Press left button
        state.handle_mouse_button(MouseButton::Left, true);
        assert!(state.mouse_buttons.contains(&MouseButton::Left));
        assert_eq!(state.event_count(), 1);

        // Release left button
        state.handle_mouse_button(MouseButton::Left, false);
        assert!(!state.mouse_buttons.contains(&MouseButton::Left));
        assert_eq!(state.event_count(), 2);

        let events = state.take_events();
        match &events[0] {
            InputEvent::Mouse(MouseEvent { kind: MouseEventKind::Down, button, .. }) => {
                assert_eq!(*button, Some(MouseButton::Left));
            }
            _ => panic!("expected mouse down event"),
        }
        match &events[1] {
            InputEvent::Mouse(MouseEvent { kind: MouseEventKind::Up, button, .. }) => {
                assert_eq!(*button, Some(MouseButton::Left));
            }
            _ => panic!("expected mouse up event"),
        }
    }

    #[test]
    fn handle_scroll_queues_event() {
        let mut state = InputState::new();
        state.cursor_position = Some(Point::new(50.0, 50.0));

        let delta = ScrollDelta::Lines { x: 0.0, y: -1.0 };
        state.handle_scroll(delta);

        assert_eq!(state.event_count(), 1);
        let events = state.take_events();
        match &events[0] {
            InputEvent::Mouse(MouseEvent { kind: MouseEventKind::Scroll { delta: d }, .. }) => {
                assert_eq!(*d, delta);
            }
            _ => panic!("expected scroll event"),
        }
    }

    #[test]
    fn handle_modifiers_changed_updates_state() {
        let mut state = InputState::new();
        assert_eq!(state.modifiers, ModifiersState::empty());

        let new_mods = ModifiersState::SHIFT;
        state.handle_modifiers_changed(new_mods);

        assert_eq!(state.modifiers, new_mods);
        assert_eq!(state.event_count(), 1);

        let events = state.take_events();
        match &events[0] {
            InputEvent::ModifiersChanged(mods) => {
                assert_eq!(*mods, new_mods);
            }
            _ => panic!("expected modifiers changed event"),
        }
    }

    #[test]
    fn handle_key_event_queues_event() {
        let mut state = InputState::new();
        state.modifiers = ModifiersState::CONTROL;

        state.handle_key(
            Key::Character("a".into()),
            PhysicalKey::Code(KeyCode::KeyA),
            ElementState::Pressed,
        );

        assert_eq!(state.event_count(), 1);
        let events = state.take_events();
        match &events[0] {
            InputEvent::Key(KeyEvent { key, physical_key, state: key_state, modifiers }) => {
                assert_eq!(*key, Key::Character("a".into()));
                assert_eq!(*physical_key, PhysicalKey::Code(KeyCode::KeyA));
                assert_eq!(*key_state, ElementState::Pressed);
                assert_eq!(*modifiers, ModifiersState::CONTROL);
            }
            _ => panic!("expected key event"),
        }
    }
}
