//! Text editing state model.
//!
//! `TextEditState` handles text content storage, cursor/selection management,
//! and text manipulation operations. This is the stateful model for text inputs,
//! separate from visual rendering.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use winit::event::Modifiers;
use winit::keyboard::Key;

use super::bindings::{InputAction, InputBindings};

/// Result of handling a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleKeyResult {
    /// The key was handled and the input state was modified.
    Handled,
    /// The key was not recognized by the input bindings.
    NotHandled,
    /// The escape key was pressed - blur focus.
    Blur,
    /// Copy requested - contains the text to copy.
    Copy(String),
    /// Cut requested - contains the text that was cut.
    Cut(String),
    /// Paste requested - caller should provide clipboard content via `paste()`.
    Paste,
    /// Enter pressed in single-line mode - submit the input.
    Submit,
    /// Tab pressed in single-line mode - move focus to next input.
    FocusNext,
    /// Shift+Tab pressed in single-line mode - move focus to previous input.
    FocusPrev,
}

/// A history entry for undo/redo operations.
#[derive(Clone, Debug)]
struct HistoryEntry {
    /// The byte range that was modified.
    range: Range<usize>,
    /// The text that was replaced.
    old_text: String,
    /// The length of new text that replaced old_text.
    new_text_len: usize,
    /// Cursor position before the edit.
    cursor_before: usize,
}

/// Text editing state for input fields.
///
/// Manages:
/// - Text content storage
/// - Selection range with cursor direction
/// - Cursor movement and text manipulation
/// - Undo/redo history
pub struct TextEditState {
    content: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    /// Stack of previous edits for undo.
    undo_stack: Vec<HistoryEntry>,
    /// Stack of undone edits for redo.
    redo_stack: Vec<HistoryEntry>,
    /// Whether this is a multiline input (textarea) or single-line (input).
    /// Affects Enter (newline vs submit) and Tab (tab char vs focus change).
    multiline: bool,
}

impl TextEditState {
    /// Creates a new empty single-line text edit state.
    ///
    /// For single-line inputs:
    /// - Enter returns `HandleKeyResult::Submit`
    /// - Tab returns `HandleKeyResult::FocusNext`
    /// - Shift+Tab returns `HandleKeyResult::FocusPrev`
    pub fn new() -> Self {
        Self {
            content: String::new(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            multiline: false,
        }
    }

    /// Creates a new empty multiline text edit state.
    ///
    /// For multiline inputs (textareas):
    /// - Enter inserts a newline
    /// - Tab inserts a tab character
    pub fn new_multiline() -> Self {
        Self {
            content: String::new(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            multiline: true,
        }
    }

    /// Returns whether this is a multiline input.
    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    // === Content accessors ===

    /// Returns the current text content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Sets the text content, resetting selection and clearing history.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
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

    /// Moves cursor to the previous word boundary.
    pub fn word_left(&mut self) {
        let start = if self.selected_range.is_empty() {
            self.cursor_offset()
        } else {
            self.selected_range.start
        };
        let new_pos = self.previous_word_boundary(start);
        self.move_to(new_pos);
    }

    /// Moves cursor to the next word boundary.
    pub fn word_right(&mut self) {
        let start = if self.selected_range.is_empty() {
            self.cursor_offset()
        } else {
            self.selected_range.end
        };
        let new_pos = self.next_word_boundary(start);
        self.move_to(new_pos);
    }

    /// Moves cursor to the start of the current line.
    pub fn home(&mut self) {
        let start = if self.selected_range.is_empty() {
            self.cursor_offset()
        } else {
            self.selected_range.start
        };
        let new_pos = self.find_line_start(start);
        self.move_to(new_pos);
    }

    /// Moves cursor to the end of the current line.
    pub fn end(&mut self) {
        let start = if self.selected_range.is_empty() {
            self.cursor_offset()
        } else {
            self.selected_range.end
        };
        let new_pos = self.find_line_end(start);
        self.move_to(new_pos);
    }

    /// Moves cursor to the beginning of the content.
    pub fn move_to_beginning(&mut self) {
        self.move_to(0);
    }

    /// Moves cursor to the end of the content.
    pub fn move_to_end(&mut self) {
        self.move_to(self.content.len());
    }

    // === Selection extension ===

    /// Extends selection left by one grapheme.
    pub fn select_left(&mut self) {
        let new_pos = self.previous_boundary(self.cursor_offset());
        self.select_to(new_pos);
    }

    /// Extends selection right by one grapheme.
    pub fn select_right(&mut self) {
        let new_pos = self.next_boundary(self.cursor_offset());
        self.select_to(new_pos);
    }

    /// Extends selection to the previous word boundary.
    pub fn select_word_left(&mut self) {
        let new_pos = self.previous_word_boundary(self.cursor_offset());
        self.select_to(new_pos);
    }

    /// Extends selection to the next word boundary.
    pub fn select_word_right(&mut self) {
        let new_pos = self.next_word_boundary(self.cursor_offset());
        self.select_to(new_pos);
    }

    /// Selects all content.
    pub fn select_all(&mut self) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
    }

    /// Extends selection to the beginning of the content.
    pub fn select_to_beginning(&mut self) {
        self.select_to(0);
    }

    /// Extends selection to the end of the content.
    pub fn select_to_end(&mut self) {
        self.select_to(self.content.len());
    }

    // === Text manipulation ===

    /// Inserts text at the cursor position, replacing any selection.
    /// Records the edit for undo.
    pub fn insert_text(&mut self, text: &str) {
        let range = self
            .marked_range
            .clone()
            .unwrap_or(self.selected_range.clone());
        let range = range.start.min(self.content.len())..range.end.min(self.content.len());

        // Record for undo (skip during IME composition)
        if self.marked_range.is_none() {
            let old_text = self.content[range.clone()].to_string();
            self.undo_stack.push(HistoryEntry {
                range: range.clone(),
                old_text,
                new_text_len: text.len(),
                cursor_before: self.cursor_offset(),
            });
            // New edit clears redo stack
            self.redo_stack.clear();
        }

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

    /// Deletes to the previous word boundary.
    pub fn delete_word_left(&mut self) {
        if self.selected_range.is_empty() {
            let prev = self.previous_word_boundary(self.cursor_offset());
            if prev < self.cursor_offset() {
                self.select_to(prev);
            }
        }
        if !self.selected_range.is_empty() {
            self.insert_text("");
        }
    }

    /// Deletes to the next word boundary.
    pub fn delete_word_right(&mut self) {
        if self.selected_range.is_empty() {
            let next = self.next_word_boundary(self.cursor_offset());
            if next > self.cursor_offset() {
                self.select_to(next);
            }
        }
        if !self.selected_range.is_empty() {
            self.insert_text("");
        }
    }

    /// Deletes to the beginning of the current line.
    pub fn delete_to_beginning_of_line(&mut self) {
        if self.selected_range.is_empty() {
            let line_start = self.find_line_start(self.cursor_offset());
            if line_start < self.cursor_offset() {
                self.select_to(line_start);
            }
        }
        if !self.selected_range.is_empty() {
            self.insert_text("");
        }
    }

    /// Deletes to the end of the current line.
    pub fn delete_to_end_of_line(&mut self) {
        if self.selected_range.is_empty() {
            let line_end = self.find_line_end(self.cursor_offset());
            if line_end > self.cursor_offset() {
                self.select_to(line_end);
            }
        }
        if !self.selected_range.is_empty() {
            self.insert_text("");
        }
    }

    // === Clipboard operations ===

    /// Returns the currently selected text.
    pub fn selected_text(&self) -> &str {
        &self.content[self.selected_range.clone()]
    }

    /// Cuts (removes and returns) the currently selected text.
    pub fn cut_selected_text(&mut self) -> String {
        if self.selected_range.is_empty() {
            return String::new();
        }
        let text = self.content[self.selected_range.clone()].to_string();
        self.insert_text("");
        text
    }

    /// Pastes text at the cursor position, replacing any selection.
    pub fn paste(&mut self, text: &str) {
        self.insert_text(text);
    }

    // === Undo/Redo ===

    /// Returns whether undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns whether redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Undoes the last edit.
    pub fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop() {
            // Calculate the range of text that was inserted (to remove it)
            let inserted_end = entry.range.start + entry.new_text_len;
            let inserted_range = entry.range.start..inserted_end.min(self.content.len());

            // Save current state for redo
            let current_text = self.content[inserted_range.clone()].to_string();
            self.redo_stack.push(HistoryEntry {
                range: entry.range.clone(),
                old_text: current_text,
                new_text_len: entry.old_text.len(),
                cursor_before: self.cursor_offset(),
            });

            // Apply undo: replace inserted text with original text
            self.content.replace_range(inserted_range, &entry.old_text);
            self.selected_range = entry.cursor_before..entry.cursor_before;
            self.selection_reversed = false;
        }
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) {
        if let Some(entry) = self.redo_stack.pop() {
            // Calculate the range of text that was restored (to remove it)
            let restored_end = entry.range.start + entry.new_text_len;
            let restored_range = entry.range.start..restored_end.min(self.content.len());

            // Save current state for undo
            let current_text = self.content[restored_range.clone()].to_string();
            self.undo_stack.push(HistoryEntry {
                range: entry.range.clone(),
                old_text: current_text,
                new_text_len: entry.old_text.len(),
                cursor_before: self.cursor_offset(),
            });

            // Apply redo: replace restored text with the text that was there
            self.content.replace_range(restored_range, &entry.old_text);
            let new_cursor = entry.range.start + entry.old_text.len();
            self.selected_range = new_cursor..new_cursor;
            self.selection_reversed = false;
        }
    }

    // === Special insertions ===

    /// Inserts a newline at the cursor position.
    pub fn insert_newline(&mut self) {
        self.insert_text("\n");
    }

    /// Inserts a tab at the cursor position.
    pub fn insert_tab(&mut self) {
        self.insert_text("\t");
    }

    // === Key event handling ===

    /// Handles a key event using the default input bindings.
    ///
    /// Returns a `HandleKeyResult` indicating what happened:
    /// - `Handled`: The key modified the input state
    /// - `NotHandled`: The key wasn't recognized
    /// - `Blur`: Escape was pressed, blur focus
    /// - `Copy(text)`: Copy requested, caller should copy text to clipboard
    /// - `Cut(text)`: Cut requested, text was removed and should be copied to clipboard
    /// - `Paste`: Paste requested, caller should call `paste()` with clipboard content
    ///
    /// This provides centralized keybinding handling so consumers don't need to
    /// implement their own keyboard logic.
    pub fn handle_key_event(&mut self, key: &Key, modifiers: &Modifiers) -> HandleKeyResult {
        let bindings = InputBindings::new();
        let shift = modifiers.state().shift_key();

        let Some(action) = bindings.action_for_key(key, modifiers) else {
            return HandleKeyResult::NotHandled;
        };

        match action {
            // Character input
            InputAction::InsertCharacter => {
                if let Key::Character(c) = key {
                    self.insert_text(c.as_str());
                } else if let Key::Named(winit::keyboard::NamedKey::Space) = key {
                    self.insert_text(" ");
                }
                HandleKeyResult::Handled
            }
            InputAction::InsertNewline => {
                if self.multiline {
                    self.insert_newline();
                    HandleKeyResult::Handled
                } else {
                    HandleKeyResult::Submit
                }
            }
            InputAction::InsertTab => {
                if self.multiline {
                    self.insert_tab();
                    HandleKeyResult::Handled
                } else if shift {
                    HandleKeyResult::FocusPrev
                } else {
                    HandleKeyResult::FocusNext
                }
            }

            // Navigation
            InputAction::Left => {
                self.left();
                HandleKeyResult::Handled
            }
            InputAction::Right => {
                self.right();
                HandleKeyResult::Handled
            }
            InputAction::WordLeft => {
                self.word_left();
                HandleKeyResult::Handled
            }
            InputAction::WordRight => {
                self.word_right();
                HandleKeyResult::Handled
            }
            InputAction::Home => {
                self.home();
                HandleKeyResult::Handled
            }
            InputAction::End => {
                self.end();
                HandleKeyResult::Handled
            }
            InputAction::MoveToBeginning => {
                self.move_to_beginning();
                HandleKeyResult::Handled
            }
            InputAction::MoveToEnd => {
                self.move_to_end();
                HandleKeyResult::Handled
            }

            // Selection
            InputAction::SelectLeft => {
                self.select_left();
                HandleKeyResult::Handled
            }
            InputAction::SelectRight => {
                self.select_right();
                HandleKeyResult::Handled
            }
            InputAction::SelectWordLeft => {
                self.select_word_left();
                HandleKeyResult::Handled
            }
            InputAction::SelectWordRight => {
                self.select_word_right();
                HandleKeyResult::Handled
            }
            InputAction::SelectHome => {
                let line_start = self.find_line_start(self.cursor_offset());
                self.select_to(line_start);
                HandleKeyResult::Handled
            }
            InputAction::SelectEnd => {
                let line_end = self.find_line_end(self.cursor_offset());
                self.select_to(line_end);
                HandleKeyResult::Handled
            }
            InputAction::SelectToBeginning => {
                self.select_to_beginning();
                HandleKeyResult::Handled
            }
            InputAction::SelectToEnd => {
                self.select_to_end();
                HandleKeyResult::Handled
            }
            InputAction::SelectAll => {
                self.select_all();
                HandleKeyResult::Handled
            }

            // Deletion
            InputAction::Backspace => {
                self.delete_backward();
                HandleKeyResult::Handled
            }
            InputAction::Delete => {
                self.delete_forward();
                HandleKeyResult::Handled
            }
            InputAction::DeleteWordLeft => {
                self.delete_word_left();
                HandleKeyResult::Handled
            }
            InputAction::DeleteWordRight => {
                self.delete_word_right();
                HandleKeyResult::Handled
            }
            InputAction::DeleteToBeginningOfLine => {
                self.delete_to_beginning_of_line();
                HandleKeyResult::Handled
            }
            InputAction::DeleteToEndOfLine => {
                self.delete_to_end_of_line();
                HandleKeyResult::Handled
            }

            // Clipboard - return results for caller to handle
            InputAction::Copy => {
                let text = self.selected_text().to_string();
                HandleKeyResult::Copy(text)
            }
            InputAction::Cut => {
                let text = self.cut_selected_text();
                HandleKeyResult::Cut(text)
            }
            InputAction::Paste => HandleKeyResult::Paste,

            // History
            InputAction::Undo => {
                self.undo();
                HandleKeyResult::Handled
            }
            InputAction::Redo => {
                self.redo();
                HandleKeyResult::Handled
            }

            // Focus
            InputAction::Escape => HandleKeyResult::Blur,
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

    // ============================================================
    // Task: Implement word-level cursor movement
    // ============================================================

    #[test]
    fn word_left_moves_to_previous_word_start() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(8); // middle of "world"
        state.word_left();
        assert_eq!(state.cursor_offset(), 6); // start of "world"
    }

    #[test]
    fn word_left_from_word_start_goes_to_previous_word() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(6); // start of "world"
        state.word_left();
        assert_eq!(state.cursor_offset(), 0); // start of "hello"
    }

    #[test]
    fn word_left_at_start_stays_at_start() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(0);
        state.word_left();
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn word_left_with_selection_collapses_then_moves() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(2..8);
        state.word_left();
        // Should collapse to start of selection, then move to word boundary
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn word_right_moves_to_next_word_end() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(2); // middle of "hello"
        state.word_right();
        assert_eq!(state.cursor_offset(), 5); // end of "hello"
    }

    #[test]
    fn word_right_from_word_end_goes_to_next_word() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(5); // end of "hello"
        state.word_right();
        assert_eq!(state.cursor_offset(), 11); // end of "world"
    }

    #[test]
    fn word_right_at_end_stays_at_end() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.word_right();
        assert_eq!(state.cursor_offset(), 5);
    }

    #[test]
    fn word_right_with_selection_collapses_then_moves() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(2..8);
        state.word_right();
        // Should collapse to end of selection, then move to word boundary
        assert_eq!(state.cursor_offset(), 11);
    }

    // ============================================================
    // Task: Implement line-level cursor movement (home/end)
    // ============================================================

    #[test]
    fn home_moves_to_line_start() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(8); // middle of "world"
        state.home();
        assert_eq!(state.cursor_offset(), 6); // start of second line
    }

    #[test]
    fn home_on_first_line_moves_to_zero() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(3);
        state.home();
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn home_with_selection_collapses_then_moves() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(3..8);
        state.home();
        assert_eq!(state.cursor_offset(), 0);
        assert!(state.selected_range().is_empty());
    }

    #[test]
    fn end_moves_to_line_end() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(2); // middle of "hello"
        state.end();
        assert_eq!(state.cursor_offset(), 5); // end of first line (before \n)
    }

    #[test]
    fn end_on_last_line_moves_to_content_end() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(8);
        state.end();
        assert_eq!(state.cursor_offset(), 11);
    }

    #[test]
    fn end_with_selection_collapses_then_moves() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(3..8);
        state.end();
        assert_eq!(state.cursor_offset(), 11);
        assert!(state.selected_range().is_empty());
    }

    // ============================================================
    // Task: Implement document-level cursor movement
    // ============================================================

    #[test]
    fn move_to_beginning_moves_to_zero() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld\ntest");
        state.move_to(10);
        state.move_to_beginning();
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn move_to_beginning_with_selection_collapses() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(3..8);
        state.move_to_beginning();
        assert_eq!(state.cursor_offset(), 0);
        assert!(state.selected_range().is_empty());
    }

    #[test]
    fn move_to_end_moves_to_content_length() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld\ntest");
        state.move_to(5);
        state.move_to_end();
        assert_eq!(state.cursor_offset(), 16);
    }

    #[test]
    fn move_to_end_with_selection_collapses() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(3..8);
        state.move_to_end();
        assert_eq!(state.cursor_offset(), 11);
        assert!(state.selected_range().is_empty());
    }

    // ============================================================
    // Task: Implement selection extension (character level)
    // ============================================================

    #[test]
    fn select_left_extends_selection_by_one_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(3);
        state.select_left();
        assert_eq!(state.selected_range(), &(2..3));
        assert!(state.selection_reversed());
    }

    #[test]
    fn select_left_continues_extending() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(3);
        state.select_left();
        state.select_left();
        assert_eq!(state.selected_range(), &(1..3));
    }

    #[test]
    fn select_left_at_start_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(0);
        state.select_left();
        assert_eq!(state.selected_range(), &(0..0));
    }

    #[test]
    fn select_right_extends_selection_by_one_grapheme() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(2);
        state.select_right();
        assert_eq!(state.selected_range(), &(2..3));
        assert!(!state.selection_reversed());
    }

    #[test]
    fn select_right_continues_extending() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(2);
        state.select_right();
        state.select_right();
        assert_eq!(state.selected_range(), &(2..4));
    }

    #[test]
    fn select_right_at_end_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.select_right();
        assert_eq!(state.selected_range(), &(5..5));
    }

    // ============================================================
    // Task: Implement selection extension (word level)
    // ============================================================

    #[test]
    fn select_word_left_extends_to_previous_word_boundary() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(8);
        state.select_word_left();
        assert_eq!(state.selected_range(), &(6..8));
        assert!(state.selection_reversed());
    }

    #[test]
    fn select_word_left_continues_extending() {
        let mut state = TextEditState::new();
        state.set_content("hello world test");
        state.move_to(14);
        state.select_word_left();
        state.select_word_left();
        assert_eq!(state.selected_range(), &(6..14));
    }

    #[test]
    fn select_word_right_extends_to_next_word_boundary() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(2);
        state.select_word_right();
        assert_eq!(state.selected_range(), &(2..5));
        assert!(!state.selection_reversed());
    }

    #[test]
    fn select_word_right_continues_extending() {
        let mut state = TextEditState::new();
        state.set_content("hello world test");
        state.move_to(2);
        state.select_word_right();
        state.select_word_right();
        assert_eq!(state.selected_range(), &(2..11));
    }

    // ============================================================
    // Task: Implement selection extension (document level)
    // ============================================================

    #[test]
    fn select_all_selects_entire_content() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(5);
        state.select_all();
        assert_eq!(state.selected_range(), &(0..11));
        assert!(!state.selection_reversed());
    }

    #[test]
    fn select_all_on_empty_content() {
        let mut state = TextEditState::new();
        state.set_content("");
        state.select_all();
        assert_eq!(state.selected_range(), &(0..0));
    }

    #[test]
    fn select_to_beginning_extends_to_start() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(8);
        state.select_to_beginning();
        assert_eq!(state.selected_range(), &(0..8));
        assert!(state.selection_reversed());
    }

    #[test]
    fn select_to_beginning_from_selection_extends_from_anchor() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(5);
        state.select_to(8); // select 5..8
        state.select_to_beginning();
        assert_eq!(state.selected_range(), &(0..5)); // extends from anchor (5)
        assert!(state.selection_reversed());
    }

    #[test]
    fn select_to_end_extends_to_content_end() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(3);
        state.select_to_end();
        assert_eq!(state.selected_range(), &(3..11));
        assert!(!state.selection_reversed());
    }

    #[test]
    fn select_to_end_from_selection_extends_from_anchor() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(8);
        state.select_to(3); // select 3..8, reversed
        state.select_to_end();
        assert_eq!(state.selected_range(), &(8..11)); // extends from anchor (8)
        assert!(!state.selection_reversed());
    }

    // ============================================================
    // Task: Implement word deletion actions
    // ============================================================

    #[test]
    fn delete_word_left_removes_to_previous_word_boundary() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(8); // middle of "world"
        state.delete_word_left();
        assert_eq!(state.content(), "hello rld");
        assert_eq!(state.cursor_offset(), 6);
    }

    #[test]
    fn delete_word_left_at_word_start_removes_previous_word() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(6); // start of "world"
        state.delete_word_left();
        assert_eq!(state.content(), "world");
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn delete_word_left_at_start_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(0);
        state.delete_word_left();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn delete_word_left_with_selection_removes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(2..8);
        state.delete_word_left();
        assert_eq!(state.content(), "herld");
    }

    #[test]
    fn delete_word_right_removes_to_next_word_boundary() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(2); // middle of "hello"
        state.delete_word_right();
        assert_eq!(state.content(), "he world");
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn delete_word_right_at_word_end_removes_next_word() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(5); // end of "hello"
        state.delete_word_right();
        assert_eq!(state.content(), "hello");
        assert_eq!(state.cursor_offset(), 5);
    }

    #[test]
    fn delete_word_right_at_end_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.delete_word_right();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn delete_word_right_with_selection_removes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(2..8);
        state.delete_word_right();
        assert_eq!(state.content(), "herld");
    }

    // ============================================================
    // Task: Implement line deletion actions
    // ============================================================

    #[test]
    fn delete_to_beginning_of_line_removes_to_line_start() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(9); // after "wor" in "world"
        state.delete_to_beginning_of_line();
        assert_eq!(state.content(), "hello\nld"); // "wor" deleted
        assert_eq!(state.cursor_offset(), 6);
    }

    #[test]
    fn delete_to_beginning_of_line_on_first_line() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(6);
        state.delete_to_beginning_of_line();
        assert_eq!(state.content(), "world");
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn delete_to_beginning_of_line_at_line_start_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(6); // start of "world"
        state.delete_to_beginning_of_line();
        assert_eq!(state.content(), "hello\nworld");
    }

    #[test]
    fn delete_to_beginning_of_line_with_selection_removes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(2..8);
        state.delete_to_beginning_of_line();
        assert_eq!(state.content(), "herld");
    }

    #[test]
    fn delete_to_end_of_line_removes_to_line_end() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(2); // middle of "hello"
        state.delete_to_end_of_line();
        assert_eq!(state.content(), "he\nworld");
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn delete_to_end_of_line_on_last_line() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(8);
        state.delete_to_end_of_line();
        assert_eq!(state.content(), "hello\nwo");
        assert_eq!(state.cursor_offset(), 8);
    }

    #[test]
    fn delete_to_end_of_line_at_line_end_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello\nworld");
        state.move_to(5); // end of "hello" (before \n)
        state.delete_to_end_of_line();
        assert_eq!(state.content(), "hello\nworld");
    }

    #[test]
    fn delete_to_end_of_line_with_selection_removes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(2..8);
        state.delete_to_end_of_line();
        assert_eq!(state.content(), "herld");
    }

    // ============================================================
    // Task: Implement clipboard operations
    // ============================================================

    #[test]
    fn selected_text_returns_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(0..5);
        assert_eq!(state.selected_text(), "hello");
    }

    #[test]
    fn selected_text_returns_empty_when_no_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(5);
        assert_eq!(state.selected_text(), "");
    }

    #[test]
    fn cut_selected_text_returns_and_removes_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(0..6);
        let cut = state.cut_selected_text();
        assert_eq!(cut, "hello ");
        assert_eq!(state.content(), "world");
        assert_eq!(state.cursor_offset(), 0);
    }

    #[test]
    fn cut_selected_text_with_no_selection_returns_empty() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.move_to(5);
        let cut = state.cut_selected_text();
        assert_eq!(cut, "");
        assert_eq!(state.content(), "hello world");
    }

    #[test]
    fn paste_inserts_text_at_cursor() {
        let mut state = TextEditState::new();
        state.set_content("helloworld");
        state.move_to(5);
        state.paste(" ");
        assert_eq!(state.content(), "hello world");
    }

    #[test]
    fn paste_replaces_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(6..11);
        state.paste("there");
        assert_eq!(state.content(), "hello there");
    }

    // ============================================================
    // Task: Implement undo/redo with patch-based history
    // ============================================================

    #[test]
    fn can_undo_returns_false_initially() {
        let state = TextEditState::new();
        assert!(!state.can_undo());
    }

    #[test]
    fn can_undo_returns_true_after_edit() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        assert!(state.can_undo());
    }

    #[test]
    fn undo_reverses_insert() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        assert_eq!(state.content(), "hello!");

        state.undo();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn undo_reverses_delete() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.delete_backward();
        assert_eq!(state.content(), "hell");

        state.undo();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn undo_restores_cursor_position() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(2);
        state.insert_text("X");
        assert_eq!(state.cursor_offset(), 3);

        state.undo();
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn multiple_undos() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        state.insert_text("!");
        assert_eq!(state.content(), "hello!!");

        state.undo();
        assert_eq!(state.content(), "hello!");
        state.undo();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn can_redo_returns_false_initially() {
        let state = TextEditState::new();
        assert!(!state.can_redo());
    }

    #[test]
    fn can_redo_returns_true_after_undo() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        state.undo();
        assert!(state.can_redo());
    }

    #[test]
    fn redo_reapplies_undone_edit() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        state.undo();
        assert_eq!(state.content(), "hello");

        state.redo();
        assert_eq!(state.content(), "hello!");
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        state.undo();
        assert!(state.can_redo());

        state.insert_text("?");
        assert!(!state.can_redo());
    }

    #[test]
    fn undo_at_beginning_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.undo(); // no-op
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn redo_at_end_does_nothing() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        state.redo(); // no-op, nothing to redo
        assert_eq!(state.content(), "hello!");
    }

    #[test]
    fn set_content_clears_history() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);
        state.insert_text("!");
        assert!(state.can_undo());

        state.set_content("new content");
        assert!(!state.can_undo());
        assert!(!state.can_redo());
    }

    // ============================================================
    // Task: Implement Enter key for multiline
    // ============================================================

    #[test]
    fn insert_newline_inserts_newline_char() {
        let mut state = TextEditState::new();
        state.set_content("helloworld");
        state.move_to(5);
        state.insert_newline();
        assert_eq!(state.content(), "hello\nworld");
    }

    #[test]
    fn insert_newline_replaces_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(5..6);
        state.insert_newline();
        assert_eq!(state.content(), "hello\nworld");
    }

    // ============================================================
    // Task: Implement Tab key handling
    // ============================================================

    #[test]
    fn insert_tab_inserts_tab_char() {
        let mut state = TextEditState::new();
        state.set_content("helloworld");
        state.move_to(5);
        state.insert_tab();
        assert_eq!(state.content(), "hello\tworld");
    }

    #[test]
    fn insert_tab_replaces_selection() {
        let mut state = TextEditState::new();
        state.set_content("hello world");
        state.set_selected_range(5..6);
        state.insert_tab();
        assert_eq!(state.content(), "hello\tworld");
    }

    // ============================================================
    // Task: Implement handle_key_event (centralized keybinding)
    // ============================================================

    use super::HandleKeyResult;
    use winit::event::Modifiers;
    use winit::keyboard::{Key, ModifiersState, NamedKey};

    fn no_mods() -> Modifiers {
        Modifiers::default()
    }

    fn shift_mod() -> Modifiers {
        Modifiers::from(ModifiersState::SHIFT)
    }

    #[cfg(target_os = "macos")]
    fn cmd_mod() -> Modifiers {
        Modifiers::from(ModifiersState::SUPER)
    }

    #[cfg(not(target_os = "macos"))]
    fn cmd_mod() -> Modifiers {
        Modifiers::from(ModifiersState::CONTROL)
    }

    #[test]
    fn handle_key_event_character_inserts_text() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);

        let result = state.handle_key_event(&Key::Character("!".into()), &no_mods());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.content(), "hello!");
    }

    #[test]
    fn handle_key_event_space_inserts_space() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);

        let result = state.handle_key_event(&Key::Named(NamedKey::Space), &no_mods());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.content(), "hello ");
    }

    #[test]
    fn handle_key_event_backspace_deletes() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(5);

        let result = state.handle_key_event(&Key::Named(NamedKey::Backspace), &no_mods());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.content(), "hell");
    }

    #[test]
    fn handle_key_event_left_moves_cursor() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(3);

        let result = state.handle_key_event(&Key::Named(NamedKey::ArrowLeft), &no_mods());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.cursor_offset(), 2);
    }

    #[test]
    fn handle_key_event_shift_left_selects() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(3);

        let result = state.handle_key_event(&Key::Named(NamedKey::ArrowLeft), &shift_mod());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.selected_range(), &(2..3));
    }

    #[test]
    fn handle_key_event_cmd_a_selects_all() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.move_to(3);

        let result = state.handle_key_event(&Key::Character("a".into()), &cmd_mod());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.selected_range(), &(0..5));
    }

    #[test]
    fn handle_key_event_escape_blurs() {
        let mut state = TextEditState::new();
        state.set_content("hello");

        let result = state.handle_key_event(&Key::Named(NamedKey::Escape), &no_mods());

        assert_eq!(result, HandleKeyResult::Blur);
    }

    #[test]
    fn handle_key_event_unknown_key_not_handled() {
        let mut state = TextEditState::new();
        state.set_content("hello");

        // F1 key isn't handled by input bindings
        let result = state.handle_key_event(&Key::Named(NamedKey::F1), &no_mods());

        assert_eq!(result, HandleKeyResult::NotHandled);
    }

    #[test]
    fn handle_key_event_copy_returns_copy_result() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);

        let result = state.handle_key_event(&Key::Character("c".into()), &cmd_mod());

        assert_eq!(result, HandleKeyResult::Copy("ell".to_string()));
    }

    #[test]
    fn handle_key_event_cut_returns_cut_result() {
        let mut state = TextEditState::new();
        state.set_content("hello");
        state.set_selected_range(1..4);

        let result = state.handle_key_event(&Key::Character("x".into()), &cmd_mod());

        assert_eq!(result, HandleKeyResult::Cut("ell".to_string()));
        assert_eq!(state.content(), "ho");
    }

    #[test]
    fn handle_key_event_paste_returns_paste_result() {
        let mut state = TextEditState::new();
        state.set_content("hello");

        let result = state.handle_key_event(&Key::Character("v".into()), &cmd_mod());

        assert_eq!(result, HandleKeyResult::Paste);
    }

    // ============================================================
    // Task: Single-line Enter/Tab behavior
    // ============================================================

    #[test]
    fn handle_key_event_enter_single_line_returns_submit() {
        let mut state = TextEditState::new(); // single-line by default
        state.set_content("hello");

        let result = state.handle_key_event(&Key::Named(NamedKey::Enter), &no_mods());

        assert_eq!(result, HandleKeyResult::Submit);
        // Content should not change
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn handle_key_event_enter_multiline_inserts_newline() {
        let mut state = TextEditState::new_multiline();
        state.set_content("hello");
        state.move_to(5);

        let result = state.handle_key_event(&Key::Named(NamedKey::Enter), &no_mods());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.content(), "hello\n");
    }

    #[test]
    fn handle_key_event_tab_single_line_returns_focus_next() {
        let mut state = TextEditState::new();
        state.set_content("hello");

        let result = state.handle_key_event(&Key::Named(NamedKey::Tab), &no_mods());

        assert_eq!(result, HandleKeyResult::FocusNext);
        // Content should not change
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn handle_key_event_shift_tab_single_line_returns_focus_prev() {
        let mut state = TextEditState::new();
        state.set_content("hello");

        let result = state.handle_key_event(&Key::Named(NamedKey::Tab), &shift_mod());

        assert_eq!(result, HandleKeyResult::FocusPrev);
        // Content should not change
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn handle_key_event_tab_multiline_inserts_tab() {
        let mut state = TextEditState::new_multiline();
        state.set_content("hello");
        state.move_to(5);

        let result = state.handle_key_event(&Key::Named(NamedKey::Tab), &no_mods());

        assert_eq!(result, HandleKeyResult::Handled);
        assert_eq!(state.content(), "hello\t");
    }

    #[test]
    fn new_creates_single_line_state() {
        let state = TextEditState::new();
        assert!(!state.is_multiline());
    }

    #[test]
    fn new_multiline_creates_multiline_state() {
        let state = TextEditState::new_multiline();
        assert!(state.is_multiline());
    }
}
