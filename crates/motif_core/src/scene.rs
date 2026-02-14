//! Scene holds primitives for rendering.

use crate::{Corners, DeviceRect, Edges};
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

/// Holds all primitives for a frame, ready for rendering.
#[derive(Default)]
pub struct Scene {
    quads: Vec<Quad>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all primitives, reusing allocations.
    pub fn clear(&mut self) {
        self.quads.clear();
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
}
