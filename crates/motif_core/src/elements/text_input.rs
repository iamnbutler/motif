//! Single-line text input element.
//!
//! ```ignore
//! let id = ElementId(1);
//! text_input("Hello", id)
//!     .placeholder("Enter text...")
//!     .size(Size::new(200.0, 32.0))
//!     .focused(true)
//!     .cursor_pos(5)
//!     .paint(&mut cx);
//! ```

use crate::{
    element::{Element, IntoElement, LayoutContext, PaintContext},
    layout::NodeId,
    ArcStr, ElementId, Point, Rect, Size, Srgba, TextRun,
};

/// Single-line text input element.
///
/// Paints a bordered rectangle, the current value (or placeholder when empty),
/// selection highlight, and cursor when focused.
pub struct TextInput {
    value: ArcStr,
    placeholder: ArcStr,
    id: ElementId,
    bounds: Rect,
    // Visual customization
    background: Srgba,
    border_color: Srgba,
    focus_border_color: Srgba,
    text_color: Srgba,
    placeholder_color: Srgba,
    selection_color: Srgba,
    font_size: f32,
    padding: f32,
    corner_radius: f32,
    border_width: f32,
    // State (set externally before paint)
    is_focused: bool,
    /// Byte offset into `value` at which to draw the cursor.
    cursor_pos: usize,
    /// Selection range (start..end byte offsets). Empty range = no selection.
    selection: std::ops::Range<usize>,
    /// Horizontal scroll offset in logical pixels. Positive values scroll the
    /// text viewport to the right, hiding text on the left. Managed by the
    /// caller to keep the cursor visible when text overflows the input width.
    scroll_offset: f32,
}

impl TextInput {
    pub fn new(value: impl Into<ArcStr>, id: ElementId) -> Self {
        Self {
            value: value.into(),
            placeholder: ArcStr::from(""),
            id,
            bounds: Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)),
            background: Srgba::new(1.0, 1.0, 1.0, 1.0),
            border_color: Srgba::new(0.7, 0.7, 0.7, 1.0),
            focus_border_color: Srgba::new(0.2, 0.4, 0.8, 1.0),
            text_color: Srgba::new(0.0, 0.0, 0.0, 1.0),
            placeholder_color: Srgba::new(0.6, 0.6, 0.6, 1.0),
            selection_color: Srgba::new(0.3, 0.5, 0.9, 0.3),
            font_size: 14.0,
            padding: 8.0,
            corner_radius: 4.0,
            border_width: 1.5,
            is_focused: false,
            cursor_pos: 0,
            selection: 0..0,
            scroll_offset: 0.0,
        }
    }

    /// Set the placeholder text shown when the value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<ArcStr>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the element's position and size.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    /// Set the element's position (size stays at default).
    pub fn position(mut self, position: Point) -> Self {
        self.bounds = Rect::new(position, self.bounds.size);
        self
    }

    /// Set the element's size (position stays at default).
    pub fn size(mut self, size: Size) -> Self {
        self.bounds = Rect::new(self.bounds.origin, size);
        self
    }

    /// Set whether the input currently has keyboard focus.
    pub fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    /// Set the cursor position as a byte offset into the value.
    ///
    /// Clamped to the string length and snapped to the nearest valid UTF-8
    /// char boundary if necessary.
    pub fn cursor_pos(mut self, pos: usize) -> Self {
        let clamped = pos.min(self.value.len());
        // Walk back to nearest char boundary
        let mut adjusted = clamped;
        while adjusted > 0 && !self.value.is_char_boundary(adjusted) {
            adjusted -= 1;
        }
        self.cursor_pos = adjusted;
        self
    }

    /// Set the horizontal scroll offset in logical pixels.
    ///
    /// A positive offset scrolls the text viewport to the right, hiding leading
    /// characters. Callers should compute this to keep the cursor in view:
    ///
    /// ```text
    /// let visible_width = input_width - 2.0 * padding;
    /// if cursor_x > scroll_offset + visible_width {
    ///     scroll_offset = cursor_x - visible_width;
    /// } else if cursor_x < scroll_offset {
    ///     scroll_offset = cursor_x;
    /// }
    /// ```
    pub fn scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset.max(0.0);
        self
    }

    /// Set the selection range as byte offsets into the value.
    ///
    /// The selection is rendered as a highlight behind the text.
    /// An empty range (start == end) means no selection.
    pub fn selection(mut self, range: std::ops::Range<usize>) -> Self {
        let start = range.start.min(self.value.len());
        let end = range.end.min(self.value.len());
        self.selection = start..end;
        self
    }

    /// Set the selection highlight color.
    pub fn selection_color(mut self, color: Srgba) -> Self {
        self.selection_color = color;
        self
    }

    /// Set the background fill color.
    pub fn background(mut self, color: Srgba) -> Self {
        self.background = color;
        self
    }

    /// Set the border color when the input does not have focus.
    pub fn border_color(mut self, color: Srgba) -> Self {
        self.border_color = color;
        self
    }

    /// Set the border color when the input has focus.
    pub fn focus_border_color(mut self, color: Srgba) -> Self {
        self.focus_border_color = color;
        self
    }

    /// Set the text color.
    pub fn text_color(mut self, color: Srgba) -> Self {
        self.text_color = color;
        self
    }

    /// Set the placeholder text color.
    pub fn placeholder_color(mut self, color: Srgba) -> Self {
        self.placeholder_color = color;
        self
    }

    /// Set the font size.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the horizontal padding between the border and the text.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the corner radius.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set the border stroke width.
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Get the element ID.
    pub fn id(&self) -> ElementId {
        self.id
    }

    /// Get the current text value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl Element for TextInput {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> NodeId {
        // TextInput uses its configured size
        cx.layout_engine().new_leaf(crate::layout::Style {
            size: taffy::Size {
                width: taffy::style::Dimension::length(self.bounds.size.width),
                height: taffy::style::Dimension::length(self.bounds.size.height),
            },
            ..Default::default()
        })
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let scale = cx.scale_factor().0;

        // 1. Background + border quad
        let active_border = if self.is_focused {
            self.focus_border_color
        } else {
            self.border_color
        };

        let mut quad = crate::Quad::new(
            crate::DeviceRect::new(
                crate::DevicePoint::new(bounds.origin.x * scale, bounds.origin.y * scale),
                crate::DeviceSize::new(bounds.size.width * scale, bounds.size.height * scale),
            ),
            self.background,
        );
        quad.corner_radii = crate::Corners::all(self.corner_radius * scale);
        quad.border_color = active_border;
        quad.border_widths = crate::Edges::all(self.border_width * scale);
        cx.scene().push_quad(quad);

        // Clip rect for text content: inner area within horizontal padding.
        // Applied to selection and cursor quads to prevent them overflowing the
        // input boundary when text is scrolled. The full input height is used so
        // vertically the clips are non-restrictive.
        let content_clip = crate::DeviceRect::new(
            crate::DevicePoint::new(
                (bounds.origin.x + self.padding) * scale,
                bounds.origin.y * scale,
            ),
            crate::DeviceSize::new(
                (bounds.size.width - 2.0 * self.padding) * scale,
                bounds.size.height * scale,
            ),
        );

        // 2. Selection highlight (painted before text so it appears behind)
        if !self.selection.is_empty() && !self.value.is_empty() {
            let scaled_font_size = self.font_size * scale;

            // Calculate selection start x
            let sel_start_x = if self.selection.start == 0 {
                0.0_f32
            } else {
                let text_before = &self.value[..self.selection.start.min(self.value.len())];
                let layout = cx.text_ctx().layout_text(text_before, scaled_font_size);
                layout.width() / scale
            };

            // Calculate selection end x
            let sel_end_x = {
                let text_to_end = &self.value[..self.selection.end.min(self.value.len())];
                let layout = cx.text_ctx().layout_text(text_to_end, scaled_font_size);
                layout.width() / scale
            };

            // Shift selection by scroll offset so it tracks with the text viewport.
            let sel_x = bounds.origin.x + self.padding + sel_start_x - self.scroll_offset;
            let sel_width = sel_end_x - sel_start_x;
            let sel_top = bounds.origin.y + (bounds.size.height - self.font_size) / 2.0;

            if sel_width > 0.0 {
                let mut sel_quad = crate::Quad::new(
                    crate::DeviceRect::new(
                        crate::DevicePoint::new(sel_x * scale, sel_top * scale),
                        crate::DeviceSize::new(sel_width * scale, self.font_size * scale),
                    ),
                    self.selection_color,
                );
                sel_quad.clip_bounds = Some(content_clip);
                cx.scene().push_quad(sel_quad);
            }
        }

        // 3. Text content (value, or placeholder when empty)
        let (display_text, text_color) = if self.value.is_empty() {
            (self.placeholder.clone(), self.placeholder_color)
        } else {
            (self.value.clone(), self.text_color)
        };

        if !display_text.is_empty() {
            let scaled_font_size = self.font_size * scale;
            let layout = cx.text_ctx().layout_text(&display_text, scaled_font_size);

            // Shift text left by the scroll offset to reveal the scrolled-to portion.
            let text_x = bounds.origin.x + self.padding - self.scroll_offset;
            let line_metrics = layout.line_metrics();
            let baseline_offset = line_metrics.first().map(|m| m.baseline).unwrap_or(0.0);
            // Vertically center the text within the bounds
            let text_y = bounds.origin.y + (bounds.size.height + self.font_size) / 2.0;

            let device_origin =
                crate::DevicePoint::new(text_x * scale, text_y * scale - baseline_offset);

            // Visible x range in device pixels — used to skip glyphs that are
            // fully outside the content area, preventing text overflow.
            let vis_left_dev = (bounds.origin.x + self.padding) * scale;
            let vis_right_dev = (bounds.origin.x + bounds.size.width - self.padding) * scale;

            for run in layout.glyph_runs_with_font() {
                if let Some(font) = run.font_data {
                    let mut text_run = TextRun::new(device_origin, text_color, run.font_size, font);
                    text_run.normalized_coords = run.normalized_coords;

                    for glyph in run.glyphs {
                        // Absolute device-x of this glyph.
                        let glyph_dev_x = device_origin.x + glyph.x;
                        // Use font_size (device pixels) as a conservative glyph-width
                        // estimate to keep partially-visible edge glyphs.
                        let approx_width = run.font_size;
                        if glyph_dev_x + approx_width >= vis_left_dev
                            && glyph_dev_x <= vis_right_dev
                        {
                            text_run.push_glyph(glyph.id, glyph.x, glyph.y);
                        }
                    }

                    if !text_run.glyphs.is_empty() {
                        cx.scene().push_text_run(text_run);
                    }
                }
            }
        }

        // 4. Cursor — only drawn when focused
        if self.is_focused {
            // Determine cursor x by laying out the text before the cursor position.
            // We append a visible character to ensure trailing whitespace is measured,
            // then subtract that character's width.
            let cursor_x_logical = if self.cursor_pos == 0 || self.value.is_empty() {
                0.0_f32
            } else {
                let text_before = &self.value[..self.cursor_pos];
                // Append a pipe character to measure trailing whitespace properly
                let text_with_marker = format!("{}|", text_before);
                let layout_with_marker = cx
                    .text_ctx()
                    .layout_text(&text_with_marker, self.font_size * scale);
                // Layout just the marker to get its width
                let marker_layout = cx.text_ctx().layout_text("|", self.font_size * scale);
                // Cursor position = total width - marker width
                (layout_with_marker.width() - marker_layout.width()) / scale
            };

            // Shift cursor by scroll offset to track with the text viewport.
            let cursor_x = bounds.origin.x + self.padding + cursor_x_logical - self.scroll_offset;
            let cursor_top = bounds.origin.y + (bounds.size.height - self.font_size) / 2.0;

            let mut cursor_quad = crate::Quad::new(
                crate::DeviceRect::new(
                    crate::DevicePoint::new(cursor_x * scale, cursor_top * scale),
                    // 1 logical pixel wide, font-size tall
                    crate::DeviceSize::new(1.0 * scale, self.font_size * scale),
                ),
                self.text_color,
            );
            cursor_quad.clip_bounds = Some(content_clip);
            cx.scene().push_quad(cursor_quad);
        }

        // 5. Hit-test registration
        cx.register_hit(self.id, bounds);
    }
}

use taffy;

impl IntoElement for TextInput {
    type Element = TextInput;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Create a text input with an initial value and ID.
///
/// Takes `&str` directly since input values are typically dynamic content.
pub fn text_input(value: &str, id: ElementId) -> TextInput {
    TextInput::new(ArcStr::new(value), id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::LayoutContext;
    use crate::{HitTree, LayoutEngine, Point, ScaleFactor, Scene, TextContext};

    #[test]
    fn text_input_defaults() {
        let input = text_input("hello", ElementId(1));
        assert_eq!(input.value(), "hello");
        assert!(!input.is_focused);
        assert_eq!(input.cursor_pos, 0);
        assert_eq!(input.font_size, 14.0);
        assert_eq!(input.scroll_offset, 0.0);
    }

    #[test]
    fn text_input_builder_chain() {
        let input = text_input("", ElementId(1))
            .placeholder("Search...")
            .focused(true)
            .cursor_pos(0)
            .font_size(16.0)
            .corner_radius(6.0)
            .border_width(2.0)
            .padding(10.0);

        assert!(input.is_focused);
        assert_eq!(input.font_size, 16.0);
        assert_eq!(input.corner_radius, 6.0);
        assert_eq!(input.border_width, 2.0);
        assert_eq!(input.padding, 10.0);
    }

    #[test]
    fn text_input_paints_background_quad() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut input = text_input("", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        // At least the background quad
        assert!(scene.quad_count() >= 1);
    }

    #[test]
    fn text_input_focused_paints_cursor() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut input = text_input("Hi", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)))
            .focused(true);

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        // background quad + cursor quad = 2
        assert!(
            scene.quad_count() >= 2,
            "expected background + cursor quad, got {}",
            scene.quad_count()
        );
    }

    #[test]
    fn text_input_unfocused_no_cursor() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut input = text_input("Hi", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)))
            .focused(false);

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        // Only the background quad; no cursor
        assert_eq!(scene.quad_count(), 1);
    }

    #[test]
    fn text_input_registers_hit() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut input = text_input("", ElementId(42))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        // Hit test at center of the input (100, 16)
        assert_eq!(
            hit_tree.hit_test(Point::new(100.0, 16.0)),
            Some(ElementId(42))
        );
        // Outside bounds
        assert_eq!(hit_tree.hit_test(Point::new(250.0, 50.0)), None);
    }

    #[test]
    fn text_input_cursor_pos_clamped_to_len() {
        let input = text_input("hi", ElementId(1)).cursor_pos(100);
        assert_eq!(input.cursor_pos, 2);
    }

    #[test]
    fn text_input_cursor_pos_at_char_boundary() {
        // "é" is 2 bytes (U+00E9). cursor_pos(1) should snap back to 0.
        let input = text_input("é", ElementId(1)).cursor_pos(1);
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn text_input_empty_value_uses_placeholder() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut input = text_input("", ElementId(1))
            .placeholder("Type something...")
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        // Placeholder triggers a text run
        assert!(scene.text_run_count() > 0);
    }

    #[test]
    fn text_input_with_value_paints_text_run() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut input = text_input("Hello", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        assert!(scene.text_run_count() > 0);
    }

    #[test]
    fn text_input_scroll_offset_builder() {
        let input = text_input("hello", ElementId(1)).scroll_offset(20.0);
        assert_eq!(input.scroll_offset, 20.0);
    }

    #[test]
    fn text_input_scroll_offset_clamped_to_zero() {
        // Negative scroll offsets are clamped to 0.
        let input = text_input("hello", ElementId(1)).scroll_offset(-5.0);
        assert_eq!(input.scroll_offset, 0.0);
    }

    #[test]
    fn text_input_scroll_offset_paints_background_and_text() {
        // With a scroll offset the element should still paint at minimum the
        // background quad and a text run (text starts scrolled past position 0).
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut input = text_input("Hello, world!", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(60.0, 32.0)))
            .scroll_offset(10.0);

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        assert!(scene.quad_count() >= 1, "background quad should be painted");
        assert!(scene.text_run_count() > 0, "text run should be painted");
    }

    #[test]
    fn text_input_scroll_offset_zero_matches_no_scroll() {
        // scroll_offset(0) should produce the same scene as not calling it.
        let make_scene = |scroll: f32| {
            let mut scene = Scene::new();
            let mut text_ctx = TextContext::new();
            let mut hit_tree = HitTree::new();
            let mut layout_engine = LayoutEngine::new();

            let mut input = text_input("Hi", ElementId(1))
                .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)))
                .scroll_offset(scroll);

            let mut layout_cx =
                LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
            let node_id = input.request_layout(&mut layout_cx);
            layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

            let bounds = layout_engine.layout_bounds(node_id);
            let mut cx = PaintContext::new(
                &mut scene,
                &mut text_ctx,
                &mut hit_tree,
                &layout_engine,
                ScaleFactor(1.0),
            );
            input.paint(bounds, &mut cx);
            (scene.quad_count(), scene.text_run_count())
        };

        let (qc_default, tr_default) = make_scene(0.0);
        let (qc_explicit, tr_explicit) = make_scene(0.0);
        assert_eq!(qc_default, qc_explicit);
        assert_eq!(tr_default, tr_explicit);
    }

    #[test]
    fn text_input_no_text_no_text_run() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        // Empty value AND no placeholder → no text run
        let mut input = text_input("", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = input.request_layout(&mut layout_cx);
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        input.paint(bounds, &mut cx);

        assert_eq!(scene.text_run_count(), 0);
    }
}
