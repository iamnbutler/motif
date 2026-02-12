//! Metal renderer implementation (macOS only).

/// Metal shader source, compiled at runtime.
const SHADER_SOURCE: &str = include_str!("shaders.metal");

use crate::Quad;
use core_graphics_types::geometry::CGSize;
use foreign_types::ForeignType;
use metal::{Device, MetalLayer};
use objc::{msg_send, sel, sel_impl, runtime::{Object, YES}};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[repr(C)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}

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

/// Wraps CAMetalLayer attached to a window.
pub struct MetalSurface {
    layer: MetalLayer,
    drawable_size: (f32, f32),
}

impl MetalSurface {
    /// Create a Metal surface for the given window.
    ///
    /// # Safety
    /// Window must remain valid for the lifetime of this surface.
    pub unsafe fn new(window: &impl HasWindowHandle, device: &Device) -> Self {
        let handle = window.window_handle().unwrap();
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            panic!("Expected AppKit window handle on macOS");
        };

        let ns_view = handle.ns_view.as_ptr() as *mut Object;

        let layer = MetalLayer::new();
        layer.set_device(device);
        layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);

        // Set layer on view
        let _: () = msg_send![ns_view, setWantsLayer: YES];
        let _: () = msg_send![ns_view, setLayer: layer.as_ptr()];

        // Get initial size
        let bounds: CGRect = msg_send![ns_view, bounds];
        let scale: f64 = msg_send![ns_view, backingScaleFactor];
        let drawable_size = (
            (bounds.size.width * scale) as f32,
            (bounds.size.height * scale) as f32,
        );
        layer.set_drawable_size(CGSize::new(drawable_size.0 as f64, drawable_size.1 as f64));

        Self {
            layer,
            drawable_size,
        }
    }

    /// Update drawable size (call on window resize).
    pub fn resize(&mut self, width: f32, height: f32) {
        self.drawable_size = (width, height);
        self.layer.set_drawable_size(CGSize::new(width as f64, height as f64));
    }

    pub fn drawable_size(&self) -> (f32, f32) {
        self.drawable_size
    }

    pub fn layer(&self) -> &MetalLayer {
        &self.layer
    }
}
