//! InputHandler trait — abstraction over text input receivers.
//!
//! Defines a common interface for components that accept text input, covering
//! both direct keyboard input and IME (Input Method Editor) composition.
//!
//! # IME composition flow
//!
//! When the user types with an IME (e.g. Japanese, Chinese, Korean input methods),
//! the OS sends a sequence of events:
//!
//! 1. `set_marked_text("候補", ...)` — one or more times as composition proceeds
//! 2. Either:
//!    - `commit("確定テキスト")` — committed text (replaces preedit)
//!    - `set_marked_text("", ...)` — cancelled (removes preedit)
//!
//! # Example
//!
//! ```rust,ignore
//! fn handle_ime(handler: &mut impl InputHandler, event: &winit::event::Ime) {
//!     match event {
//!         Ime::Preedit(text, _cursor) => handler.set_marked_text(text),
//!         Ime::Commit(text) => handler.commit(text),
//!         Ime::Enabled | Ime::Disabled => {}
//!     }
//! }
//! ```

use std::ops::Range;

use winit::event::Modifiers;
use winit::keyboard::Key;

use super::text_state::HandleKeyResult;
use super::text_state::TextEditState;

/// Abstraction for components that receive text input.
///
/// Implementors handle both direct keyboard events and platform IME events.
/// [`TextEditState`] implements this trait, providing a ready-made text editor
/// that satisfies the contract.
pub trait InputHandler {
    /// Handle a keyboard event, returning what the caller should do next.
    fn handle_key_event(&mut self, key: &Key, modifiers: &Modifiers) -> HandleKeyResult;

    /// Insert text directly at the cursor (non-IME or IME commit).
    ///
    /// Replaces any active selection or preedit range.
    fn commit(&mut self, text: &str);

    /// Replace the current preedit region with new IME composition text.
    ///
    /// If `preedit` is empty the composition is cancelled and the preedit
    /// text is removed from the buffer.
    fn set_marked_text(&mut self, preedit: &str);

    /// Clear any in-progress IME composition without committing.
    fn unmark_text(&mut self);

    /// Return the current text content.
    fn content(&self) -> &str;

    /// Return the current cursor position as a byte offset.
    fn cursor_offset(&self) -> usize;

    /// Return the current selection as a byte range.
    fn selected_range(&self) -> Range<usize>;

    /// Return the active IME composition range, if any.
    fn marked_range(&self) -> Option<Range<usize>>;
}

impl InputHandler for TextEditState {
    fn handle_key_event(&mut self, key: &Key, modifiers: &Modifiers) -> HandleKeyResult {
        self.handle_key_event(key, modifiers)
    }

    fn commit(&mut self, text: &str) {
        self.insert_text(text);
    }

    fn set_marked_text(&mut self, preedit: &str) {
        self.set_marked_text(preedit);
    }

    fn unmark_text(&mut self) {
        self.clear_marked_range();
    }

    fn content(&self) -> &str {
        self.content()
    }

    fn cursor_offset(&self) -> usize {
        self.cursor_offset()
    }

    fn selected_range(&self) -> Range<usize> {
        self.selected_range().clone()
    }

    fn marked_range(&self) -> Option<Range<usize>> {
        self.marked_range().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> impl InputHandler {
        TextEditState::new()
    }

    #[test]
    fn trait_content_returns_empty_initially() {
        let h = make_handler();
        assert_eq!(h.content(), "");
    }

    #[test]
    fn trait_commit_inserts_text() {
        let mut h = make_handler();
        h.commit("hello");
        assert_eq!(h.content(), "hello");
        assert_eq!(h.cursor_offset(), 5);
    }

    #[test]
    fn trait_set_marked_text_tracks_preedit() {
        let mut h = make_handler();
        h.commit("hi");
        h.set_marked_text("abc");
        assert_eq!(h.content(), "hiabc");
        assert_eq!(h.marked_range(), Some(2..5));
        assert!(h.selected_range().is_empty());
    }

    #[test]
    fn trait_set_marked_text_empty_cancels_preedit() {
        let mut h = make_handler();
        h.commit("hi");
        h.set_marked_text("xyz");
        h.set_marked_text("");
        assert_eq!(h.content(), "hi");
        assert_eq!(h.marked_range(), None);
    }

    #[test]
    fn trait_unmark_clears_mark_without_removing_text() {
        // Use TextEditState directly to verify unmark_text via InputHandler
        let mut state = TextEditState::new();
        state.set_content("hi");
        // Put text in a marked range via the struct method
        state.set_marked_range(0..2);
        assert!(state.marked_range().is_some());
        // Now call unmark_text through the trait
        InputHandler::unmark_text(&mut state);
        assert_eq!(state.marked_range(), None);
        // Text should be unchanged
        assert_eq!(state.content(), "hi");
    }

    #[test]
    fn trait_commit_after_preedit_replaces_marked_range() {
        let mut h = make_handler();
        h.commit("hello ");
        h.set_marked_text("wor");
        // commit replaces the preedit
        h.commit("world");
        assert_eq!(h.content(), "hello world");
        assert_eq!(h.marked_range(), None);
    }
}
