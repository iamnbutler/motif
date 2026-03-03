# Input System Design

Foundation for capturing mouse and keyboard events from winit and exposing them in motif-native types.

## Goals

- Capture mouse and keyboard events from winit
- Expose events in motif-native types with logical coordinates
- Track current input state (cursor position, pressed buttons, modifiers)
- Make events observable via debug CLI
- Build toward: hit testing, focus management, element callbacks

## Core Types

### Mouse Events

```rust
// crates/motif_core/src/input.rs

/// Mouse event in logical coordinates (pre-scaled for DPI)
pub struct MouseEvent {
    /// Position relative to window, in logical pixels
    pub position: Point,
    /// Which mouse button (if applicable)
    pub button: MouseButton,
    /// Current modifier keys held
    pub modifiers: Modifiers,
}

/// What happened with the mouse
pub enum MouseEventKind {
    Move,
    Down,
    Up,
    Scroll { delta: ScrollDelta },
    Enter,
    Leave,
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

pub enum ScrollDelta {
    /// Discrete lines (mouse wheel)
    Lines { x: f32, y: f32 },
    /// Continuous pixels (trackpad)
    Pixels { x: f32, y: f32 },
}
```

### Keyboard Events

Re-export winit's keyboard types (well-designed, handles international layouts and IME):

```rust
pub use winit::keyboard::{Key, NamedKey, PhysicalKey, KeyCode, ModifiersState};

pub struct KeyEvent {
    /// Logical key (what the user typed, e.g. Key::Character("a"))
    pub key: Key,
    /// Physical key (keyboard position, e.g. KeyCode::KeyA)
    pub physical_key: PhysicalKey,
    /// Press or release
    pub state: ElementState,
    /// Current modifiers
    pub modifiers: Modifiers,
    /// Text produced (if any)
    pub text: Option<SmolStr>,
}
```

### Input State

```rust
/// Current input state for a window
pub struct InputState {
    /// Current cursor position (logical pixels), None if cursor outside window
    pub cursor_position: Option<Point>,
    /// Currently pressed mouse buttons
    pub mouse_buttons: HashSet<MouseButton>,
    /// Current modifier keys
    pub modifiers: Modifiers,
    /// Events this frame (cleared each frame)
    events: Vec<InputEvent>,
}

/// Unified input event enum
pub enum InputEvent {
    Mouse { kind: MouseEventKind, event: MouseEvent },
    Key(KeyEvent),
}

impl InputState {
    pub fn new() -> Self { ... }

    /// Called by the windowing layer to record an event
    pub fn push_event(&mut self, event: InputEvent) { ... }

    /// Drain events for processing (called once per frame)
    pub fn take_events(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.events)
    }

    /// Update state from winit event (physical → logical conversion)
    pub fn handle_winit_event(&mut self, event: &WindowEvent, scale: ScaleFactor) { ... }
}
```

## Event Flow

1. winit delivers `WindowEvent` in `window_event()`
2. Call `input_state.handle_winit_event(&event, scale)`
3. State is updated, event is queued
4. During render/update, `take_events()` to process them

## Implementation Sequence

### Step 1: Define input types

File: `motif_core/src/input.rs`

- `MouseButton`, `ScrollDelta`, `MouseEventKind`
- `MouseEvent`, `KeyEvent`, `InputEvent`
- `InputState` struct with basic state tracking
- Re-export winit keyboard types
- Tests: state updates correctly from synthetic events

### Step 2: Implement winit translation

Method: `InputState::handle_winit_event`

- Physical → logical coordinate conversion
- Map winit `MouseButton`, `ElementState`, `MouseScrollDelta`
- Track `cursor_position`, `mouse_buttons`, `modifiers`
- Queue events into the vec
- Tests: verify coordinate scaling, button tracking

### Step 3: Wire into playground

- Add `InputState` to `App`
- Call `handle_winit_event` in `window_event()`
- Add stub `handle_input()` that logs events
- Manual test: run playground, see events in console

### Step 4: Debug CLI integration

- Add `input.state` command to debug server protocol
- Add `InputStateSnapshot` to debug server (like `SceneSnapshot`)
- Implement CLI command
- Test: `motif-debug input.state` shows cursor position

## Out of Scope

- Hit testing (separate spool task)
- Focus management (separate spool task)
- Element callbacks (future work)
