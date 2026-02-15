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
    pub fn new(origin: DevicePoint, color: impl Into<Srgba>, font_size: f32, font: FontData) -> Self {
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
