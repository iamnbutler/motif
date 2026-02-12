# Metal Renderer Design

GPU backend using Metal for rendering Scene to window on macOS.

## Goals

- Implement `Renderer` trait with Metal backend
- Instanced rendering of quads (unit quad + instance buffer)
- Solid color fills (borders/corners later)
- Integration with winit window

## Non-Goals

- Borders, rounded corners (future shader work)
- Texture support
- Other primitive types (paths, sprites)
- Cross-platform (Metal is macOS/iOS only)

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│ MetalRenderer                                            │
├─────────────────────────────────────────────────────────┤
│ device: MTLDevice                                        │
│ command_queue: MTLCommandQueue                           │
│ pipeline: MTLRenderPipelineState                         │
│ unit_quad_buffer: MTLBuffer      (4 vertices, static)    │
│ instance_buffer: MTLBuffer       (dynamic, per-frame)    │
└─────────────────────────────────────────────────────────┘
         │
         │ render(&scene, &surface)
         ▼
┌─────────────────────────────────────────────────────────┐
│ Per frame:                                               │
│ 1. Get drawable from CAMetalLayer                        │
│ 2. Convert Scene quads → instance data                   │
│ 3. Upload instance buffer                                │
│ 4. Encode instanced draw call                            │
│ 5. Commit & present                                      │
└─────────────────────────────────────────────────────────┘
```

## Data Structures

### QuadInstance (GPU)

```rust
#[repr(C)]
struct QuadInstance {
    bounds: [f32; 4],     // x, y, width, height (device pixels)
    color: [f32; 4],      // r, g, b, a
}
```

32 bytes per quad, tightly packed for GPU.

### Unit Quad Vertices

```
[0,0], [1,0], [0,1], [1,1]  // triangle strip, 4 vertices
```

Static buffer, shared by all instances.

## Shader

Metal Shading Language (MSL):

```metal
struct QuadInstance {
    float4 bounds;    // x, y, width, height
    float4 color;     // r, g, b, a
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

    // Device pixels → clip space
    float2 clip = (pos / viewport_size) * 2.0 - 1.0;
    clip.y = -clip.y;

    VertexOut out;
    out.position = float4(clip, 0.0, 1.0);
    out.color = inst.color;
    return out;
}

fragment float4 fragment_main(VertexOut in [[stage_in]]) {
    return in.color;
}
```

## Components

### MetalRenderer

```rust
pub struct MetalRenderer {
    device: Device,
    command_queue: CommandQueue,
    pipeline: RenderPipelineState,
    unit_quad_buffer: Buffer,
    instance_buffer: Buffer,
    instance_capacity: usize,
}

impl Renderer for MetalRenderer {
    type Surface = MetalSurface;

    fn render(&mut self, scene: &Scene, surface: &mut MetalSurface);
}
```

### MetalSurface

```rust
pub struct MetalSurface {
    layer: MetalLayer,
    drawable_size: (f32, f32),
}
```

Wraps CAMetalLayer attached to window.

## Dependencies

```toml
metal = "0.29"
objc = "0.2"
core-graphics-types = "0.1"
foreign-types = "0.5"
```

## Render Flow

1. **Init** (once):
   - Get system Metal device
   - Create command queue
   - Compile shader, create pipeline
   - Create unit quad buffer (4 vertices)
   - Create instance buffer (initial capacity ~1000)

2. **Per frame**:
   - Grow instance buffer if needed
   - Copy Scene quads to instance buffer
   - Get next drawable from layer
   - Create command buffer
   - Create render command encoder
   - Set pipeline, vertex buffer, instance buffer, viewport uniform
   - Draw triangle strip, 4 vertices, N instances
   - End encoding, present drawable, commit

## Future Additions

- SDF-based rounded corners and borders in fragment shader
- Texture sampling for sprites/text
- Blend modes
- Clip rect via scissor test
