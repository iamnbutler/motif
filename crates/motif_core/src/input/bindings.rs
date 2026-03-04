//! Input keybindings for text editing.
//!
//! Provides platform-specific default keybindings that map keyboard events
//! to semantic input actions. Consumers can use the defaults or customize.

use winit::event::Modifiers;
use winit::keyboard::{Key, NamedKey};

/// Semantic actions for text input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    /// Insert the character from the key event.
    InsertCharacter,
    /// Insert a newline.
    InsertNewline,
    /// Insert a tab.
    InsertTab,

    // Navigation
    /// Move cursor one grapheme left.
    Left,
    /// Move cursor one grapheme right.
    Right,
    /// Move cursor one word left.
    WordLeft,
    /// Move cursor one word right.
    WordRight,
    /// Move cursor to start of line.
    Home,
    /// Move cursor to end of line.
    End,
    /// Move cursor to start of content.
    MoveToBeginning,
    /// Move cursor to end of content.
    MoveToEnd,

    // Selection
    /// Extend selection one grapheme left.
    SelectLeft,
    /// Extend selection one grapheme right.
    SelectRight,
    /// Extend selection one word left.
    SelectWordLeft,
    /// Extend selection one word right.
    SelectWordRight,
    /// Extend selection to start of line.
    SelectHome,
    /// Extend selection to end of line.
    SelectEnd,
    /// Extend selection to start of content.
    SelectToBeginning,
    /// Extend selection to end of content.
    SelectToEnd,
    /// Select all content.
    SelectAll,

    // Deletion
    /// Delete character before cursor.
    Backspace,
    /// Delete character after cursor.
    Delete,
    /// Delete word before cursor.
    DeleteWordLeft,
    /// Delete word after cursor.
    DeleteWordRight,
    /// Delete to start of line.
    DeleteToBeginningOfLine,
    /// Delete to end of line.
    DeleteToEndOfLine,

    // Clipboard
    /// Copy selection to clipboard.
    Copy,
    /// Cut selection to clipboard.
    Cut,
    /// Paste from clipboard.
    Paste,

    // History
    /// Undo last edit.
    Undo,
    /// Redo last undone edit.
    Redo,

    // Focus
    /// Blur focus (typically Escape).
    Escape,
}

/// Maps keyboard events to input actions.
///
/// Provides platform-specific defaults for macOS vs other platforms.
#[derive(Debug, Clone, Default)]
pub struct InputBindings {
    // This struct exists for future customization.
    // Currently we just use the static platform defaults.
    _private: (),
}

impl InputBindings {
    /// Creates default platform-specific bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Maps a key event to an input action.
    ///
    /// Returns `None` if the key event doesn't map to any action.
    pub fn action_for_key(&self, key: &Key, modifiers: &Modifiers) -> Option<InputAction> {
        let shift = modifiers.state().shift_key();
        let ctrl = modifiers.state().control_key();
        let alt = modifiers.state().alt_key();
        let cmd = modifiers.state().super_key();

        // Platform-specific modifier for word operations
        #[cfg(target_os = "macos")]
        let word_mod = alt;
        #[cfg(not(target_os = "macos"))]
        let word_mod = ctrl;

        // Platform-specific modifier for command operations (select all, undo, etc.)
        #[cfg(target_os = "macos")]
        let cmd_mod = cmd;
        #[cfg(not(target_os = "macos"))]
        let cmd_mod = ctrl;

        match key {
            // Named keys
            Key::Named(named) => match named {
                NamedKey::Backspace => {
                    if cmd_mod && cfg!(target_os = "macos") {
                        Some(InputAction::DeleteToBeginningOfLine)
                    } else if word_mod {
                        Some(InputAction::DeleteWordLeft)
                    } else {
                        Some(InputAction::Backspace)
                    }
                }
                NamedKey::Delete => {
                    if word_mod {
                        Some(InputAction::DeleteWordRight)
                    } else {
                        Some(InputAction::Delete)
                    }
                }
                NamedKey::ArrowLeft => {
                    if shift && word_mod {
                        Some(InputAction::SelectWordLeft)
                    } else if shift && cmd_mod && cfg!(target_os = "macos") {
                        Some(InputAction::SelectHome)
                    } else if shift {
                        Some(InputAction::SelectLeft)
                    } else if word_mod {
                        Some(InputAction::WordLeft)
                    } else if cmd_mod && cfg!(target_os = "macos") {
                        Some(InputAction::Home)
                    } else {
                        Some(InputAction::Left)
                    }
                }
                NamedKey::ArrowRight => {
                    if shift && word_mod {
                        Some(InputAction::SelectWordRight)
                    } else if shift && cmd_mod && cfg!(target_os = "macos") {
                        Some(InputAction::SelectEnd)
                    } else if shift {
                        Some(InputAction::SelectRight)
                    } else if word_mod {
                        Some(InputAction::WordRight)
                    } else if cmd_mod && cfg!(target_os = "macos") {
                        Some(InputAction::End)
                    } else {
                        Some(InputAction::Right)
                    }
                }
                NamedKey::Home => {
                    if shift && cmd_mod {
                        Some(InputAction::SelectToBeginning)
                    } else if shift {
                        Some(InputAction::SelectHome)
                    } else if cmd_mod {
                        Some(InputAction::MoveToBeginning)
                    } else {
                        Some(InputAction::Home)
                    }
                }
                NamedKey::End => {
                    if shift && cmd_mod {
                        Some(InputAction::SelectToEnd)
                    } else if shift {
                        Some(InputAction::SelectEnd)
                    } else if cmd_mod {
                        Some(InputAction::MoveToEnd)
                    } else {
                        Some(InputAction::End)
                    }
                }
                NamedKey::Enter => Some(InputAction::InsertNewline),
                NamedKey::Tab => Some(InputAction::InsertTab),
                NamedKey::Escape => Some(InputAction::Escape),
                NamedKey::Space => Some(InputAction::InsertCharacter),
                _ => None,
            },

            // Character keys
            Key::Character(c) => {
                let c_lower = c.to_ascii_lowercase();
                if cmd_mod {
                    match c_lower.as_str() {
                        "a" => Some(InputAction::SelectAll),
                        "z" => {
                            if shift {
                                Some(InputAction::Redo)
                            } else {
                                Some(InputAction::Undo)
                            }
                        }
                        "x" => Some(InputAction::Cut),
                        "c" => Some(InputAction::Copy),
                        "v" => Some(InputAction::Paste),
                        _ => None,
                    }
                } else if ctrl && cfg!(target_os = "macos") {
                    // macOS Emacs-style bindings
                    match c_lower.as_str() {
                        "k" => Some(InputAction::DeleteToEndOfLine),
                        _ => Some(InputAction::InsertCharacter),
                    }
                } else {
                    Some(InputAction::InsertCharacter)
                }
            }

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event::Modifiers;
    use winit::keyboard::{Key, ModifiersState, NamedKey};

    fn mods(shift: bool, ctrl: bool, alt: bool, cmd: bool) -> Modifiers {
        let mut state = ModifiersState::empty();
        if shift {
            state |= ModifiersState::SHIFT;
        }
        if ctrl {
            state |= ModifiersState::CONTROL;
        }
        if alt {
            state |= ModifiersState::ALT;
        }
        if cmd {
            state |= ModifiersState::SUPER;
        }
        Modifiers::from(state)
    }

    fn no_mods() -> Modifiers {
        mods(false, false, false, false)
    }

    fn shift() -> Modifiers {
        mods(true, false, false, false)
    }

    #[cfg(target_os = "macos")]
    fn word_mod() -> Modifiers {
        mods(false, false, true, false) // Alt on macOS
    }

    #[cfg(not(target_os = "macos"))]
    fn word_mod() -> Modifiers {
        mods(false, true, false, false) // Ctrl on other platforms
    }

    #[cfg(target_os = "macos")]
    fn cmd() -> Modifiers {
        mods(false, false, false, true) // Super on macOS
    }

    #[cfg(not(target_os = "macos"))]
    fn cmd() -> Modifiers {
        mods(false, true, false, false) // Ctrl on other platforms
    }

    #[cfg(target_os = "macos")]
    fn cmd_shift() -> Modifiers {
        mods(true, false, false, true)
    }

    #[cfg(not(target_os = "macos"))]
    fn cmd_shift() -> Modifiers {
        mods(true, true, false, false)
    }

    #[test]
    fn basic_arrow_keys() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::ArrowLeft), &no_mods()),
            Some(InputAction::Left)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::ArrowRight), &no_mods()),
            Some(InputAction::Right)
        );
    }

    #[test]
    fn shift_arrow_selects() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::ArrowLeft), &shift()),
            Some(InputAction::SelectLeft)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::ArrowRight), &shift()),
            Some(InputAction::SelectRight)
        );
    }

    #[test]
    fn word_navigation() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::ArrowLeft), &word_mod()),
            Some(InputAction::WordLeft)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::ArrowRight), &word_mod()),
            Some(InputAction::WordRight)
        );
    }

    #[test]
    fn backspace_and_delete() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Backspace), &no_mods()),
            Some(InputAction::Backspace)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Delete), &no_mods()),
            Some(InputAction::Delete)
        );
    }

    #[test]
    fn word_deletion() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Backspace), &word_mod()),
            Some(InputAction::DeleteWordLeft)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Delete), &word_mod()),
            Some(InputAction::DeleteWordRight)
        );
    }

    #[test]
    fn select_all() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Character("a".into()), &cmd()),
            Some(InputAction::SelectAll)
        );
    }

    #[test]
    fn undo_redo() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Character("z".into()), &cmd()),
            Some(InputAction::Undo)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Character("z".into()), &cmd_shift()),
            Some(InputAction::Redo)
        );
    }

    #[test]
    fn clipboard_operations() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Character("c".into()), &cmd()),
            Some(InputAction::Copy)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Character("x".into()), &cmd()),
            Some(InputAction::Cut)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Character("v".into()), &cmd()),
            Some(InputAction::Paste)
        );
    }

    #[test]
    fn character_input() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Character("a".into()), &no_mods()),
            Some(InputAction::InsertCharacter)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Character("Z".into()), &shift()),
            Some(InputAction::InsertCharacter)
        );
    }

    #[test]
    fn special_keys() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Enter), &no_mods()),
            Some(InputAction::InsertNewline)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Tab), &no_mods()),
            Some(InputAction::InsertTab)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Escape), &no_mods()),
            Some(InputAction::Escape)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Space), &no_mods()),
            Some(InputAction::InsertCharacter)
        );
    }

    #[test]
    fn home_end() {
        let bindings = InputBindings::new();

        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Home), &no_mods()),
            Some(InputAction::Home)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::End), &no_mods()),
            Some(InputAction::End)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::Home), &shift()),
            Some(InputAction::SelectHome)
        );
        assert_eq!(
            bindings.action_for_key(&Key::Named(NamedKey::End), &shift()),
            Some(InputAction::SelectEnd)
        );
    }
}
