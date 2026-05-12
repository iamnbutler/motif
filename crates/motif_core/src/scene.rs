//! Scene holds primitives for rendering.

use crate::{Corners, DevicePoint, DeviceRect, Edges, FontData};
use palette::Srgba;

/// A filled/stroked rectangle with optional rounded corners.
#[derive(Clone, Debug)]
pub struct Quad {
    pub bounds: DeviceRect,
    pub background: Srgba,
    pub border_color: Srgba,
    pub border_widths: Edges<f32>,
    pub corner_radii: Corners<f32>,
    /// Optional clip bounds in device pixels. Fragments outside are discarded.
    pub clip_bounds: Option<DeviceRect>,
}

impl Quad {
    pub fn new(bounds: DeviceRect, background: impl Into<Srgba>) -> Self {
        Self {
            bounds,
            background: background.into(),
            border_color: Srgba::new(0.0, 0.0, 0.0, 0.0),
            border_widths: Edges::default(),
            corner_radii: Corners::default(),
            clip_bounds: None,
        }
    }
}

/// A positioned glyph within a text run.
#[derive(Clone, Debug)]
pub struct GlyphInstance {
    /// Glyph ID in the font.
    pub glyph_id: u32,
    /// X offset from run origin.
    pub x: f32,
    /// Y offset from run baseline.
    pub y: f32,
}

/// A run of glyphs to render as text.
#[derive(Clone, Debug)]
pub struct TextRun {
    /// Origin point (baseline start) in device pixels.
    pub origin: DevicePoint,
    /// Text color.
    pub color: Srgba,
    /// Font size in pixels.
    pub font_size: f32,
    /// Font data for rasterization.
    pub font: FontData,
    /// Normalized coordinates for variable fonts.
    pub normalized_coords: Vec<i16>,
    /// Glyphs to render.
    pub glyphs: Vec<GlyphInstance>,
}

impl TextRun {
    pub fn new(
        origin: DevicePoint,
        color: impl Into<Srgba>,
        font_size: f32,
        font: FontData,
    ) -> Self {
        Self {
            origin,
            color: color.into(),
            font_size,
            font,
            normalized_coords: Vec::new(),
            glyphs: Vec::new(),
        }
    }

    pub fn with_normalized_coords(mut self, coords: Vec<i16>) -> Self {
        self.normalized_coords = coords;
        self
    }

    pub fn push_glyph(&mut self, glyph_id: u32, x: f32, y: f32) {
        self.glyphs.push(GlyphInstance { glyph_id, x, y });
    }
}

/// Holds all primitives for a frame, ready for rendering.
#[derive(Default)]
pub struct Scene {
    quads: Vec<Quad>,
    text_runs: Vec<TextRun>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all primitives, reusing allocations.
    pub fn clear(&mut self) {
        self.quads.clear();
        self.text_runs.clear();
    }

    pub fn push_quad(&mut self, quad: Quad) {
        self.quads.push(quad);
    }

    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub fn quad_count(&self) -> usize {
        self.quads.len()
    }

    pub fn push_text_run(&mut self, text_run: TextRun) {
        self.text_runs.push(text_run);
    }

    pub fn text_runs(&self) -> &[TextRun] {
        &self.text_runs
    }

    pub fn text_run_count(&self) -> usize {
        self.text_runs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Corners, DevicePoint, DeviceRect, DeviceSize, Edges};
    use linebender_resource_handle::{Blob, FontData};
    use palette::Srgba;

    fn dummy_font() -> FontData {
        FontData::new(Blob::from(vec![0u8; 4]), 0)
    }

    fn red() -> Srgba {
        Srgba::new(1.0, 0.0, 0.0, 1.0)
    }

    fn unit_rect() -> DeviceRect {
        DeviceRect::new(DevicePoint::new(0.0, 0.0), DeviceSize::new(100.0, 50.0))
    }

    // --- Quad tests ---

    #[test]
    fn quad_new_has_transparent_border_color() {
        let q = Quad::new(unit_rect(), red());
        assert_eq!(q.border_color, Srgba::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn quad_new_has_zero_border_widths() {
        let q = Quad::new(unit_rect(), red());
        assert_eq!(q.border_widths, Edges::default());
    }

    #[test]
    fn quad_new_has_no_clip_bounds() {
        let q = Quad::new(unit_rect(), red());
        assert!(q.clip_bounds.is_none());
    }

    #[test]
    fn quad_new_has_zero_corner_radii() {
        let q = Quad::new(unit_rect(), red());
        assert_eq!(q.corner_radii, Corners::default());
    }

    #[test]
    fn quad_clip_bounds_can_be_set() {
        let mut q = Quad::new(unit_rect(), red());
        let clip = DeviceRect::new(DevicePoint::new(10.0, 10.0), DeviceSize::new(50.0, 30.0));
        q.clip_bounds = Some(clip);
        assert!(q.clip_bounds.is_some());
    }

    // --- TextRun tests ---

    #[test]
    fn text_run_new_has_empty_glyphs() {
        let run = TextRun::new(DevicePoint::new(10.0, 20.0), red(), 16.0, dummy_font());
        assert!(run.glyphs.is_empty());
    }

    #[test]
    fn text_run_new_has_empty_normalized_coords() {
        let run = TextRun::new(DevicePoint::new(0.0, 0.0), red(), 12.0, dummy_font());
        assert!(run.normalized_coords.is_empty());
    }

    #[test]
    fn text_run_push_glyph_stores_correct_fields() {
        let mut run = TextRun::new(DevicePoint::new(0.0, 0.0), red(), 16.0, dummy_font());
        run.push_glyph(42, 1.5, 2.5);
        assert_eq!(run.glyphs.len(), 1);
        assert_eq!(run.glyphs[0].glyph_id, 42);
        assert_eq!(run.glyphs[0].x, 1.5);
        assert_eq!(run.glyphs[0].y, 2.5);
    }

    #[test]
    fn text_run_with_normalized_coords_sets_coords() {
        let run = TextRun::new(DevicePoint::new(0.0, 0.0), red(), 16.0, dummy_font())
            .with_normalized_coords(vec![100, 200]);
        assert_eq!(run.normalized_coords, vec![100i16, 200i16]);
    }

    #[test]
    fn text_run_multiple_glyphs_accumulate() {
        let mut run = TextRun::new(DevicePoint::new(0.0, 0.0), red(), 16.0, dummy_font());
        run.push_glyph(1, 0.0, 0.0);
        run.push_glyph(2, 10.0, 0.0);
        run.push_glyph(3, 20.0, 0.0);
        assert_eq!(run.glyphs.len(), 3);
    }

    // --- Scene tests ---

    #[test]
    fn scene_new_is_empty() {
        let scene = Scene::new();
        assert_eq!(scene.quad_count(), 0);
        assert_eq!(scene.text_run_count(), 0);
    }

    #[test]
    fn scene_default_is_empty() {
        let scene = Scene::default();
        assert_eq!(scene.quad_count(), 0);
        assert_eq!(scene.text_run_count(), 0);
    }

    #[test]
    fn scene_push_quad_increments_count() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(unit_rect(), red()));
        assert_eq!(scene.quad_count(), 1);
    }

    #[test]
    fn scene_quads_returns_pushed_quads() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(unit_rect(), red()));
        assert_eq!(scene.quads().len(), 1);
    }

    #[test]
    fn scene_push_text_run_increments_count() {
        let mut scene = Scene::new();
        let run = TextRun::new(DevicePoint::new(0.0, 0.0), red(), 16.0, dummy_font());
        scene.push_text_run(run);
        assert_eq!(scene.text_run_count(), 1);
    }

    #[test]
    fn scene_text_runs_returns_pushed_runs() {
        let mut scene = Scene::new();
        let run = TextRun::new(DevicePoint::new(0.0, 0.0), red(), 16.0, dummy_font());
        scene.push_text_run(run);
        assert_eq!(scene.text_runs().len(), 1);
    }

    #[test]
    fn scene_clear_removes_all_primitives() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(unit_rect(), red()));
        let run = TextRun::new(DevicePoint::new(0.0, 0.0), red(), 16.0, dummy_font());
        scene.push_text_run(run);
        scene.clear();
        assert_eq!(scene.quad_count(), 0);
        assert_eq!(scene.text_run_count(), 0);
    }

    #[test]
    fn scene_clear_allows_reuse() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(unit_rect(), red()));
        scene.clear();
        scene.push_quad(Quad::new(unit_rect(), red()));
        assert_eq!(scene.quad_count(), 1);
    }

    #[test]
    fn scene_multiple_quads_accumulate() {
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(unit_rect(), red()));
        scene.push_quad(Quad::new(unit_rect(), red()));
        scene.push_quad(Quad::new(unit_rect(), red()));
        assert_eq!(scene.quad_count(), 3);
    }
}
