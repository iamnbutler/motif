//! Line layout for multiline text.
//!
//! [`LineLayout`] computes and caches the byte-offset boundaries of each line
//! in a string, enabling efficient O(log n) lookups between byte offsets and
//! `(line, column)` coordinates.
//!
//! This is the foundation for multiline cursor positioning, vertical scrolling,
//! and pixel-accurate caret rendering in [`TextEditState`].
//!
//! [`TextEditState`]: crate::input::TextEditState
//!
//! # Example
//!
//! ```
//! use motif_core::input::LineLayout;
//!
//! let text = "hello\nworld\nrust";
//! let ll = LineLayout::new(text);
//!
//! assert_eq!(ll.line_count(), 3);
//! assert_eq!(ll.line_for_offset(7), 1);  // "world" is line 1
//! assert_eq!(ll.col_for_offset(8, text), 2);  // 'r' in "world" is col 2
//! ```

/// Efficient line layout for multiline text.
///
/// Stores the byte offset of the first character of each line, built once and
/// reused for many lookups.  All offsets are in terms of UTF-8 bytes, matching
/// Rust's `&str` indexing.
///
/// # Line numbering
///
/// Lines are 0-indexed.  A string with no newlines has one line (line 0).
/// A trailing newline creates an additional empty line.
///
/// ```
/// use motif_core::input::LineLayout;
///
/// assert_eq!(LineLayout::new("hello").line_count(), 1);
/// assert_eq!(LineLayout::new("a\nb").line_count(), 2);
/// assert_eq!(LineLayout::new("a\n").line_count(), 2);   // trailing newline
/// assert_eq!(LineLayout::new("").line_count(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineLayout {
    /// Byte offset of the first character in each line.
    /// `line_starts[0]` is always `0`.
    line_starts: Vec<usize>,
    /// Total byte length of the source text.
    text_len: usize,
}

impl LineLayout {
    /// Build a [`LineLayout`] from the given text.
    ///
    /// Iterates the text once to collect newline positions.  O(n) in text
    /// length.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(i + ch.len_utf8());
            }
        }
        Self {
            line_starts,
            text_len: text.len(),
        }
    }

    /// Returns the number of lines (always at least 1).
    ///
    /// A trailing `\n` counts as an additional empty line.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns the 0-based line index containing `offset`.
    ///
    /// Offsets beyond the text length are clamped to the last line.
    ///
    /// The `\n` terminator of a line belongs to that line (not the next).
    ///
    /// ```
    /// use motif_core::input::LineLayout;
    ///
    /// let ll = LineLayout::new("ab\ncd");
    /// assert_eq!(ll.line_for_offset(0), 0);  // 'a'
    /// assert_eq!(ll.line_for_offset(2), 0);  // '\n'
    /// assert_eq!(ll.line_for_offset(3), 1);  // 'c'
    /// assert_eq!(ll.line_for_offset(99), 1); // clamped
    /// ```
    pub fn line_for_offset(&self, offset: usize) -> usize {
        let offset = offset.min(self.text_len);
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    /// Returns the byte offset of the start of `line`.
    ///
    /// If `line` is out of bounds, returns the start of the last line.
    pub fn line_start(&self, line: usize) -> usize {
        let line = line.min(self.line_starts.len().saturating_sub(1));
        self.line_starts[line]
    }

    /// Returns the byte offset one past the last *content* character of
    /// `line`, i.e. excluding any trailing `\n`.
    ///
    /// For the final line (or a line with no trailing newline), this equals
    /// `text.len()`.
    ///
    /// ```
    /// use motif_core::input::LineLayout;
    ///
    /// let text = "hello\nworld";
    /// let ll = LineLayout::new(text);
    /// assert_eq!(ll.line_end(0, text), 5);   // "hello" is 5 bytes
    /// assert_eq!(ll.line_end(1, text), 11);  // "world" ends at EOF
    /// ```
    pub fn line_end(&self, line: usize, text: &str) -> usize {
        if line + 1 < self.line_starts.len() {
            // The next line starts after the '\n' — step back to exclude it.
            self.line_starts[line + 1].saturating_sub(1)
        } else {
            text.len()
        }
    }

    /// Returns the grapheme-cluster column of `offset` within its line.
    ///
    /// Column 0 is the first character of the line.  If `offset` points to
    /// the `\n` terminator, the column equals the number of content
    /// characters on that line.
    ///
    /// ```
    /// use motif_core::input::LineLayout;
    ///
    /// let text = "hi\nthere";
    /// let ll = LineLayout::new(text);
    /// assert_eq!(ll.col_for_offset(0, text), 0);   // 'h'
    /// assert_eq!(ll.col_for_offset(1, text), 1);   // 'i'
    /// assert_eq!(ll.col_for_offset(2, text), 2);   // '\n'
    /// assert_eq!(ll.col_for_offset(3, text), 0);   // 't' on line 1
    /// assert_eq!(ll.col_for_offset(5, text), 2);   // 'e' in "there"
    /// ```
    pub fn col_for_offset(&self, offset: usize, text: &str) -> usize {
        let offset = offset.min(text.len());
        let line = self.line_for_offset(offset);
        let line_start = self.line_start(line);
        // Count scalar values (chars) from line start to offset.
        // For correct grapheme-cluster counting, the caller may use
        // `unicode_segmentation`; char-based columns match the existing
        // move_left / move_right semantics in TextEditState.
        text[line_start..offset].chars().count()
    }

    /// Returns the byte offset for the given `(line, col)` pair.
    ///
    /// `col` is measured in Unicode scalar values (chars).  If `col` exceeds
    /// the number of chars on the line, the offset is clamped to the end of
    /// that line (before any `\n`).
    ///
    /// ```
    /// use motif_core::input::LineLayout;
    ///
    /// let text = "hello\nworld";
    /// let ll = LineLayout::new(text);
    /// assert_eq!(ll.offset_for_line_col(0, 0, text), 0);
    /// assert_eq!(ll.offset_for_line_col(0, 3, text), 3);   // 'l' in "hello"
    /// assert_eq!(ll.offset_for_line_col(0, 99, text), 5);  // clamped to line end
    /// assert_eq!(ll.offset_for_line_col(1, 2, text), 8);   // 'r' in "world"
    /// ```
    pub fn offset_for_line_col(&self, line: usize, col: usize, text: &str) -> usize {
        let line = line.min(self.line_starts.len().saturating_sub(1));
        let line_start = self.line_start(line);
        let line_end = self.line_end(line, text);
        let line_text = &text[line_start..line_end];

        let mut char_count = 0usize;
        for (byte_pos, _ch) in line_text.char_indices() {
            if char_count == col {
                return line_start + byte_pos;
            }
            char_count += 1;
        }
        // col >= line length: clamp to end of line content.
        line_start + line_text.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── construction ─────────────────────────────────────────────────────────

    #[test]
    fn empty_string_has_one_line() {
        let ll = LineLayout::new("");
        assert_eq!(ll.line_count(), 1);
    }

    #[test]
    fn single_line_no_newline() {
        let ll = LineLayout::new("hello");
        assert_eq!(ll.line_count(), 1);
    }

    #[test]
    fn two_lines() {
        let ll = LineLayout::new("hello\nworld");
        assert_eq!(ll.line_count(), 2);
    }

    #[test]
    fn three_lines() {
        let ll = LineLayout::new("a\nb\nc");
        assert_eq!(ll.line_count(), 3);
    }

    #[test]
    fn trailing_newline_adds_empty_line() {
        let ll = LineLayout::new("hello\n");
        assert_eq!(ll.line_count(), 2);
    }

    #[test]
    fn consecutive_newlines() {
        let ll = LineLayout::new("a\n\nb");
        assert_eq!(ll.line_count(), 3);
    }

    // ── line_for_offset ───────────────────────────────────────────────────────

    #[test]
    fn line_for_offset_single_line() {
        let ll = LineLayout::new("hello");
        assert_eq!(ll.line_for_offset(0), 0);
        assert_eq!(ll.line_for_offset(4), 0);
    }

    #[test]
    fn line_for_offset_two_lines() {
        let text = "ab\ncd";
        let ll = LineLayout::new(text);
        assert_eq!(ll.line_for_offset(0), 0); // 'a'
        assert_eq!(ll.line_for_offset(1), 0); // 'b'
        assert_eq!(ll.line_for_offset(2), 0); // '\n' — belongs to line 0
        assert_eq!(ll.line_for_offset(3), 1); // 'c'
        assert_eq!(ll.line_for_offset(4), 1); // 'd'
    }

    #[test]
    fn line_for_offset_clamped_beyond_text() {
        let ll = LineLayout::new("hi\nthere");
        assert_eq!(ll.line_for_offset(999), 1);
    }

    #[test]
    fn line_for_offset_at_text_len() {
        let text = "abc";
        let ll = LineLayout::new(text);
        assert_eq!(ll.line_for_offset(text.len()), 0);
    }

    // ── line_start ────────────────────────────────────────────────────────────

    #[test]
    fn line_start_first_line_is_zero() {
        let ll = LineLayout::new("anything");
        assert_eq!(ll.line_start(0), 0);
    }

    #[test]
    fn line_start_second_line() {
        let ll = LineLayout::new("hello\nworld");
        assert_eq!(ll.line_start(1), 6); // 'w' of "world"
    }

    #[test]
    fn line_start_clamped() {
        let ll = LineLayout::new("single");
        assert_eq!(ll.line_start(99), 0);
    }

    // ── line_end ──────────────────────────────────────────────────────────────

    #[test]
    fn line_end_excludes_newline() {
        let text = "hello\nworld";
        let ll = LineLayout::new(text);
        assert_eq!(ll.line_end(0, text), 5); // "hello" = bytes 0..5
    }

    #[test]
    fn line_end_last_line_is_text_len() {
        let text = "hello\nworld";
        let ll = LineLayout::new(text);
        assert_eq!(ll.line_end(1, text), text.len());
    }

    #[test]
    fn line_end_trailing_newline() {
        let text = "hello\n";
        let ll = LineLayout::new(text);
        assert_eq!(ll.line_end(0, text), 5); // "hello"
        assert_eq!(ll.line_end(1, text), 6); // empty line 1 ends at EOF
    }

    // ── col_for_offset ────────────────────────────────────────────────────────

    #[test]
    fn col_for_offset_first_char_is_zero() {
        let text = "hello";
        let ll = LineLayout::new(text);
        assert_eq!(ll.col_for_offset(0, text), 0);
    }

    #[test]
    fn col_for_offset_on_second_line() {
        let text = "hi\nthere";
        let ll = LineLayout::new(text);
        assert_eq!(ll.col_for_offset(3, text), 0); // 't' of "there"
        assert_eq!(ll.col_for_offset(5, text), 2); // 'e' at index 2 in "there"
    }

    #[test]
    fn col_for_offset_newline_char() {
        let text = "hi\nthere";
        let ll = LineLayout::new(text);
        assert_eq!(ll.col_for_offset(2, text), 2); // '\n' is col 2 in "hi"
    }

    // ── offset_for_line_col ───────────────────────────────────────────────────

    #[test]
    fn offset_for_line_col_first_char() {
        let text = "hello\nworld";
        let ll = LineLayout::new(text);
        assert_eq!(ll.offset_for_line_col(0, 0, text), 0);
    }

    #[test]
    fn offset_for_line_col_within_line() {
        let text = "hello\nworld";
        let ll = LineLayout::new(text);
        assert_eq!(ll.offset_for_line_col(0, 3, text), 3); // 'l'
        assert_eq!(ll.offset_for_line_col(1, 2, text), 8); // 'r' in "world"
    }

    #[test]
    fn offset_for_line_col_clamp_to_line_end() {
        let text = "hello\nworld";
        let ll = LineLayout::new(text);
        assert_eq!(ll.offset_for_line_col(0, 99, text), 5); // end of "hello"
        assert_eq!(ll.offset_for_line_col(1, 99, text), 11); // EOF
    }

    #[test]
    fn offset_for_line_col_clamp_line_out_of_bounds() {
        let text = "hello";
        let ll = LineLayout::new(text);
        assert_eq!(ll.offset_for_line_col(99, 0, text), 0);
    }

    // ── round-trip ────────────────────────────────────────────────────────────

    #[test]
    fn round_trip_offset_to_line_col_to_offset() {
        let text = "first\nsecond\nthird";
        let ll = LineLayout::new(text);
        for offset in 0..=text.len() {
            // Only round-trip on char boundaries.
            if !text.is_char_boundary(offset) {
                continue;
            }
            let line = ll.line_for_offset(offset);
            let col = ll.col_for_offset(offset, text);
            let recovered = ll.offset_for_line_col(line, col, text);
            assert_eq!(
                recovered, offset,
                "round-trip failed for offset {offset} (line {line}, col {col})"
            );
        }
    }

    // ── unicode ───────────────────────────────────────────────────────────────

    #[test]
    fn multibyte_unicode_chars() {
        // "é" is 2 bytes in UTF-8 (U+00E9).
        let text = "caf\u{00e9}\nhello";
        let ll = LineLayout::new(text);
        assert_eq!(ll.line_count(), 2);
        // "caf\u{00e9}" is 5 bytes; '\n' is at byte 5; line 1 starts at 6.
        assert_eq!(ll.line_start(1), 6);
        // col_for_offset: 'é' at byte 3..5 is 1 char past "caf" = col 3.
        assert_eq!(ll.col_for_offset(3, text), 3);
        // offset_for_line_col: col 3 on line 0 → byte 3.
        assert_eq!(ll.offset_for_line_col(0, 3, text), 3);
    }
}
