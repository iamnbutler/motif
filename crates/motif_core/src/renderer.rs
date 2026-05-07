//! Renderer trait for backend abstraction.
//!
//! The `Renderer` trait defines the interface that all rendering backends must
//! implement. Backends (Metal, wgpu, software, debug) provide their own
//! concrete `Surface` type, which holds GPU-side state such as swap chains,
//! textures, or framebuffers.
//!
//! ## Implementing a backend
//!
//! ```rust,ignore
//! use motif_core::{Renderer, Scene};
//!
//! struct MyRenderer { /* GPU state */ }
//! struct MySurface { width: f32, height: f32 }
//!
//! impl Renderer for MyRenderer {
//!     type Surface = MySurface;
//!
//!     fn render(&mut self, scene: &Scene, surface: &mut MySurface) {
//!         // upload quads, draw text, present
//!     }
//!
//!     fn resize_surface(&self, surface: &mut MySurface, width: f32, height: f32) {
//!         surface.width = width;
//!         surface.height = height;
//!         // reconfigure swap chain, etc.
//!     }
//! }
//! ```
//!
//! ## Lifecycle
//!
//! 1. Create the renderer once on startup.
//! 2. Create a surface attached to the window (backend-specific).
//! 3. On every `WindowEvent::Resized`, call [`Renderer::resize_surface`].
//! 4. On every `WindowEvent::RedrawRequested`, call [`Renderer::render`].

use crate::Scene;

/// Backend-agnostic renderer.
///
/// Implementors provide a concrete [`Surface`](Self::Surface) type that holds
/// GPU-side resources (textures, swap chains, pipelines). The renderer itself
/// holds device-level state (queues, pipelines) that is independent of any
/// particular window.
pub trait Renderer {
    /// The surface type for this renderer.
    ///
    /// Examples: `MetalSurface`, `WgpuSurface`, `SoftwareSurface`.
    type Surface;

    /// Render the scene to the surface.
    ///
    /// Called once per frame on `WindowEvent::RedrawRequested`. Implementations
    /// should upload scene primitives to GPU buffers and issue draw calls.
    fn render(&mut self, scene: &Scene, surface: &mut Self::Surface);

    /// Update the surface dimensions after a window resize.
    ///
    /// Called on `WindowEvent::Resized`. The default implementation is a no-op,
    /// suitable for backends that derive size from the surface automatically.
    /// Backends that manage explicit swap chains or framebuffers should override
    /// this to reallocate resources.
    ///
    /// `width` and `height` are in physical (device) pixels.
    fn resize_surface(&self, surface: &mut Self::Surface, width: f32, height: f32) {
        // Default: no-op. Backends that need to handle resize explicitly
        // (e.g. wgpu swap chain reconfiguration) should override this.
        let _ = (surface, width, height);
    }
}

/// Debug renderer that counts primitives without GPU.
///
/// Useful in tests and on platforms that lack a real GPU backend.
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

    fn resize_surface(&self, _surface: &mut Self::Surface, _width: f32, _height: f32) {
        // No GPU resources to resize.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DevicePoint, DeviceRect, DeviceSize, Quad, Srgba};

    #[test]
    fn debug_renderer_starts_zeroed() {
        let r = DebugRenderer::default();
        assert_eq!(r.frames_rendered, 0);
        assert_eq!(r.last_quad_count, 0);
    }

    #[test]
    fn debug_renderer_counts_frames() {
        let mut r = DebugRenderer::default();
        let scene = Scene::new();
        r.render(&scene, &mut ());
        assert_eq!(r.frames_rendered, 1);
        r.render(&scene, &mut ());
        assert_eq!(r.frames_rendered, 2);
    }

    #[test]
    fn debug_renderer_counts_quads() {
        let mut r = DebugRenderer::default();
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(
            DeviceRect::new(DevicePoint::new(0.0, 0.0), DeviceSize::new(100.0, 100.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        ));
        r.render(&scene, &mut ());
        assert_eq!(r.last_quad_count, 1);
    }

    #[test]
    fn debug_renderer_resize_is_noop() {
        let r = DebugRenderer::default();
        let mut surface = ();
        // Should not panic or error.
        r.resize_surface(&mut surface, 1920.0, 1080.0);
    }

    #[test]
    fn resize_surface_default_impl_compiles_and_is_noop() {
        // A minimal renderer that does NOT override resize_surface gets the
        // default no-op impl without compilation errors.
        struct MinimalRenderer;
        impl Renderer for MinimalRenderer {
            type Surface = ();
            fn render(&mut self, _scene: &Scene, _surface: &mut ()) {}
        }
        let r = MinimalRenderer;
        let mut surface = ();
        // Must compile and not panic.
        r.resize_surface(&mut surface, 800.0, 600.0);
    }
}
