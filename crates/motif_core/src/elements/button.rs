//! Interactive button element.
//!
//! ```ignore
//! let button_id = ElementId(1);
//! button("Click me", button_id)
//!     .on_click(|| println!("Clicked!"))
//!     .render(cx);
//! ```

use crate::{
    element::{Element, IntoElement, LayoutContext, PaintContext},
    layout::{MeasureContext, NodeId},
    ArcStr, ElementId, Rect, Srgba, TextRun,
};

/// Interactive button element.
pub struct Button {
    label: ArcStr,
    id: ElementId,
    // Visual customization
    background: Srgba,
    hover_background: Srgba,
    press_background: Srgba,
    text_color: Srgba,
    font_size: f32,
    corner_radius: f32,
    padding: f32,
    // State (set externally before paint)
    is_hovered: bool,
    is_pressed: bool,
}

impl Button {
    pub fn new(label: impl Into<ArcStr>, id: ElementId) -> Self {
        Self {
            label: label.into(),
            id,
            background: Srgba::new(0.2, 0.4, 0.8, 1.0),
            hover_background: Srgba::new(0.3, 0.5, 0.9, 1.0),
            press_background: Srgba::new(0.15, 0.3, 0.6, 1.0),
            text_color: Srgba::new(1.0, 1.0, 1.0, 1.0),
            font_size: 14.0,
            corner_radius: 6.0,
            padding: 12.0,
            is_hovered: false,
            is_pressed: false,
        }
    }

    /// Set whether the button is currently hovered.
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.is_hovered = hovered;
        self
    }

    /// Set whether the button is currently pressed.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.is_pressed = pressed;
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Srgba) -> Self {
        self.background = color;
        self
    }

    /// Set the hover background color.
    pub fn hover_background(mut self, color: Srgba) -> Self {
        self.hover_background = color;
        self
    }

    /// Set the press background color.
    pub fn press_background(mut self, color: Srgba) -> Self {
        self.press_background = color;
        self
    }

    /// Set the text color.
    pub fn text_color(mut self, color: Srgba) -> Self {
        self.text_color = color;
        self
    }

    /// Set the font size.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the corner radius.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set the padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Get the element ID.
    pub fn id(&self) -> ElementId {
        self.id
    }
}

impl Element for Button {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> NodeId {
        // Button sizes itself based on text content + padding
        // Use MeasureContext for the text, then add padding in the style
        cx.layout_engine().new_leaf_with_context(
            crate::layout::Style {
                padding: taffy::Rect {
                    left: taffy::style::LengthPercentage::length(self.padding),
                    right: taffy::style::LengthPercentage::length(self.padding),
                    top: taffy::style::LengthPercentage::length(self.padding),
                    bottom: taffy::style::LengthPercentage::length(self.padding),
                },
                ..Default::default()
            },
            MeasureContext::Text {
                content: self.label.to_string(),
                font_size: self.font_size,
            },
        )
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Determine background color based on state
        let bg_color = if self.is_pressed {
            self.press_background
        } else if self.is_hovered {
            self.hover_background
        } else {
            self.background
        };

        // Paint background quad
        let scale = cx.scale_factor().0;
        let mut quad = crate::Quad::new(
            crate::DeviceRect::new(
                crate::DevicePoint::new(bounds.origin.x * scale, bounds.origin.y * scale),
                crate::DeviceSize::new(bounds.size.width * scale, bounds.size.height * scale),
            ),
            bg_color,
        );
        quad.corner_radii = crate::Corners::all(self.corner_radius * scale);
        cx.scene().push_quad(quad);

        // Paint label (centered)
        let scaled_font_size = self.font_size * scale;
        let layout = cx.text_ctx().layout_text(&self.label, scaled_font_size);
        let text_width = layout.width() / scale; // Convert back to logical for centering
        let text_x = bounds.origin.x + (bounds.size.width - text_width) / 2.0;

        // Get baseline offset for correct vertical positioning
        let line_metrics = layout.line_metrics();
        let baseline_offset = line_metrics.first().map(|m| m.baseline).unwrap_or(0.0);
        let text_y = bounds.origin.y + (bounds.size.height + self.font_size) / 2.0;

        let device_origin =
            crate::DevicePoint::new(text_x * scale, text_y * scale - baseline_offset);

        for run in layout.glyph_runs_with_font() {
            if let Some(font) = run.font_data {
                let mut text_run =
                    TextRun::new(device_origin, self.text_color, run.font_size, font);
                text_run.normalized_coords = run.normalized_coords;

                for glyph in run.glyphs {
                    text_run.push_glyph(glyph.id, glyph.x, glyph.y);
                }

                cx.scene().push_text_run(text_run);
            }
        }

        // Register for hit testing
        cx.register_hit(self.id, bounds);
    }
}

use taffy;

impl IntoElement for Button {
    type Element = Button;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Create a button with a label and ID.
pub fn button(label: impl Into<ArcStr>, id: ElementId) -> Button {
    Button::new(label, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::LayoutContext;
    use crate::{HitTree, LayoutEngine, Point, ScaleFactor, Scene, TextContext};

    #[test]
    fn button_registers_hit() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut btn = button("Test", ElementId(1));

        // Request layout
        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = btn.request_layout(&mut layout_cx);

        // Compute layout
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        // Paint
        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        btn.paint(bounds, &mut cx);

        // Should be registered in hit tree at the computed bounds
        let center = Point::new(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );
        assert_eq!(hit_tree.hit_test(center), Some(ElementId(1)));
    }

    #[test]
    fn button_paints_quad() {
        let mut scene = Scene::new();
        let mut text_ctx = TextContext::new();
        let mut hit_tree = HitTree::new();
        let mut layout_engine = LayoutEngine::new();

        let mut btn = button("Test", ElementId(1));

        // Request layout
        let mut layout_cx = LayoutContext::new(&mut layout_engine, &mut text_ctx, ScaleFactor(1.0));
        let node_id = btn.request_layout(&mut layout_cx);

        // Compute layout
        layout_engine.compute_layout(node_id, 800.0, 600.0, &mut text_ctx);

        // Paint
        let bounds = layout_engine.layout_bounds(node_id);
        let mut cx = PaintContext::new(
            &mut scene,
            &mut text_ctx,
            &mut hit_tree,
            &layout_engine,
            ScaleFactor(1.0),
        );
        btn.paint(bounds, &mut cx);

        assert!(scene.quad_count() > 0);
    }

    #[test]
    fn button_visual_states() {
        let normal = button("Test", ElementId(1));
        let hovered = button("Test", ElementId(2)).hovered(true);
        let pressed = button("Test", ElementId(3)).pressed(true);

        // Just verify they can be created with different states
        assert!(!normal.is_hovered);
        assert!(!normal.is_pressed);
        assert!(hovered.is_hovered);
        assert!(pressed.is_pressed);
    }
}
