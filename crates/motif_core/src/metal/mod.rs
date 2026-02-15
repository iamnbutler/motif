//! Metal renderer implementation (macOS only).

/// Metal shader source, compiled at runtime.
const SHADER_SOURCE: &str = include_str!("shaders.metal");

use crate::{FontData, GlyphCache, Quad, RasterizedGlyph, Renderer, Scene, TextRun};
use core_graphics_types::geometry::CGSize;
use foreign_types::ForeignType;
use metal::{
    Buffer, CommandQueue, CompileOptions, Device, MTLPixelFormat, MTLResourceOptions,
    MTLTextureUsage, MetalLayer, RenderPipelineDescriptor, RenderPipelineState, Texture,
    TextureDescriptor,
};
use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSView;
use std::collections::HashMap;
use std::mem;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Unit quad vertices for triangle strip: [0,0], [1,0], [0,1], [1,1]
const UNIT_QUAD_VERTICES: [[f32; 2]; 4] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [0.0, 1.0],
    [1.0, 1.0],
];

const INITIAL_INSTANCE_CAPACITY: usize = 1024;

/// GPU-side quad instance data.
/// Tightly packed for Metal buffer: 104 bytes per quad.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadInstance {
    /// x, y, width, height in device pixels
    pub bounds: [f32; 4],
    /// r, g, b, a (background)
    pub color: [f32; 4],
    /// r, g, b, a (border)
    pub border_color: [f32; 4],
    /// top, right, bottom, left
    pub border_widths: [f32; 4],
    /// top_left, top_right, bottom_right, bottom_left
    pub corner_radii: [f32; 4],
    /// x, y, width, height of clip region
    pub clip_bounds: [f32; 4],
    /// 1.0 if clip is active, 0.0 otherwise
    pub has_clip: f32,
    /// Padding for alignment (Metal likes 16-byte alignment)
    pub _padding: [f32; 3],
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
            border_color: [
                quad.border_color.red,
                quad.border_color.green,
                quad.border_color.blue,
                quad.border_color.alpha,
            ],
            border_widths: [
                quad.border_widths.top,
                quad.border_widths.right,
                quad.border_widths.bottom,
                quad.border_widths.left,
            ],
            corner_radii: [
                quad.corner_radii.top_left,
                quad.corner_radii.top_right,
                quad.corner_radii.bottom_right,
                quad.corner_radii.bottom_left,
            ],
            clip_bounds: quad.clip_bounds.map_or([0.0, 0.0, 0.0, 0.0], |r| {
                [r.origin.x, r.origin.y, r.size.width, r.size.height]
            }),
            has_clip: if quad.clip_bounds.is_some() { 1.0 } else { 0.0 },
            _padding: [0.0; 3],
        }
    }
}

/// GPU-side glyph instance data for text rendering.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GlyphInstance {
    /// x, y, width, height in device pixels
    pub bounds: [f32; 4],
    /// UV coordinates in atlas: u_min, v_min, u_max, v_max
    pub uv: [f32; 4],
    /// r, g, b, a (text color)
    pub color: [f32; 4],
}

/// A region in the texture atlas for a cached glyph.
#[derive(Clone, Copy, Debug)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Simple row-based texture atlas for glyph caching.
pub struct GlyphAtlas {
    texture: Texture,
    width: u32,
    height: u32,
    /// Current row Y position
    row_y: u32,
    /// Current X position in row
    row_x: u32,
    /// Height of current row (max glyph height in row)
    row_height: u32,
    /// Cached glyph locations: (font_id, glyph_id, size_bits) -> region
    cache: HashMap<(u64, u32, u32), AtlasRegion>,
}

impl GlyphAtlas {
    const ATLAS_SIZE: u32 = 1024;
    const PADDING: u32 = 1;

    pub fn new(device: &Device) -> Self {
        let descriptor = TextureDescriptor::new();
        descriptor.set_width(Self::ATLAS_SIZE as u64);
        descriptor.set_height(Self::ATLAS_SIZE as u64);
        descriptor.set_pixel_format(MTLPixelFormat::R8Unorm);
        descriptor.set_usage(MTLTextureUsage::ShaderRead);

        let texture = device.new_texture(&descriptor);

        Self {
            texture,
            width: Self::ATLAS_SIZE,
            height: Self::ATLAS_SIZE,
            row_y: 0,
            row_x: 0,
            row_height: 0,
            cache: HashMap::new(),
        }
    }

    /// Get or insert a glyph into the atlas.
    /// Returns the atlas region for the glyph.
    pub fn get_or_insert(
        &mut self,
        font: &FontData,
        glyph_id: u32,
        font_size: f32,
        glyph_cache: &mut GlyphCache,
        normalized_coords: &[i16],
    ) -> Option<AtlasRegion> {
        let key = (font.data.id(), glyph_id, font_size.to_bits());

        // Check if already in atlas
        if let Some(&region) = self.cache.get(&key) {
            return Some(region);
        }

        // Rasterize the glyph
        let rasterized = glyph_cache.rasterize(font, normalized_coords, glyph_id, font_size)?;

        if rasterized.width == 0 || rasterized.height == 0 {
            // Empty glyph (e.g., space) - return zero-size region
            let region = AtlasRegion {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            };
            self.cache.insert(key, region);
            return Some(region);
        }

        // Find space in atlas
        let region = self.allocate(rasterized.width, rasterized.height)?;

        // Upload to texture
        self.upload_glyph(&region, rasterized);

        self.cache.insert(key, region);
        Some(region)
    }

    /// Allocate space for a glyph in the atlas.
    fn allocate(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
        let padded_width = width + Self::PADDING;
        let padded_height = height + Self::PADDING;

        // Check if fits in current row
        if self.row_x + padded_width <= self.width {
            let region = AtlasRegion {
                x: self.row_x,
                y: self.row_y,
                width,
                height,
            };
            self.row_x += padded_width;
            self.row_height = self.row_height.max(padded_height);
            return Some(region);
        }

        // Start new row
        self.row_y += self.row_height;
        self.row_x = 0;
        self.row_height = 0;

        // Check if fits in atlas
        if self.row_y + padded_height > self.height {
            // Atlas full - would need to implement atlas growth or eviction
            return None;
        }

        let region = AtlasRegion {
            x: self.row_x,
            y: self.row_y,
            width,
            height,
        };
        self.row_x += padded_width;
        self.row_height = padded_height;
        Some(region)
    }

    /// Upload glyph data to the texture.
    fn upload_glyph(&self, region: &AtlasRegion, glyph: &RasterizedGlyph) {
        let mtl_region = metal::MTLRegion {
            origin: metal::MTLOrigin {
                x: region.x as u64,
                y: region.y as u64,
                z: 0,
            },
            size: metal::MTLSize {
                width: region.width as u64,
                height: region.height as u64,
                depth: 1,
            },
        };

        self.texture.replace_region(
            mtl_region,
            0,
            glyph.data.as_ptr() as *const _,
            region.width as u64, // bytes per row
        );
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Get UV coordinates for a region (0.0 to 1.0 range).
    pub fn uv_for_region(&self, region: &AtlasRegion) -> [f32; 4] {
        let w = self.width as f32;
        let h = self.height as f32;
        [
            region.x as f32 / w,
            region.y as f32 / h,
            (region.x + region.width) as f32 / w,
            (region.y + region.height) as f32 / h,
        ]
    }

    /// Clear the atlas (for when it fills up).
    pub fn clear(&mut self) {
        self.row_y = 0;
        self.row_x = 0;
        self.row_height = 0;
        self.cache.clear();
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

        let ns_view: &NSView = unsafe {
            (handle.ns_view.as_ptr() as *const NSView).as_ref().unwrap()
        };

        let layer = MetalLayer::new();
        layer.set_device(device);
        layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);

        // Set layer on view
        ns_view.setWantsLayer(true);
        let layer_ptr = layer.as_ptr() as *mut AnyObject;
        let _: () = unsafe { msg_send![ns_view, setLayer: layer_ptr] };

        // Get initial size
        let bounds = ns_view.bounds();
        let scale = ns_view.window().map_or(1.0, |w| w.backingScaleFactor());
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

    /// Enable or disable vsync. Disabled is useful for benchmarking.
    pub fn set_vsync(&self, enabled: bool) {
        self.layer.set_display_sync_enabled(enabled);
    }
}

pub struct MetalRenderer {
    device: Device,
    command_queue: CommandQueue,
    // Quad rendering
    quad_pipeline: RenderPipelineState,
    unit_quad_buffer: Buffer,
    instance_buffer: Buffer,
    instance_capacity: usize,
    // Text rendering
    text_pipeline: RenderPipelineState,
    glyph_instance_buffer: Buffer,
    glyph_instance_capacity: usize,
    glyph_atlas: GlyphAtlas,
    glyph_cache: GlyphCache,
}

impl MetalRenderer {
    pub fn new() -> Self {
        let device = Device::system_default().expect("No Metal device found");
        let command_queue = device.new_command_queue();

        // Compile shader
        let library = device
            .new_library_with_source(SHADER_SOURCE, &CompileOptions::new())
            .expect("Failed to compile shader");

        // Quad pipeline
        let vertex_fn = library.get_function("vertex_main", None).unwrap();
        let fragment_fn = library.get_function("fragment_main", None).unwrap();

        let pipeline_desc = RenderPipelineDescriptor::new();
        pipeline_desc.set_vertex_function(Some(&vertex_fn));
        pipeline_desc.set_fragment_function(Some(&fragment_fn));
        let color_attachment = pipeline_desc.color_attachments().object_at(0).unwrap();
        color_attachment.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);

        let quad_pipeline = device
            .new_render_pipeline_state(&pipeline_desc)
            .expect("Failed to create quad pipeline");

        // Text pipeline
        let text_vertex_fn = library.get_function("text_vertex_main", None).unwrap();
        let text_fragment_fn = library.get_function("text_fragment_main", None).unwrap();

        let text_pipeline_desc = RenderPipelineDescriptor::new();
        text_pipeline_desc.set_vertex_function(Some(&text_vertex_fn));
        text_pipeline_desc.set_fragment_function(Some(&text_fragment_fn));
        let text_color_attachment = text_pipeline_desc.color_attachments().object_at(0).unwrap();
        text_color_attachment.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        // Enable alpha blending for text
        text_color_attachment.set_blending_enabled(true);
        text_color_attachment.set_source_rgb_blend_factor(metal::MTLBlendFactor::SourceAlpha);
        text_color_attachment
            .set_destination_rgb_blend_factor(metal::MTLBlendFactor::OneMinusSourceAlpha);
        text_color_attachment.set_source_alpha_blend_factor(metal::MTLBlendFactor::One);
        text_color_attachment
            .set_destination_alpha_blend_factor(metal::MTLBlendFactor::OneMinusSourceAlpha);

        let text_pipeline = device
            .new_render_pipeline_state(&text_pipeline_desc)
            .expect("Failed to create text pipeline");

        // Create unit quad buffer
        let unit_quad_buffer = device.new_buffer_with_data(
            UNIT_QUAD_VERTICES.as_ptr() as *const _,
            (UNIT_QUAD_VERTICES.len() * mem::size_of::<[f32; 2]>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        // Create instance buffers
        let instance_buffer = device.new_buffer(
            (INITIAL_INSTANCE_CAPACITY * mem::size_of::<QuadInstance>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let glyph_instance_buffer = device.new_buffer(
            (INITIAL_INSTANCE_CAPACITY * mem::size_of::<GlyphInstance>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        // Create glyph atlas
        let glyph_atlas = GlyphAtlas::new(&device);
        let glyph_cache = GlyphCache::new();

        Self {
            device,
            command_queue,
            quad_pipeline,
            unit_quad_buffer,
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            text_pipeline,
            glyph_instance_buffer,
            glyph_instance_capacity: INITIAL_INSTANCE_CAPACITY,
            glyph_atlas,
            glyph_cache,
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
        let text_runs = scene.text_runs();

        if quads.is_empty() && text_runs.is_empty() {
            return;
        }

        // Prepare quad instances
        let quad_instances: Vec<QuadInstance> = quads.iter().map(QuadInstance::from_quad).collect();

        // Prepare glyph instances (must be done before command buffer due to &mut self)
        let glyph_instances = self.build_glyph_instances(text_runs);

        // Grow instance buffers if needed
        if quad_instances.len() > self.instance_capacity {
            self.instance_capacity = quad_instances.len().next_power_of_two();
            self.instance_buffer = self.device.new_buffer(
                (self.instance_capacity * mem::size_of::<QuadInstance>()) as u64,
                MTLResourceOptions::StorageModeShared,
            );
        }

        if glyph_instances.len() > self.glyph_instance_capacity {
            self.glyph_instance_capacity = glyph_instances.len().next_power_of_two();
            self.glyph_instance_buffer = self.device.new_buffer(
                (self.glyph_instance_capacity * mem::size_of::<GlyphInstance>()) as u64,
                MTLResourceOptions::StorageModeShared,
            );
        }

        // Copy data to GPU buffers
        if !quad_instances.is_empty() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    quad_instances.as_ptr(),
                    self.instance_buffer.contents() as *mut QuadInstance,
                    quad_instances.len(),
                );
            }
        }

        if !glyph_instances.is_empty() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    glyph_instances.as_ptr(),
                    self.glyph_instance_buffer.contents() as *mut GlyphInstance,
                    glyph_instances.len(),
                );
            }
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

        let viewport_size: [f32; 2] = [surface.drawable_size().0, surface.drawable_size().1];

        // Render quads with instancing
        if !quad_instances.is_empty() {
            encoder.set_render_pipeline_state(&self.quad_pipeline);
            encoder.set_vertex_buffer(0, Some(&self.unit_quad_buffer), 0);
            encoder.set_vertex_buffer(1, Some(&self.instance_buffer), 0);
            encoder.set_vertex_bytes(
                2,
                mem::size_of::<[f32; 2]>() as u64,
                viewport_size.as_ptr() as *const _,
            );

            encoder.draw_primitives_instanced(
                metal::MTLPrimitiveType::TriangleStrip,
                0,
                4,
                quad_instances.len() as u64,
            );
        }

        // Render text
        if !glyph_instances.is_empty() {
            encoder.set_render_pipeline_state(&self.text_pipeline);
            encoder.set_vertex_buffer(0, Some(&self.unit_quad_buffer), 0);
            encoder.set_vertex_buffer(1, Some(&self.glyph_instance_buffer), 0);
            encoder.set_vertex_bytes(
                2,
                mem::size_of::<[f32; 2]>() as u64,
                viewport_size.as_ptr() as *const _,
            );
            encoder.set_fragment_texture(0, Some(self.glyph_atlas.texture()));

            encoder.draw_primitives_instanced(
                metal::MTLPrimitiveType::TriangleStrip,
                0,
                4,
                glyph_instances.len() as u64,
            );
        }

        encoder.end_encoding();

        command_buffer.present_drawable(drawable);
        command_buffer.commit();
    }
}

impl MetalRenderer {
    /// Build glyph instances from text runs, uploading glyphs to atlas as needed.
    fn build_glyph_instances(&mut self, text_runs: &[TextRun]) -> Vec<GlyphInstance> {
        let mut instances = Vec::new();

        for run in text_runs {
            for glyph in &run.glyphs {
                // Get or rasterize glyph and add to atlas
                let region = match self.glyph_atlas.get_or_insert(
                    &run.font,
                    glyph.glyph_id,
                    run.font_size,
                    &mut self.glyph_cache,
                    &run.normalized_coords,
                ) {
                    Some(r) => r,
                    None => continue, // Atlas full or rasterization failed
                };

                // Skip empty glyphs (spaces)
                if region.width == 0 || region.height == 0 {
                    continue;
                }

                // Get glyph metrics from cache for positioning
                let rasterized = match self.glyph_cache.rasterize(
                    &run.font,
                    &run.normalized_coords,
                    glyph.glyph_id,
                    run.font_size,
                ) {
                    Some(r) => r,
                    None => continue,
                };

                // Calculate screen position
                let x = run.origin.x + glyph.x + rasterized.bearing_x as f32;
                let y = run.origin.y + glyph.y - rasterized.bearing_y as f32;

                let uv = self.glyph_atlas.uv_for_region(&region);

                instances.push(GlyphInstance {
                    bounds: [x, y, region.width as f32, region.height as f32],
                    uv,
                    color: [run.color.red, run.color.green, run.color.blue, run.color.alpha],
                });
            }
        }

        instances
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceRect, Edges};
    use glamour::{Point2, Size2};
    use palette::Srgba;

    #[test]
    fn quad_instance_captures_border_data() {
        let mut quad = Quad::new(
            DeviceRect::new(Point2::new(10.0, 20.0), Size2::new(100.0, 50.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );
        quad.border_color = Srgba::new(0.0, 0.0, 1.0, 1.0);
        quad.border_widths = Edges::all(2.0);

        let instance = QuadInstance::from_quad(&quad);

        // Border color should be captured
        assert_eq!(instance.border_color, [0.0, 0.0, 1.0, 1.0]);
        // Border widths should be captured (top, right, bottom, left)
        assert_eq!(instance.border_widths, [2.0, 2.0, 2.0, 2.0]);
    }

    #[test]
    fn quad_instance_captures_corner_radii() {
        use crate::Corners;

        let mut quad = Quad::new(
            DeviceRect::new(Point2::new(10.0, 20.0), Size2::new(100.0, 50.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );
        quad.corner_radii = Corners::all(8.0);

        let instance = QuadInstance::from_quad(&quad);

        // Corner radii should be captured (top_left, top_right, bottom_right, bottom_left)
        assert_eq!(instance.corner_radii, [8.0, 8.0, 8.0, 8.0]);
    }

    #[test]
    fn quad_instance_captures_clip_bounds() {
        let mut quad = Quad::new(
            DeviceRect::new(Point2::new(0.0, 0.0), Size2::new(100.0, 100.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );
        quad.clip_bounds = Some(DeviceRect::new(
            Point2::new(10.0, 20.0),
            Size2::new(50.0, 60.0),
        ));

        let instance = QuadInstance::from_quad(&quad);

        // Clip bounds: x, y, width, height
        assert_eq!(instance.clip_bounds, [10.0, 20.0, 50.0, 60.0]);
        assert_eq!(instance.has_clip, 1.0); // Flag indicating clip is active
    }

    #[test]
    fn quad_instance_no_clip() {
        let quad = Quad::new(
            DeviceRect::new(Point2::new(0.0, 0.0), Size2::new(100.0, 100.0)),
            Srgba::new(1.0, 0.0, 0.0, 1.0),
        );

        let instance = QuadInstance::from_quad(&quad);

        assert_eq!(instance.has_clip, 0.0); // No clip
    }
}
