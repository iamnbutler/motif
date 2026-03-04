//! Text editing state model.
//!
//! `TextEditState` handles text content storage, cursor/selection management,
//! and text manipulation operations. This is the stateful model for text inputs,
//! separate from visual rendering.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Text editing state for input fields.
///
/// Manages:
/// - Text content storage
/// - Selection range with cursor direction
/// - Cursor movement and text manipulation
pub struct TextEditState {
    content: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
}

impl TextEditState {
    /// Creates a new empty text edit state.
    pub fn new() -> Self {
        Self {
            content: String::new(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
        }
    }

    // === Content accessors ===

    /// Returns the current text content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Sets the text content, resetting selection to the start.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
    }

    // === Selection accessors ===

    /// Returns the current selection range.
    pub fn selected_range(&self) -> &Range<usize> {
        &self.selected_range
    }

    /// Returns whether the selection is reversed (cursor at start).
    pub fn selection_reversed(&self) -> bool {
        self.selection_reversed
    }

    /// Returns the current cursor offset.
    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Returns the marked text range (for IME composition).
    pub fn marked_range(&self) -> Option<&Range<usize>> {
        self.marked_range.as_ref()
    }

    /// Sets the selection range, clamping to content length.
    pub fn set_selected_range(&mut self, range: Range<usize>) {
        let len = self.content.len();
        self.selected_range = range.start.min(len)..range.end.min(len);
        self.selection_reversed = false;
    }

    /// Sets whether the selection is reversed.
    pub fn set_selection_reversed(&mut self, reversed: bool) {
        self.selection_reversed = reversed;
    }

    // === Grapheme boundary navigation ===

    /// Returns the byte offset of the previous grapheme boundary.
    pub fn previous_boundary(&self, offset: usize) -> usize {
        if offset == 0 {
            return 0;
        }
        let clamped = offset.min(self.content.len());
        self.content[..clamped]
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .next_back()
            .unwrap_or(0)
    }

    /// Returns the byte offset of the next grapheme boundary.
    pub fn next_boundary(&self, offset: usize) -> usize {
        if offset >= self.content.len() {
            return self.content.len();
        }
        self.content[offset..]
            .grapheme_indices(true)
            .nth(1)
            .map(|(i, _)| offset + i)
            .unwrap_or(self.content.len())
    }

    // === Word boundary navigation ===

    /// Returns the byte offset of the previous word boundary.
    pub fn previous_word_boundary(&self, offset: usize) -> usize {
        if offset == 0 {
            return 0;
        }
        let clamped = offset.min(self.content.len());
        let text_before = &self.content[..clamped];

        // Find the last word start before offset
        let mut last_word_start = 0;
        for (idx, _) in text_before.unicode_word_indices() {
            if idx < offset {
                last_word_start = idx;
            }
        }

        // If we found a word and we're at its start, find the previous word
        if last_word_start > 0 || offset > 0 {
            // Check if we're right at the start of a word
            if last_word_start == offset
                || (last_word_start < offset
                    && text_before[last_word_start..]
                        .unicode_words()
                        .next()
                        .is_some())
            {
                return last_word_start;
            }
        }

        last_word_start
    }

    /// Returns the byte offset of the next word boundary.
    pub fn next_word_boundary(&self, offset: usize) -> usize {
        if offset >= self.content.len() {
            return self.content.len();
        }
        let text_after = &self.content[offset..];

        for (idx, word) in text_after.unicode_word_indices() {
            let word_end = offset + idx + word.len();
            if word_end > offset {
                return word_end;
            }
        }

        self.content.len()
    }

    /// Returns the word range (start, end) at the given offset.
    pub fn word_range_at(&self, offset: usize) -> (usize, usize) {
        let clamped = offset.min(self.content.len());

        for (idx, word) in self.content.unicode_word_indices() {
            let word_end = idx + word.len();
            // Position must be strictly inside the word (not at the end boundary)
            if clamped >= idx && clamped < word_end {
                return (idx, word_end);
            }
        }

        (clamped, clamped)
    }

    // === Line boundary navigation ===

    /// Returns the byte offset of the start of the line containing `offset`.
    pub fn find_line_start(&self, offset: usize) -> usize {
        let clamped = offset.min(self.content.len());
        self.content[..clamped]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0)
    }

    /// Returns the byte offset of the end of the line containing `offset`.
    pub fn find_line_end(&self, offset: usize) -> usize {
        let clamped = offset.min(self.content.len());
        self.content[clamped..]
            .find('\n')
            .map(|pos| clamped + pos)
            .unwrap_or(self.content.len())
    }

    // === Cursor movement ===

    /// Moves the cursor to the given offset, collapsing any selection.
    pub fn move_to(&mut self, offset: usize) {
        let clamped = offset.min(self.content.len());
        self.selected_range = clamped..clamped;
        self.selection_reversed = false;
    }

    /// Extends the selection to the given offset.
    pub fn select_to(&mut self, offset: usize) {
        let clamped = offset.min(self.content.len());

        // The anchor is the non-cursor end of the selection
        let anchor = if self.selection_reversed {
            self.selected_range.end
        } else {
            self.selected_range.start
        };

        if clamped < anchor {
            self.selected_range = clamped..anchor;
            self.selection_reversed = true;
        } else {
            self.selected_range = anchor..clamped;
            self.selection_reversed = false;
        }
    }

    /// Moves cursor left by one grapheme, or collapses selection to start.
    pub fn left(&mut self) {
        if self.selected_range.is_empty() {
            let new_pos = self.previous_boundary(self.cursor_offset());
            self.move_to(new_pos);
        } else {
            self.move_to(self.selected_range.start);
        }
    }

    /// Moves cursor right by one grapheme, or collapses selection to end.
    pub fn right(&mut self) {
        if self.selected_range.is_empty() {
            let new_pos = self.next_boundary(self.cursor_offset());
            self.move_to(new_pos);
        } else {
            self.move_to(self.selected_range.end);
        }
    }

    // === Text manipulation ===

    /// Inserts text at the cursor position, replacing any selection.
    pub fn insert_text(&mut self, text: &str) {
        let range = self
            .marked_range
            .clone()
            .unwrap_or(self.selected_range.clone());
        let range = range.start.min(self.content.len())..range.end.min(self.content.len());

        self.content.replace_range(range.clone(), text);
        self.selected_range = range.start + text.len()..range.start + text.len();
        self.marked_range = None;
    }

    /// Deletes the character before the cursor (backspace).
    pub fn delete_backward(&mut self) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if prev < self.cursor_offset() {
                self.select_to(prev);
            }
        }
        if !self.selected_range.is_empty() {
            self.insert_text("");
        }
    }

    /// Deletes the character after the cursor (delete key).
    pub fn delete_forward(&mut self) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if next > self.cursor_offset() {
                self.select_to(next);
            }
        }
        if !self.selected_range.is_empty() {
            self.insert_text("");
        }
    }
}

impl Default for TextEditState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Task: Design InputState struct and core data model
    // ============================================================

    #[test]
    fn new_creates_empty_state() {
        let state = TextEditState::new();
        assert_eq!(state.content(), "");
        assert_eq!(state.selected_range(), &(0..0));
        assert!(!state.selection_reversed());
        assert!(state.marked_range().is_none());
    }

    #[test]
    fn set_content_replaces_text() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn set_content_resets_selection_to_start() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        // After set_content, cursor should be at the start
        assert_eq!(state.selected_range(), &(0..0));
    }

    #[test]
    fn cursor_offset_returns_end_when_not_reversed() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..3);
        // When not reversed, cursor is at end of selection
        assert_eq!(state.cursor_offset(), 3);
    }

    #[test]
    fn cursor_offset_returns_start_when_reversed() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..3);
        state.set_selection_reversed(true);
        // When reversed, cursor is at start of selection
        assert_eq!(state.cursor_offset(), 1);
    }

    #[test]
    fn set_selected_range_clamps_to_content_length() {
        let mut state = TextEditState::new();
        state.set_content("hi"); // length 2
        state.set_selected_range(0..100);
        assert_eq!(state.selected_range(), &(0..2));
    }

    // ============================================================
    // Task: Implement grapheme boundary navigation
    // ============================================================

    #[test]
    fn previous_boundary_at_start_returns_zero() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        assert_eq!(state.previous_boundary(0), 0);
    }

    #[test]
    fn previous_boundary_moves_back_one_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        assert_eq!(state.previous_boundary(3), 2);
    }

    #[test]
    fn previous_boundary_handles_multi_byte_chars() {
        let mut state = TextEditState::new();
        state.set_content("héllo"); // é is 2 bytes (U+00E9)
                                    // "h" = 1 byte, "é" = 2 bytes, so offset 3 is after "hé"
        assert_eq!(state.previous_boundary(3), 1); // back to after "h"
    }

    #[test]
    fn previous_boundary_handles_emoji() {
        let mut state = TextEditState::new();
        state.set_content("a👍b"); // 👍 is 4 bytes
                                   // "a" = 1 byte, "👍" = 4 bytes, "b" = 1 byte
                                   // offset 5 is after "a👍"
        assert_eq!(state.previous_boundary(5), 1); // back to after "a"
    }

    #[test]
    fn next_boundary_at_end_returns_length() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        assert_eq!(state.next_boundary(5), 5);
    }

    #[test]
    fn next_boundary_moves_forward_one_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        assert_eq!(state.next_boundary(2), 3);
    }

    #[test]
    fn next_boundary_handles_multi_byte_chars() {
        let mut state = TextEditState::new();
        state.set_content("héllo"); // é is 2 bytes
                                    // offset 1 is after "h", next grapheme is "é" which ends at offset 3
        assert_eq!(state.next_boundary(1), 3);
    }

    #[test]
    fn next_boundary_handles_emoji() {
        let mut state = TextEditState::new();
        state.set_content("a👍b"); // 👍 is 4 bytes
                                   // offset 1 is after "a", next grapheme is "👍" which ends at offset 5
        assert_eq!(state.next_boundary(1), 5);
    }

    // ============================================================
    // Task: Implement word boundary navigation
    // ============================================================

    #[test]
    fn previous_word_boundary_at_start_returns_zero() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        assert_eq!(state.previous_word_boundary(0), 0);
    }

    #[test]
    fn previous_word_boundary_finds_word_start() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        // offset 8 is in "world", should go to start of "world" at 6
        assert_eq!(state.previous_word_boundary(8), 6);
    }

    #[test]
    fn previous_word_boundary_from_word_start_goes_to_previous_word() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        // offset 6 is start of "world", should go to start of "hello" at 0
        assert_eq!(state.previous_word_boundary(6), 0);
    }

    #[test]
    fn next_word_boundary_at_end_returns_length() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        assert_eq!(state.next_word_boundary(11), 11);
    }

    #[test]
    fn next_word_boundary_finds_word_end() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        // offset 2 is in "hello", should go to end of "hello" at 5
        assert_eq!(state.next_word_boundary(2), 5);
    }

    #[test]
    fn next_word_boundary_from_word_end_goes_to_next_word_end() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        // offset 5 is end of "hello", should go to end of "world" at 11
        assert_eq!(state.next_word_boundary(5), 11);
    }

    #[test]
    fn word_range_at_returns_word_boundaries() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        // offset 2 is in "hello"
        assert_eq!(state.word_range_at(2), (0, 5));
        // offset 8 is in "world"
        assert_eq!(state.word_range_at(8), (6, 11));
    }

    #[test]
    fn word_range_at_whitespace_returns_cursor_position() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        // offset 5 is at the space
        assert_eq!(state.word_range_at(5), (5, 5));
    }

    // ============================================================
    // Task: Implement line boundary navigation
    // ============================================================

    #[test]
    fn find_line_start_at_start_returns_zero() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        assert_eq!(state.find_line_start(0), 0);
    }

    #[test]
    fn find_line_start_within_first_line() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        assert_eq!(state.find_line_start(3), 0);
    }

    #[test]
    fn find_line_start_within_second_line() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        // offset 8 is in "world", line starts after newline at offset 6
        assert_eq!(state.find_line_start(8), 6);
    }

    #[test]
    fn find_line_end_at_end_returns_length() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        assert_eq!(state.find_line_end(11), 11);
    }

    #[test]
    fn find_line_end_within_first_line() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        // offset 3 is in "hello", line ends at offset 5 (before newline)
        assert_eq!(state.find_line_end(3), 5);
    }

    #[test]
    fn find_line_end_within_second_line() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        // offset 8 is in "world", line ends at 11
        assert_eq!(state.find_line_end(8), 11);
    }

    // ============================================================
    // Task: Implement move_to and select_to helpers
    // ============================================================

    #[test]
    fn move_to_collapses_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);
        state.move_to(2);
        assert_eq!(state.selected_range(), &(2..2));
        assert!(!state.selection_reversed());
    }

    #[test]
    fn move_to_clamps_to_content_length() {
        let mut state = TextEditState::new();
        state.set_content("hi");
        state.move_to(100);
        assert_eq!(state.selected_range(), &(2..2));
    }

    #[test]
    fn select_to_extends_selection_forward() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(1);
        state.select_to(4);
        assert_eq!(state.selected_range(), &(1..4));
        assert!(!state.selection_reversed());
    }

    #[test]
    fn select_to_extends_selection_backward() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(4);
        state.select_to(1);
        assert_eq!(state.selected_range(), &(1..4));
        assert!(state.selection_reversed());
    }

    #[test]
    fn select_to_continues_extending_in_same_direction() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(3);
        state.select_to(6);
        state.select_to(8);
        assert_eq!(state.selected_range(), &(3..8));
        assert!(!state.selection_reversed());
    }

    #[test]
    fn select_to_reverses_when_crossing_anchor() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(5);
        state.select_to(8);
        // Selection is 5..8, not reversed (cursor at 8)
        assert_eq!(state.selected_range(), &(5..8));
        assert!(!state.selection_reversed());

        // Now select backward past the anchor
        state.select_to(2);
        assert_eq!(state.selected_range(), &(2..5));
        assert!(state.selection_reversed());
    }

    // ============================================================
    // Task: Implement basic cursor movement actions (left/right)
    // ============================================================

    #[test]
    fn left_moves_cursor_by_one_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(3);
        state.left();
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn left_at_start_stays_at_start() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(0);
        state.left();
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn left_with_selection_collapses_to_start() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);
        state.left();
        assert_eq!(state.selected_range(), &(1..1));
    }

    #[test]
    fn right_moves_cursor_by_one_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(2);
        state.right();
        assert_eq!(state.cursor_offset(), 3);
    }

    #[test]
    fn right_at_end_stays_at_end() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.right();
        assert_eq!(state.cursor_offset(), 5);
    }

    #[test]
    fn right_with_selection_collapses_to_end() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);
        state.right();
        assert_eq!(state.selected_range(), &(4..4));
    }

    // ============================================================
    // Task: Implement insert_text and replace_text_in_range
    // ============================================================

    #[test]
    fn insert_text_at_cursor() {
        let mut state = TextEditState::new();
        state.set_content("hllo");
        state.move_to(1);
        state.insert_text("e");
        assert_eq!(state.content(), "hello");
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn insert_text_replaces_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);
        state.insert_text("i");
        assert_eq!(state.content(), "hio");
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn insert_text_at_end() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        assert_eq!(state.content(), "hello!");
        assert_eq!(state.cursor_offset(), 6);
    }

    #[test]
    fn insert_empty_text_deletes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);
        state.insert_text("");
        assert_eq!(state.content(), "ho");
        assert_eq!(state.cursor_offset(), 1);
    }

    // ============================================================
    // Task: Implement backspace and delete actions
    // ============================================================

    #[test]
    fn delete_backward_removes_previous_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(3);
        state.delete_backward();
        assert_eq!(state.content(), "helo");
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn delete_backward_at_start_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(0);
        state.delete_backward();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn delete_backward_removes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);
        state.delete_backward();
        assert_eq!(state.content(), "ho");
        assert_eq!(state.cursor_offset(), 1);
    }

    #[test]
    fn delete_forward_removes_next_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(2);
        state.delete_forward();
        assert_eq!(state.content(), "helo");
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn delete_forward_at_end_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.delete_forward();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn delete_forward_removes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);
        state.delete_forward();
        assert_eq!(state.content(), "ho");
        assert_eq!(state.cursor_offset(), 1);
    }

    #[test]
    fn delete_backward_handles_emoji() {
        let mut state = TextEditState::new();
        state.set_content("a👍b");
        state.move_to(5); // after emoji
        state.delete_backward();
        assert_eq!(state.content(), "ab");
    }

    #[test]
    fn delete_forward_handles_emoji() {
        let mut state = TextEditState::new();
        state.set_content("a👍b");
        state.move_to(1); // after "a"
        state.delete_forward();
        assert_eq!(state.content(), "ab");
    }
}
