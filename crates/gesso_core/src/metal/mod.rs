//! Metal renderer implementation (macOS only).

use crate::Quad;

/// GPU-side quad instance data.
/// Tightly packed for Metal buffer: 32 bytes per quad.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct QuadInstance {
    /// x, y, width, height in device pixels
    pub bounds: [f32; 4],
    /// r, g, b, a
    pub color: [f32; 4],
}

impl QuadInstance {
    pub fn from_quad(quad: &Quad) -> Self {
        Self {
            bounds: [
                quad.bounds.origin.x,
                quad.bounds.origin.y,
                quad.bounds.size.width,
                quad.bounds.size.height,
            ],
            color: [
                quad.background.red,
                quad.background.green,
                quad.background.blue,
                quad.background.alpha,
            ],
        }
    }
}
