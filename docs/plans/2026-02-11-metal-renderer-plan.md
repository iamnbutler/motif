# Metal Renderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a Metal GPU renderer that displays quads on screen using instanced rendering.

**Architecture:** MetalRenderer implements the Renderer trait, using a unit quad with instance buffer for efficient batched rendering. MetalSurface wraps CAMetalLayer attached to a winit window.

**Tech Stack:** metal-rs, objc, core-graphics-types, foreign-types, winit

---

## Task 1: Add Metal Dependencies

**Files:**
- Modify: `crates/gesso_core/Cargo.toml`

**Step 1: Add dependencies**

Add to `[target.'cfg(target_os = "macos")'.dependencies]`:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.29"
objc = "0.2"
core-graphics-types = "0.1"
foreign-types = "0.5"
```

**Step 2: Verify compilation**

Run: `cargo check -p gesso_core`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/gesso_core/Cargo.toml
git commit -m "deps: add metal dependencies for macos"
```

---

## Task 2: Create QuadInstance GPU Struct

**Files:**
- Create: `crates/gesso_core/src/metal/mod.rs`
- Modify: `crates/gesso_core/src/lib.rs`

**Step 1: Create metal module with QuadInstance**

```rust
//! Metal renderer implementation (macOS only).

use crate::{DeviceRect, Quad};

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
```

**Step 2: Add conditional module to lib.rs**

Add to `crates/gesso_core/src/lib.rs`:

```rust
#[cfg(target_os = "macos")]
pub mod metal;
```

**Step 3: Verify compilation**

Run: `cargo check -p gesso_core`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add crates/gesso_core/src/metal/mod.rs crates/gesso_core/src/lib.rs
git commit -m "feat(metal): add QuadInstance GPU struct"
```

---

## Task 3: Create Metal Shader

**Files:**
- Create: `crates/gesso_core/src/metal/shaders.metal`

**Step 1: Write the shader**

```metal
#include <metal_stdlib>
using namespace metal;

struct QuadInstance {
    float4 bounds;    // x, y, width, height
    float4 color;     // r, g, b, a
};

struct VertexOut {
    float4 position [[position]];
    float4 color;
};

vertex VertexOut vertex_main(
    uint vertex_id [[vertex_id]],
    uint instance_id [[instance_id]],
    constant float2 *vertices [[buffer(0)]],
    constant QuadInstance *instances [[buffer(1)]],
    constant float2 &viewport_size [[buffer(2)]]
) {
    float2 unit_pos = vertices[vertex_id];
    QuadInstance inst = instances[instance_id];

    // Scale unit quad to instance bounds
    float2 pos = inst.bounds.xy + unit_pos * inst.bounds.zw;

    // Device pixels → clip space [-1, 1]
    float2 clip = (pos / viewport_size) * 2.0 - 1.0;
    clip.y = -clip.y;  // Flip Y for Metal's coordinate system

    VertexOut out;
    out.position = float4(clip, 0.0, 1.0);
    out.color = inst.color;
    return out;
}

fragment float4 fragment_main(VertexOut in [[stage_in]]) {
    return in.color;
}
```

**Step 2: Commit**

```bash
git add crates/gesso_core/src/metal/shaders.metal
git commit -m "feat(metal): add vertex/fragment shaders"
```

---

## Task 4: Embed Shader as Compile-Time String

**Files:**
- Modify: `crates/gesso_core/src/metal/mod.rs`

**Step 1: Add shader source constant**

Add at top of metal/mod.rs:

```rust
/// Metal shader source, compiled at runtime.
const SHADER_SOURCE: &str = include_str!("shaders.metal");
```

**Step 2: Verify compilation**

Run: `cargo check -p gesso_core`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/gesso_core/src/metal/mod.rs
git commit -m "feat(metal): embed shader source"
```

---

## Task 5: Create MetalSurface

**Files:**
- Modify: `crates/gesso_core/src/metal/mod.rs`

**Step 1: Add imports**

```rust
use core_graphics_types::geometry::CGSize;
use foreign_types::ForeignType;
use metal::{CAMetalLayer, Device, MetalLayer};
use objc::runtime::{Object, YES};
use std::mem;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
```

**Step 2: Add MetalSurface struct**

```rust
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
```

**Step 3: Add CGRect definition and objc import**

Add near the top:

```rust
use objc::msg_send;

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
```

**Step 4: Verify compilation**

Run: `cargo check -p gesso_core`
Expected: Compiles (may have warnings about unused items, that's fine)

**Step 5: Commit**

```bash
git add crates/gesso_core/src/metal/mod.rs
git commit -m "feat(metal): add MetalSurface"
```

---

## Task 6: Create MetalRenderer Struct

**Files:**
- Modify: `crates/gesso_core/src/metal/mod.rs`

**Step 1: Add MetalRenderer struct and initialization**

```rust
use metal::{
    Buffer, CommandQueue, CompileOptions, Device, Library, MTLResourceOptions,
    RenderPipelineDescriptor, RenderPipelineState,
};

/// Unit quad vertices for triangle strip: [0,0], [1,0], [0,1], [1,1]
const UNIT_QUAD_VERTICES: [[f32; 2]; 4] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [0.0, 1.0],
    [1.0, 1.0],
];

const INITIAL_INSTANCE_CAPACITY: usize = 1024;

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
```

**Step 2: Verify compilation**

Run: `cargo check -p gesso_core`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/gesso_core/src/metal/mod.rs
git commit -m "feat(metal): add MetalRenderer struct with initialization"
```

---

## Task 7: Implement Renderer Trait for MetalRenderer

**Files:**
- Modify: `crates/gesso_core/src/metal/mod.rs`

**Step 1: Add Renderer implementation**

```rust
use crate::{Renderer, Scene};

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
```

**Step 2: Verify compilation**

Run: `cargo check -p gesso_core`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/gesso_core/src/metal/mod.rs
git commit -m "feat(metal): implement Renderer trait"
```

---

## Task 8: Update hello_window Example to Render a Quad

**Files:**
- Modify: `crates/gesso/Cargo.toml`
- Modify: `crates/gesso/examples/hello_window.rs`

**Step 1: Add gesso_core dependency to gesso crate**

The gesso crate already has gesso_core, but ensure examples can access it.

**Step 2: Rewrite hello_window.rs**

```rust
//! Opens a window and renders a red quad using Metal.

use gesso_core::{
    metal::{MetalRenderer, MetalSurface},
    DeviceRect, Quad, Renderer, Scene,
};
use glamour::{Point2, Size2};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Gesso - Hello Quad")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            let window = event_loop.create_window(attrs).unwrap();

            let renderer = MetalRenderer::new();
            let surface = unsafe { MetalSurface::new(&window, renderer.device()) };

            self.window = Some(window);
            self.renderer = Some(renderer);
            self.surface = Some(surface);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    if let Some(window) = &self.window {
                        let scale = window.scale_factor() as f32;
                        surface.resize(size.width as f32 * scale, size.height as f32 * scale);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(surface)) =
                    (&mut self.renderer, &mut self.surface)
                {
                    // Build scene: red quad centered in window
                    self.scene.clear();

                    let (width, height) = surface.drawable_size();
                    let quad_size = 200.0;
                    let quad = Quad::new(
                        DeviceRect::new(
                            Point2::new((width - quad_size) / 2.0, (height - quad_size) / 2.0),
                            Size2::new(quad_size, quad_size),
                        ),
                        palette::Srgba::new(1.0, 0.0, 0.0, 1.0), // Red
                    );
                    self.scene.push_quad(quad);

                    renderer.render(&self.scene, surface);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
```

**Step 3: Add glamour dependency to gesso crate**

Add to `crates/gesso/Cargo.toml`:

```toml
glamour = "0.18"
```

**Step 4: Verify compilation**

Run: `cargo build --example hello_window`
Expected: Compiles without errors

**Step 5: Run the example**

Run: `cargo run --example hello_window`
Expected: Window opens with black background and red square centered

**Step 6: Commit**

```bash
git add crates/gesso/Cargo.toml crates/gesso/examples/hello_window.rs
git commit -m "feat: hello_window renders red quad with Metal"
```

---

## Task 9: Update llms.txt

**Files:**
- Modify: `llms.txt`

**Step 1: Add Metal renderer documentation**

Add to Key Files section:

```markdown
- [metal/mod.rs](crates/gesso_core/src/metal/mod.rs) - Metal renderer (macOS): MetalRenderer, MetalSurface, QuadInstance
- [metal/shaders.metal](crates/gesso_core/src/metal/shaders.metal) - MSL shaders for instanced quad rendering
```

Update Documentation section to include Metal design doc.

**Step 2: Commit**

```bash
git add llms.txt
git commit -m "docs: add metal renderer to llms.txt"
```

---

## Summary

After completing all tasks:
- MetalRenderer implements Renderer trait with instanced quad rendering
- MetalSurface wraps CAMetalLayer for window integration
- hello_window example displays a red quad
- Foundation ready for borders, rounded corners, and more primitives
