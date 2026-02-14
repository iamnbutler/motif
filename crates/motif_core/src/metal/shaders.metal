#include <metal_stdlib>
using namespace metal;

struct QuadInstance {
    float4 bounds;        // x, y, width, height
    float4 color;         // r, g, b, a (background)
    float4 border_color;  // r, g, b, a
    float4 border_widths; // top, right, bottom, left
    float4 corner_radii;  // top_left, top_right, bottom_right, bottom_left
    float4 clip_bounds;   // x, y, width, height of clip region
    float has_clip;       // 1.0 if clip active
    float3 _padding;
};

struct VertexOut {
    float4 position [[position]];
    float4 color;
    float4 border_color;
    float4 border_widths;
    float4 corner_radii;
    float4 clip_bounds;   // in device pixels
    float has_clip;
    float2 quad_size;     // width, height in pixels
    float2 local_pos;     // position within quad in pixels
    float2 device_pos;    // absolute position in device pixels
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
    out.border_color = inst.border_color;
    out.border_widths = inst.border_widths;
    out.corner_radii = inst.corner_radii;
    out.clip_bounds = inst.clip_bounds;
    out.has_clip = inst.has_clip;
    out.quad_size = inst.bounds.zw;
    out.local_pos = unit_pos * inst.bounds.zw;
    out.device_pos = pos; // Absolute position in device pixels
    return out;
}

// SDF for rounded rectangle - returns negative inside, positive outside
float rounded_rect_sdf(float2 pos, float2 size, float4 radii) {
    // radii: top_left, top_right, bottom_right, bottom_left
    // Select corner radius based on quadrant
    float2 center = size * 0.5;
    float r;
    if (pos.x < center.x) {
        r = (pos.y < center.y) ? radii.x : radii.w; // top_left or bottom_left
    } else {
        r = (pos.y < center.y) ? radii.y : radii.z; // top_right or bottom_right
    }

    // Clamp radius to half the smaller dimension
    r = min(r, min(size.x, size.y) * 0.5);

    // Transform to corner-relative coordinates
    float2 q = abs(pos - center) - (center - r);

    // SDF: distance to rounded corner
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

fragment float4 fragment_main(VertexOut in [[stage_in]]) {
    // Apply clip bounds first
    if (in.has_clip > 0.5) {
        float2 clip_min = in.clip_bounds.xy;
        float2 clip_max = clip_min + in.clip_bounds.zw;
        if (in.device_pos.x < clip_min.x || in.device_pos.x > clip_max.x ||
            in.device_pos.y < clip_min.y || in.device_pos.y > clip_max.y) {
            discard_fragment();
        }
    }

    float2 pos = in.local_pos;
    float2 size = in.quad_size;

    // Compute SDF distance (negative = inside)
    float dist = rounded_rect_sdf(pos, size, in.corner_radii);

    // Outside the rounded rect - discard
    if (dist > 0.0) {
        discard_fragment();
    }

    // Check border: use max border width for simplicity
    // A proper implementation would use per-edge border widths
    float max_border = max(max(in.border_widths.x, in.border_widths.y),
                           max(in.border_widths.z, in.border_widths.w));

    // In border region if close to edge
    if (dist > -max_border && in.border_color.a > 0.0) {
        return in.border_color;
    }

    return in.color;
}
