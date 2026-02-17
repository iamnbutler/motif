//! Text element for rendering text content.

use crate::element::{Element, IntoElement, PaintContext};
use crate::{Point, ArcStr, TextRun};
use palette::Srgba;

/// A text element that renders a string at a given position.
///
/// ```ignore
/// text("Hello, World!")
///     .position(Point::new(50.0, 100.0))
///     .font_size(24.0)
///     .color(Srgba::new(1.0, 1.0, 1.0, 1.0))
/// ```
pub struct Text {
    content: ArcStr,
    position: Point,
    font_size: f32,
    color: Srgba,
}

impl Text {
    pub fn new(content: impl Into<ArcStr>) -> Self {
        Self {
            content: content.into(),
            position: Point::new(0.0, 0.0),
            font_size: 16.0,
            color: Srgba::new(1.0, 1.0, 1.0, 1.0),
        }
    }

    pub fn position(mut self, position: Point) -> Self {
        self.position = position;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: impl Into<Srgba>) -> Self {
        self.color = color.into();
        self
    }
}

impl Element for Text {
    fn paint(&mut self, cx: &mut PaintContext) {
        if self.content.is_empty() {
            return;
        }

        let scale = cx.scale_factor();
        let scaled_font_size = self.font_size * scale.0;
        let layout = cx.text_ctx().layout_text(&self.content, scaled_font_size);

        let device_position = scale.scale_point(self.position);

        // Get baseline offset for correct vertical positioning
        let line_metrics = layout.line_metrics();
        let baseline_offset = line_metrics.first().map(|m| m.baseline).unwrap_or(0.0);

        let device_origin = crate::DevicePoint::new(
            device_position.x,
            device_position.y - baseline_offset,
        );

        for run in layout.glyph_runs_with_font() {
            if let Some(font) = run.font_data {
                let mut text_run =
                    TextRun::new(device_origin, self.color, run.font_size, font);
                text_run.normalized_coords = run.normalized_coords;

                for glyph in run.glyphs {
                    text_run.push_glyph(glyph.id, glyph.x, glyph.y);
                }

                cx.scene().push_text_run(text_run);
            }
        }
    }
}

impl IntoElement for Text {
    type Element = Text;
    fn into_element(self) -> Self::Element {
        self
    }
}

/// Create a new Text element.
pub fn text(content: impl Into<ArcStr>) -> Text {
    Text::new(content)
}

// Allow strings to be used directly as elements.
impl IntoElement for &'static str {
    type Element = Text;
    fn into_element(self) -> Text {
        Text::new(ArcStr::from(self))
    }
}

impl IntoElement for String {
    type Element = Text;
    fn into_element(self) -> Text {
        Text::new(ArcStr::from(self))
    }
}

impl IntoElement for ArcStr {
    type Element = Text;
    fn into_element(self) -> Text {
        Text::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_builder_defaults() {
        let t = text("hello");
        assert_eq!(t.content, "hello");
        assert_eq!(t.font_size, 16.0);
    }

    #[test]
    fn text_builder_chain() {
        let t = text("hello")
            .position(Point::new(10.0, 20.0))
            .font_size(24.0)
            .color(Srgba::new(1.0, 0.0, 0.0, 1.0));

        assert_eq!(t.position.x, 10.0);
        assert_eq!(t.font_size, 24.0);
    }

    #[test]
    fn string_into_element() {
        let t: Text = "hello".into_element();
        assert_eq!(t.content, "hello");
    }

    #[test]
    fn owned_string_into_element() {
        let t: Text = String::from("hello").into_element();
        assert_eq!(t.content, "hello");
    }
}
