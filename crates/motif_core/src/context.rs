//! DrawContext provides a painter's stack for building scenes.

use crate::{DevicePoint, DeviceRect, Point, Quad, Rect, ScaleFactor, Scene, TextContext, TextRun};
use palette::Srgba;

/// Painter's stack for hierarchical drawing.
pub struct DrawContext<'a> {
    scene: &'a mut Scene,
    scale_factor: ScaleFactor,
    offset_stack: Vec<Point>,
    clip_stack: Vec<Rect>,
}

impl<'a> DrawContext<'a> {
    pub fn new(scene: &'a mut Scene, scale_factor: ScaleFactor) -> Self {
        Self {
            scene,
            scale_factor,
            offset_stack: vec![Point::new(0.0, 0.0)],
            clip_stack: Vec::new(),
        }
    }

    /// Current offset (sum of all pushed offsets).
    fn current_offset(&self) -> Point {
        self.offset_stack.last().copied().unwrap_or_default()
    }

    /// Execute closure with additional offset applied.
    pub fn with_offset<R>(&mut self, offset: Point, f: impl FnOnce(&mut Self) -> R) -> R {
        let current = self.current_offset();
        let new_offset = Point::new(current.x + offset.x, current.y + offset.y);
        self.offset_stack.push(new_offset);
        let result = f(self);
        self.offset_stack.pop();
        result
    }

    /// Execute closure with clip bounds applied.
    pub fn with_clip<R>(&mut self, bounds: Rect, f: impl FnOnce(&mut Self) -> R) -> R {
        // Transform clip bounds by current offset
        let offset = self.current_offset();
        let clipped = Rect::new(
            Point::new(bounds.origin.x + offset.x, bounds.origin.y + offset.y),
            bounds.size,
        );
        self.clip_stack.push(clipped);
        let result = f(self);
        self.clip_stack.pop();
        result
    }

    /// Paint a simple filled quad.
    pub fn paint_quad(&mut self, bounds: Rect, fill: impl Into<Srgba>) {
        let mut quad = Quad::new(self.to_device_rect(bounds), fill);
        self.apply_clip(&mut quad);
        self.scene.push_quad(quad);
    }

    /// Paint a quad with full control.
    pub fn paint(&mut self, mut quad: Quad) {
        self.apply_clip(&mut quad);
        self.scene.push_quad(quad);
    }

    /// Apply current clip stack to quad.
    fn apply_clip(&self, quad: &mut Quad) {
        if let Some(clip) = self.clip_stack.last() {
            quad.clip_bounds = Some(self.scale_factor.scale_rect(*clip));
        }
    }

    /// Convert logical rect to device rect, applying current offset and scale.
    fn to_device_rect(&self, rect: Rect) -> DeviceRect {
        let offset = self.current_offset();
        let origin = Point::new(rect.origin.x + offset.x, rect.origin.y + offset.y);
        let scaled_origin = self.scale_factor.scale_point(origin);
        let scaled_size = self.scale_factor.scale_size(rect.size);
        DeviceRect::new(scaled_origin, scaled_size)
    }

    /// Convert logical point to device point, applying current offset and scale.
    fn to_device_point(&self, point: Point) -> DevicePoint {
        let offset = self.current_offset();
        let origin = Point::new(point.x + offset.x, point.y + offset.y);
        self.scale_factor.scale_point(origin)
    }

    /// Paint text at the given position.
    ///
    /// The position is the baseline origin (left side of first glyph baseline).
    pub fn paint_text(
        &mut self,
        text: &str,
        position: Point,
        font_size: f32,
        color: impl Into<Srgba>,
        text_ctx: &mut TextContext,
    ) {
        let layout = text_ctx.layout_text(text, font_size * self.scale_factor.0);
        let device_origin = self.to_device_point(position);
        let color = color.into();

        for run in layout.glyph_runs_with_font() {
            if let Some(font) = run.font_data {
                let mut text_run = TextRun::new(device_origin, color, run.font_size, font);
                text_run.normalized_coords = run.normalized_coords;

                for glyph in run.glyphs {
                    text_run.push_glyph(glyph.id, glyph.x, glyph.y);
                }

                self.scene.push_text_run(text_run);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Size, TextContext};

    #[test]
    fn offset_stacking() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);

        // Paint at origin
        cx.paint_quad(
            Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );

        // Paint with offset
        cx.with_offset(Point::new(100.0, 50.0), |cx| {
            cx.paint_quad(
                Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)),
                Srgba::new(0.0, 1.0, 0.0, 1.0),
            );
        });

        assert_eq!(scene.quad_count(), 2);

        let quads = scene.quads();
        // First quad at origin
        assert_eq!(quads[0].bounds.origin.x, 0.0);
        assert_eq!(quads[0].bounds.origin.y, 0.0);
        // Second quad offset by (100, 50)
        assert_eq!(quads[1].bounds.origin.x, 100.0);
        assert_eq!(quads[1].bounds.origin.y, 50.0);
    }

    #[test]
    fn nested_offsets() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);

        cx.with_offset(Point::new(10.0, 10.0), |cx| {
            cx.with_offset(Point::new(5.0, 5.0), |cx| {
                cx.paint_quad(
                    Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)),
                    Srgba::new(1.0, 0.0, 0.0, 1.0),
                );
            });
        });

        let quads = scene.quads();
        // Nested offsets should accumulate: 10+5 = 15
        assert_eq!(quads[0].bounds.origin.x, 15.0);
        assert_eq!(quads[0].bounds.origin.y, 15.0);
    }

    #[test]
    fn scale_factor_applied() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(2.0); // 2x HiDPI
        let mut cx = DrawContext::new(&mut scene, scale);

        cx.paint_quad(
            Rect::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );

        let quads = scene.quads();
        // Everything should be scaled by 2
        assert_eq!(quads[0].bounds.origin.x, 20.0);
        assert_eq!(quads[0].bounds.origin.y, 40.0);
        assert_eq!(quads[0].bounds.size.width, 200.0);
        assert_eq!(quads[0].bounds.size.height, 100.0);
    }

    #[test]
    fn clip_bounds_applied() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);

        // Paint without clip - should have no clip bounds
        cx.paint_quad(
            Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 100.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );

        // Paint with clip
        cx.with_clip(Rect::new(Point::new(10.0, 10.0), Size::new(50.0, 50.0)), |cx| {
            cx.paint_quad(
                Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 100.0)),
                Srgba::new(0.0, 1.0, 0.0, 1.0),
            );
        });

        let quads = scene.quads();
        // First quad has no clip
        assert!(quads[0].clip_bounds.is_none());
        // Second quad should have clip bounds
        let clip = quads[1].clip_bounds.expect("should have clip bounds");
        assert_eq!(clip.origin.x, 10.0);
        assert_eq!(clip.origin.y, 10.0);
        assert_eq!(clip.size.width, 50.0);
        assert_eq!(clip.size.height, 50.0);
    }

    #[test]
    fn paint_text_creates_text_runs() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);
        let mut text_ctx = TextContext::new();

        cx.paint_text(
            "Hello",
            Point::new(10.0, 50.0),
            16.0,
            Srgba::new(0.0, 0.0, 0.0, 1.0),
            &mut text_ctx,
        );

        assert!(scene.text_run_count() > 0, "should create text runs");
        let text_run = &scene.text_runs()[0];
        assert!(!text_run.glyphs.is_empty(), "should have glyphs");
        assert_eq!(text_run.origin.x, 10.0);
        assert_eq!(text_run.origin.y, 50.0);
    }

    #[test]
    fn paint_text_respects_offset() {
        let mut scene = Scene::new();
        let scale = ScaleFactor(1.0);
        let mut cx = DrawContext::new(&mut scene, scale);
        let mut text_ctx = TextContext::new();

        cx.with_offset(Point::new(100.0, 200.0), |cx| {
            cx.paint_text(
                "Hi",
                Point::new(10.0, 20.0),
                16.0,
                Srgba::new(0.0, 0.0, 0.0, 1.0),
                &mut text_ctx,
            );
        });

        let text_run = &scene.text_runs()[0];
        // Position should be offset: 100+10=110, 200+20=220
        assert_eq!(text_run.origin.x, 110.0);
        assert_eq!(text_run.origin.y, 220.0);
    }
}
