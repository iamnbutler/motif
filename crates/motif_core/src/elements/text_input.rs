//! Single-line text input element.
//!
//! ```ignore
//! let id = ElementId(1);
//! text_input("Hello", id)
//!     .placeholder("Enter text...")
//!     .bounds(Rect::new(Point::new(10.0, 10.0), Size::new(200.0, 32.0)))
//!     .focused(true)
//!     .cursor_pos(5)
//!     .paint(&mut cx);
//! ```

use crate::{
    element::{Element, IntoElement, PaintContext},
    ArcStr, ElementId, Point, Rect, Size, Srgba, TextRun,
};

/// Single-line text input element.
///
/// Paints a bordered rectangle, the current value (or placeholder when empty),
/// and a blinking-cursor quad when the field has focus.
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
    font_size: f32,
    padding: f32,
    corner_radius: f32,
    border_width: f32,
    // State (set externally before paint)
    is_focused: bool,
    /// Byte offset into `value` at which to draw the cursor.
    /// Always kept at a valid UTF-8 char boundary.
    cursor_pos: usize,
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
            font_size: 14.0,
            padding: 8.0,
            corner_radius: 4.0,
            border_width: 1.5,
            is_focused: false,
            cursor_pos: 0,
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
    fn paint(&mut self, cx: &mut PaintContext) {
        let scale = cx.scale_factor().0;

        // 1. Background + border quad
        let active_border = if self.is_focused {
            self.focus_border_color
        } else {
            self.border_color
        };

        let mut quad = crate::Quad::new(
            crate::DeviceRect::new(
                crate::DevicePoint::new(self.bounds.origin.x * scale, self.bounds.origin.y * scale),
                crate::DeviceSize::new(
                    self.bounds.size.width * scale,
                    self.bounds.size.height * scale,
                ),
            ),
            self.background,
        );
        quad.corner_radii = crate::Corners::all(self.corner_radius * scale);
        quad.border_color = active_border;
        quad.border_widths = crate::Edges::all(self.border_width * scale);
        cx.scene().push_quad(quad);

        // 2. Text content (value, or placeholder when empty)
        let (display_text, text_color) = if self.value.is_empty() {
            (self.placeholder.clone(), self.placeholder_color)
        } else {
            (self.value.clone(), self.text_color)
        };

        if !display_text.is_empty() {
            let scaled_font_size = self.font_size * scale;
            let layout = cx.text_ctx().layout_text(&display_text, scaled_font_size);

            let text_x = self.bounds.origin.x + self.padding;
            let line_metrics = layout.line_metrics();
            let baseline_offset = line_metrics.first().map(|m| m.baseline).unwrap_or(0.0);
            // Vertically center the text within the bounds
            let text_y = self.bounds.origin.y + (self.bounds.size.height + self.font_size) / 2.0;

            let device_origin =
                crate::DevicePoint::new(text_x * scale, text_y * scale - baseline_offset);

            for run in layout.glyph_runs_with_font() {
                if let Some(font) = run.font_data {
                    let mut text_run = TextRun::new(device_origin, text_color, run.font_size, font);
                    text_run.normalized_coords = run.normalized_coords;

                    for glyph in run.glyphs {
                        text_run.push_glyph(glyph.id, glyph.x, glyph.y);
                    }

                    cx.scene().push_text_run(text_run);
                }
            }
        }

        // 3. Cursor — only drawn when focused
        if self.is_focused {
            // Determine cursor x by laying out the text before the cursor position
            let cursor_x_logical = if self.cursor_pos == 0 || self.value.is_empty() {
                0.0_f32
            } else {
                let text_before = &self.value[..self.cursor_pos];
                let layout = cx
                    .text_ctx()
                    .layout_text(text_before, self.font_size * scale);
                layout.width() / scale
            };

            let cursor_x = self.bounds.origin.x + self.padding + cursor_x_logical;
            let cursor_top =
                self.bounds.origin.y + (self.bounds.size.height - self.font_size) / 2.0;

            let cursor_quad = crate::Quad::new(
                crate::DeviceRect::new(
                    crate::DevicePoint::new(cursor_x * scale, cursor_top * scale),
                    // 1 logical pixel wide, font-size tall
                    crate::DeviceSize::new(1.0 * scale, self.font_size * scale),
                ),
                self.text_color,
            );
            cx.scene().push_quad(cursor_quad);
        }

        // 4. Hit-test registration
        cx.register_hit(self.id, self.bounds);
    }
}

impl IntoElement for TextInput {
    type Element = TextInput;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Create a text input with an initial value and ID.
pub fn text_input(value: impl Into<ArcStr>, id: ElementId) -> TextInput {
    TextInput::new(value, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HitTree, ScaleFactor, Scene, TextContext};

    fn make_cx<'a>(
        scene: &'a mut Scene,
        text_ctx: &'a mut TextContext,
        hit_tree: &'a mut HitTree,
    ) -> PaintContext<'a> {
        PaintContext::new(scene, text_ctx, hit_tree, ScaleFactor(1.0))
    }

    #[test]
    fn text_input_defaults() {
        let input = text_input("hello", ElementId(1));
        assert_eq!(input.value(), "hello");
        assert!(!input.is_focused);
        assert_eq!(input.cursor_pos, 0);
        assert_eq!(input.font_size, 14.0);
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

        let mut input = text_input("", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut cx = make_cx(&mut scene, &mut text_ctx, &mut hit_tree);
        input.paint(&mut cx);

        // At least the background quad
        assert!(scene.quad_count() >= 1);
    }

    #[test]
    fn text_input_focused_paints_cursor() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut input = text_input("Hi", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)))
            .focused(true);

        let mut cx = make_cx(&mut scene, &mut text_ctx, &mut hit_tree);
        input.paint(&mut cx);

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

        let mut input = text_input("Hi", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)))
            .focused(false);

        let mut cx = make_cx(&mut scene, &mut text_ctx, &mut hit_tree);
        input.paint(&mut cx);

        // Only the background quad; no cursor
        assert_eq!(scene.quad_count(), 1);
    }

    #[test]
    fn text_input_registers_hit() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut input = text_input("", ElementId(42))
            .bounds(Rect::new(Point::new(10.0, 10.0), Size::new(200.0, 32.0)));

        let mut cx = make_cx(&mut scene, &mut text_ctx, &mut hit_tree);
        input.paint(&mut cx);

        assert_eq!(
            hit_tree.hit_test(Point::new(50.0, 26.0)),
            Some(ElementId(42))
        );
        assert_eq!(hit_tree.hit_test(Point::new(5.0, 5.0)), None);
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

        let mut input = text_input("", ElementId(1))
            .placeholder("Type something...")
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut cx = make_cx(&mut scene, &mut text_ctx, &mut hit_tree);
        input.paint(&mut cx);

        // Placeholder triggers a text run
        assert!(scene.text_run_count() > 0);
    }

    #[test]
    fn text_input_with_value_paints_text_run() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        let mut input = text_input("Hello", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut cx = make_cx(&mut scene, &mut text_ctx, &mut hit_tree);
        input.paint(&mut cx);

        assert!(scene.text_run_count() > 0);
    }

    #[test]
    fn text_input_no_text_no_text_run() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();

        // Empty value AND no placeholder → no text run
        let mut input = text_input("", ElementId(1))
            .bounds(Rect::new(Point::new(0.0, 0.0), Size::new(200.0, 32.0)));

        let mut cx = make_cx(&mut scene, &mut text_ctx, &mut hit_tree);
        input.paint(&mut cx);

        assert_eq!(scene.text_run_count(), 0);
    }
}
