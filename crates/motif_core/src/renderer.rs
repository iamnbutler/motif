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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DevicePoint, DeviceRect, DeviceSize, Quad};
    use palette::Srgba;

    fn make_quad() -> Quad {
        let bounds = DeviceRect::new(DevicePoint::new(0.0, 0.0), DeviceSize::new(10.0, 10.0));
        Quad::new(bounds, Srgba::new(1.0, 0.0, 0.0, 1.0))
    }

    #[test]
    fn debug_renderer_initial_state() {
        let r = DebugRenderer::default();
        assert_eq!(r.frames_rendered, 0);
        assert_eq!(r.last_quad_count, 0);
    }

    #[test]
    fn debug_renderer_increments_frame_count() {
        let mut r = DebugRenderer::default();
        let scene = Scene::new();
        r.render(&scene, &mut ());
        assert_eq!(r.frames_rendered, 1);
    }

    #[test]
    fn debug_renderer_records_quad_count() {
        let mut r = DebugRenderer::default();
        let mut scene = Scene::new();
        scene.push_quad(make_quad());
        scene.push_quad(make_quad());
        scene.push_quad(make_quad());
        r.render(&scene, &mut ());
        assert_eq!(r.last_quad_count, 3);
    }

    #[test]
    fn debug_renderer_accumulates_frames_across_renders() {
        let mut r = DebugRenderer::default();
        let scene = Scene::new();
        r.render(&scene, &mut ());
        r.render(&scene, &mut ());
        r.render(&scene, &mut ());
        assert_eq!(r.frames_rendered, 3);
    }

    #[test]
    fn debug_renderer_updates_quad_count_each_frame() {
        let mut r = DebugRenderer::default();

        let mut scene = Scene::new();
        scene.push_quad(make_quad());
        r.render(&scene, &mut ());
        assert_eq!(r.last_quad_count, 1);

        scene.clear();
        scene.push_quad(make_quad());
        scene.push_quad(make_quad());
        r.render(&scene, &mut ());
        assert_eq!(r.last_quad_count, 2);
        assert_eq!(r.frames_rendered, 2);
    }

    #[test]
    fn debug_renderer_zero_quads_after_clear() {
        let mut r = DebugRenderer::default();
        let mut scene = Scene::new();
        scene.push_quad(make_quad());
        r.render(&scene, &mut ());
        assert_eq!(r.last_quad_count, 1);

        scene.clear();
        r.render(&scene, &mut ());
        assert_eq!(r.last_quad_count, 0);
    }
}
