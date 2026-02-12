//! Metal renderer implementation (macOS only).

/// Metal shader source, compiled at runtime.
const SHADER_SOURCE: &str = include_str!("shaders.metal");

use crate::{Quad, Renderer, Scene};
use core_graphics_types::geometry::CGSize;
use foreign_types::ForeignType;
use metal::{
    Buffer, CommandQueue, CompileOptions, Device, MTLResourceOptions,
    MetalLayer, RenderPipelineDescriptor, RenderPipelineState,
};
use std::mem;
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

/// Unit quad vertices for triangle strip: [0,0], [1,0], [0,1], [1,1]
const UNIT_QUAD_VERTICES: [[f32; 2]; 4] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [0.0, 1.0],
    [1.0, 1.0],
];

const INITIAL_INSTANCE_CAPACITY: usize = 1024;

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

pub struct MetalRenderer {
    device: Device,
    command_queue: CommandQueue,
    pipeline: RenderPipelineState,
    unit_quad_buffer: Buffer,
    instance_buffer: Buffer,
    instance_capacity: usize,
}

impl MetalRenderer {
    pub fn new() -> Self {
        let device = Device::system_default().expect("No Metal device found");
        let command_queue = device.new_command_queue();

        // Compile shader
        let library = device
            .new_library_with_source(SHADER_SOURCE, &CompileOptions::new())
            .expect("Failed to compile shader");

        let vertex_fn = library.get_function("vertex_main", None).unwrap();
        let fragment_fn = library.get_function("fragment_main", None).unwrap();

        // Create pipeline
        let pipeline_desc = RenderPipelineDescriptor::new();
        pipeline_desc.set_vertex_function(Some(&vertex_fn));
        pipeline_desc.set_fragment_function(Some(&fragment_fn));
        pipeline_desc
            .color_attachments()
            .object_at(0)
            .unwrap()
            .set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);

        let pipeline = device
            .new_render_pipeline_state(&pipeline_desc)
            .expect("Failed to create pipeline");

        // Create unit quad buffer
        let unit_quad_buffer = device.new_buffer_with_data(
            UNIT_QUAD_VERTICES.as_ptr() as *const _,
            (UNIT_QUAD_VERTICES.len() * mem::size_of::<[f32; 2]>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        // Create instance buffer
        let instance_buffer = device.new_buffer(
            (INITIAL_INSTANCE_CAPACITY * mem::size_of::<QuadInstance>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        Self {
            device,
            command_queue,
            pipeline,
            unit_quad_buffer,
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
        }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }
}

impl Default for MetalRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for MetalRenderer {
    type Surface = MetalSurface;

    fn render(&mut self, scene: &Scene, surface: &mut MetalSurface) {
        let quads = scene.quads();
        if quads.is_empty() {
            return;
        }

        // Grow instance buffer if needed
        if quads.len() > self.instance_capacity {
            self.instance_capacity = quads.len().next_power_of_two();
            self.instance_buffer = self.device.new_buffer(
                (self.instance_capacity * mem::size_of::<QuadInstance>()) as u64,
                MTLResourceOptions::StorageModeShared,
            );
        }

        // Copy quad data to instance buffer
        let instances: Vec<QuadInstance> = quads.iter().map(QuadInstance::from_quad).collect();
        unsafe {
            std::ptr::copy_nonoverlapping(
                instances.as_ptr(),
                self.instance_buffer.contents() as *mut QuadInstance,
                instances.len(),
            );
        }

        // Get drawable
        let drawable = match surface.layer().next_drawable() {
            Some(d) => d,
            None => return,
        };

        // Create command buffer and encoder
        let command_buffer = self.command_queue.new_command_buffer();

        let render_pass_desc = metal::RenderPassDescriptor::new();
        let color_attachment = render_pass_desc.color_attachments().object_at(0).unwrap();
        color_attachment.set_texture(Some(drawable.texture()));
        color_attachment.set_load_action(metal::MTLLoadAction::Clear);
        color_attachment.set_clear_color(metal::MTLClearColor::new(0.0, 0.0, 0.0, 1.0));
        color_attachment.set_store_action(metal::MTLStoreAction::Store);

        let encoder = command_buffer.new_render_command_encoder(render_pass_desc);

        encoder.set_render_pipeline_state(&self.pipeline);
        encoder.set_vertex_buffer(0, Some(&self.unit_quad_buffer), 0);
        encoder.set_vertex_buffer(1, Some(&self.instance_buffer), 0);

        // Pass viewport size as uniform
        let viewport_size: [f32; 2] = [surface.drawable_size().0, surface.drawable_size().1];
        encoder.set_vertex_bytes(
            2,
            mem::size_of::<[f32; 2]>() as u64,
            viewport_size.as_ptr() as *const _,
        );

        // Draw instanced triangle strip
        encoder.draw_primitives_instanced(
            metal::MTLPrimitiveType::TriangleStrip,
            0,
            4,
            quads.len() as u64,
        );

        encoder.end_encoding();

        command_buffer.present_drawable(drawable);
        command_buffer.commit();
    }
}
