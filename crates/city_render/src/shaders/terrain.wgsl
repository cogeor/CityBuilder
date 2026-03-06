// Isometric terrain tile shader — instanced colored diamond rendering.
//
// Each instance is a terrain tile rendered as a flat isometric diamond.
// No texture atlas needed for the first demo — just colored quads.

struct VertexInput {
    @location(0) position: vec2<f32>,   // unit quad vertex [-0.5, 0.5]
};

struct InstanceInput {
    @location(1) screen_pos: vec2<f32>, // screen position (pixels)
    @location(2) color: vec4<f32>,      // RGBA color
    @location(3) z_order: f32,          // depth sort key (0..1)
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct Uniforms {
    projection: mat4x4<f32>,  // ortho projection
    camera_offset: vec2<f32>, // camera pan offset in pixels
    _padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

// Tile dimensions in pixels (isometric diamond)
const TILE_W: f32 = 64.0;
const TILE_H: f32 = 32.0;

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    // Scale unit quad to tile size (diamond shape via vertex positions)
    let world_pos = vec2<f32>(
        inst.screen_pos.x + vert.position.x * TILE_W - u.camera_offset.x,
        inst.screen_pos.y + vert.position.y * TILE_H - u.camera_offset.y,
    );

    out.clip_pos = u.projection * vec4<f32>(world_pos, inst.z_order, 1.0);
    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
