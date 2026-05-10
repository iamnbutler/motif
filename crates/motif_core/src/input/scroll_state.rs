//! Vertical scroll state for multiline text inputs.
//!
//! [`VerticalScrollState`] tracks which line is at the top of the viewport
//! and provides the scrolling logic needed to keep the cursor visible in a
//! multiline text area.
//!
//! # Design
//!
//! The scroll state operates at the *logical line* level (newline-separated
//! lines) rather than at pixel level. This keeps the data structure
//! renderer-agnostic and easy to test in pure Rust without a running graphics
//! stack.
//!
//! Renderers that know the line height (`px_per_line`) convert the scroll
//! offset to pixels with:
//!
//! ```text
//! scroll_offset_y = scroll_state.first_visible_line() as f32 * px_per_line
//! ```
//!
//! # Relationship to other types
//!
//! - **`LineLayout`** (PR #74) provides `line_for_offset(cursor_byte)` to
//!   obtain the cursor's line index, which is passed to
//!   [`VerticalScrollState::scroll_to_ensure_visible`].
//! - **`TextEditState`** (PR #73) tracks the cursor byte offset; pair with
//!   `LineLayout` to derive the cursor line.

/// Tracks vertical scroll state for multiline text areas.
///
/// Call [`scroll_to_ensure_visible`](Self::scroll_to_ensure_visible) after
/// every cursor movement to keep the cursor in view. Call
/// [`set_visible_line_count`](Self::set_visible_line_count) whenever the
/// viewport height changes (e.g., window resize).
///
/// # Example
///
/// ```rust,ignore
/// use motif_core::input::{VerticalScrollState, LineLayout, TextEditState};
///
/// let mut scroll = VerticalScrollState::with_visible_lines(10);
/// let mut text = TextEditState::new_multiline();
/// text.set_content("line1\nline2\nline3\n…");
///
/// let layout = LineLayout::new(text.content());
/// let cursor_line = layout.line_for_offset(text.cursor_offset());
/// scroll.scroll_to_ensure_visible(cursor_line, layout.line_count());
///
/// // Renderer multiplies by line_height to get scroll_offset_y in pixels
/// let scroll_offset_y = scroll.first_visible_line() as f32 * LINE_HEIGHT;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerticalScrollState {
    /// Index of the topmost visible line (0-based).
    first_visible_line: usize,
    /// How many lines fit in the viewport (0 means unset).
    visible_line_count: usize,
}

impl Default for VerticalScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl VerticalScrollState {
    /// Creates a new scroll state positioned at the top of the content.
    ///
    /// `visible_line_count` is initialised to 0 (unset). Set it via
    /// [`set_visible_line_count`](Self::set_visible_line_count) once layout
    /// is available.
    pub fn new() -> Self {
        Self {
            first_visible_line: 0,
            visible_line_count: 0,
        }
    }

    /// Creates a scroll state with a known viewport line capacity.
    ///
    /// Equivalent to `new()` followed by `set_visible_line_count(count)`.
    pub fn with_visible_lines(count: usize) -> Self {
        Self {
            first_visible_line: 0,
            visible_line_count: count,
        }
    }

    // === Accessors ===

    /// The index of the first (topmost) visible line (0-based).
    pub fn first_visible_line(&self) -> usize {
        self.first_visible_line
    }

    /// How many lines fit in the viewport.
    ///
    /// Returns 0 if the visible line count has not been set yet.
    pub fn visible_line_count(&self) -> usize {
        self.visible_line_count
    }

    /// The index of the last visible line, inclusive.
    ///
    /// When `visible_line_count` is 0 or 1, returns `first_visible_line`
    /// (the single-line case).
    pub fn last_visible_line(&self) -> usize {
        if self.visible_line_count <= 1 {
            self.first_visible_line
        } else {
            self.first_visible_line + self.visible_line_count - 1
        }
    }

    /// Returns `true` if `line` is within the current viewport.
    pub fn line_is_visible(&self, line: usize) -> bool {
        line >= self.first_visible_line && line <= self.last_visible_line()
    }

    // === Mutation ===

    /// Updates the number of lines visible in the viewport.
    ///
    /// Call this when the viewport height changes (e.g., on window resize).
    /// After calling, consider re-invoking
    /// [`scroll_to_ensure_visible`](Self::scroll_to_ensure_visible) with the
    /// current cursor line to re-clamp the scroll position.
    pub fn set_visible_line_count(&mut self, count: usize) {
        self.visible_line_count = count;
    }

    /// Scrolls by `delta` lines. Positive values scroll down; negative scroll up.
    ///
    /// `total_lines` is the total number of logical lines in the content (e.g.,
    /// `LineLayout::line_count()`). The scroll position is clamped so that the
    /// last visible line never exceeds `total_lines − 1`.
    pub fn scroll_by(&mut self, delta: i32, total_lines: usize) {
        if total_lines == 0 {
            self.first_visible_line = 0;
            return;
        }
        let current = self.first_visible_line as i64;
        let new_pos = (current + delta as i64).max(0) as usize;
        let max_first = total_lines.saturating_sub(self.visible_line_count.max(1));
        self.first_visible_line = new_pos.min(max_first);
    }

    /// Adjusts `first_visible_line` so that `cursor_line` is within the viewport.
    ///
    /// - If the cursor is **above** the viewport, scrolls up so the cursor
    ///   becomes the topmost visible line.
    /// - If the cursor is **below** the viewport, scrolls down so the cursor
    ///   becomes the bottommost visible line.
    /// - If the cursor is already visible, does nothing.
    ///
    /// When `visible_line_count` is 0 (unset), the scroll position simply
    /// tracks the cursor line directly.
    ///
    /// `total_lines` is used to clamp the scroll position so it never scrolls
    /// past the end of the content.
    pub fn scroll_to_ensure_visible(&mut self, cursor_line: usize, total_lines: usize) {
        if self.visible_line_count == 0 {
            // Viewport size unknown — just track the cursor.
            self.first_visible_line = cursor_line;
            return;
        }

        if cursor_line < self.first_visible_line {
            // Cursor is above the viewport: scroll up.
            self.first_visible_line = cursor_line;
        } else if cursor_line > self.last_visible_line() {
            // Cursor is below the viewport: scroll down.
            // cursor_line + 1 - visible_line_count gives the new first line.
            let new_first = cursor_line + 1 - self.visible_line_count;
            // Never scroll past where the last line would still be visible.
            let max_first = total_lines.saturating_sub(self.visible_line_count);
            self.first_visible_line = new_first.min(max_first);
        }
        // Cursor already visible — no change needed.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Construction ---

    #[test]
    fn new_starts_at_top_with_no_visible_count() {
        let s = VerticalScrollState::new();
        assert_eq!(s.first_visible_line(), 0);
        assert_eq!(s.visible_line_count(), 0);
    }

    #[test]
    fn with_visible_lines_sets_count() {
        let s = VerticalScrollState::with_visible_lines(10);
        assert_eq!(s.first_visible_line(), 0);
        assert_eq!(s.visible_line_count(), 10);
    }

    #[test]
    fn default_equals_new() {
        assert_eq!(VerticalScrollState::default(), VerticalScrollState::new());
    }

    // --- last_visible_line ---

    #[test]
    fn last_visible_line_when_count_is_zero() {
        let s = VerticalScrollState::new(); // visible_line_count = 0
        assert_eq!(s.last_visible_line(), 0);
    }

    #[test]
    fn last_visible_line_when_count_is_one() {
        let s = VerticalScrollState::with_visible_lines(1);
        assert_eq!(s.last_visible_line(), 0);
    }

    #[test]
    fn last_visible_line_normal() {
        let s = VerticalScrollState::with_visible_lines(5);
        // first=0, count=5 → last=4
        assert_eq!(s.last_visible_line(), 4);
    }

    // --- line_is_visible ---

    #[test]
    fn line_is_visible_for_first_line() {
        let s = VerticalScrollState::with_visible_lines(5);
        assert!(s.line_is_visible(0));
    }

    #[test]
    fn line_is_visible_for_last_visible_line() {
        let s = VerticalScrollState::with_visible_lines(5);
        assert!(s.line_is_visible(4));
    }

    #[test]
    fn line_is_visible_false_for_line_below_viewport() {
        let s = VerticalScrollState::with_visible_lines(5);
        assert!(!s.line_is_visible(5));
    }

    // --- scroll_to_ensure_visible ---

    #[test]
    fn scroll_ensure_no_op_when_cursor_already_visible() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        // Lines 0–4 visible; cursor on line 2
        s.scroll_to_ensure_visible(2, 20);
        assert_eq!(s.first_visible_line(), 0);
    }

    #[test]
    fn scroll_ensure_scrolls_down_when_cursor_below() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        // Viewport shows lines 0–4; cursor moves to line 7
        s.scroll_to_ensure_visible(7, 20);
        // first_visible should be 7 + 1 - 5 = 3
        assert_eq!(s.first_visible_line(), 3);
        assert!(s.line_is_visible(7));
    }

    #[test]
    fn scroll_ensure_scrolls_up_when_cursor_above() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        // Scroll down first to show lines 10–14
        s.first_visible_line = 10;
        // Then cursor moves to line 5 (above viewport)
        s.scroll_to_ensure_visible(5, 20);
        assert_eq!(s.first_visible_line(), 5);
        assert!(s.line_is_visible(5));
    }

    #[test]
    fn scroll_ensure_cursor_at_last_line_boundary() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        // Cursor moves to line 4 (last visible of first page)
        s.scroll_to_ensure_visible(4, 20);
        assert_eq!(s.first_visible_line(), 0);
    }

    #[test]
    fn scroll_ensure_zero_visible_count_tracks_cursor() {
        let mut s = VerticalScrollState::new(); // visible_line_count = 0
        s.scroll_to_ensure_visible(12, 20);
        assert_eq!(s.first_visible_line(), 12);
    }

    #[test]
    fn scroll_ensure_clamps_when_content_shorter_than_viewport() {
        let mut s = VerticalScrollState::with_visible_lines(10);
        // Only 3 lines of content, cursor on line 2
        s.scroll_to_ensure_visible(2, 3);
        assert_eq!(s.first_visible_line(), 0);
    }

    // --- scroll_by ---

    #[test]
    fn scroll_by_positive_scrolls_down() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        s.scroll_by(3, 20);
        assert_eq!(s.first_visible_line(), 3);
    }

    #[test]
    fn scroll_by_negative_scrolls_up() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        s.first_visible_line = 10;
        s.scroll_by(-3, 20);
        assert_eq!(s.first_visible_line(), 7);
    }

    #[test]
    fn scroll_by_clamps_at_start() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        s.first_visible_line = 2;
        s.scroll_by(-10, 20);
        assert_eq!(s.first_visible_line(), 0);
    }

    #[test]
    fn scroll_by_clamps_at_end() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        // 20 total lines, viewport=5 → max_first = 15
        s.scroll_by(100, 20);
        assert_eq!(s.first_visible_line(), 15);
    }

    #[test]
    fn scroll_by_zero_total_lines_resets() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        s.first_visible_line = 3;
        s.scroll_by(2, 0);
        assert_eq!(s.first_visible_line(), 0);
    }

    // --- set_visible_line_count ---

    #[test]
    fn set_visible_line_count_updates_viewport() {
        let mut s = VerticalScrollState::new();
        s.set_visible_line_count(8);
        assert_eq!(s.visible_line_count(), 8);
        assert_eq!(s.last_visible_line(), 7);
    }

    // --- round-trip ---

    #[test]
    fn scroll_down_then_up_round_trips() {
        let mut s = VerticalScrollState::with_visible_lines(5);
        s.scroll_by(10, 30);
        assert_eq!(s.first_visible_line(), 10);
        s.scroll_by(-10, 30);
        assert_eq!(s.first_visible_line(), 0);
    }
}
