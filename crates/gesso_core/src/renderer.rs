//! Renderer trait for backend abstraction.

use crate::Scene;

/// Backend-agnostic renderer.
pub trait Renderer {
    /// Surface type for this renderer (e.g., MetalTexture, WgpuSurface).
    type Surface;

    /// Render the scene to the surface.
    fn render(&mut self, scene: &Scene, surface: &mut Self::Surface);
}

/// Debug renderer that counts primitives without GPU.
#[derive(Default)]
pub struct DebugRenderer {
    pub frames_rendered: usize,
    pub last_quad_count: usize,
}

impl Renderer for DebugRenderer {
    type Surface = ();

    fn render(&mut self, scene: &Scene, _surface: &mut Self::Surface) {
        self.frames_rendered += 1;
        self.last_quad_count = scene.quad_count();
    }
}
