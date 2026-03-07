// Sprite shader — renders textured quads from a sprite atlas.
//
// Each instance is a building sprite positioned at an isometric tile.
// Sprites are taller than tiles (96px vs 32px) and overlap via depth testing.

struct VertexInput {
    @location(0) position: vec2<f32>,   // unit quad [0,1] × [0,1]
};

struct SpriteInstance {
    @location(1) screen_pos: vec2<f32>, // screen position (tile center, pixels)
    @location(2) z_order: f32,          // depth (0..1, lower = further)
    @location(3) uv_rect: vec4<f32>,    // atlas UVs: (u0, v0, u1, v1)
    @location(4) size: vec2<f32>,       // sprite size in world pixels (w, h)
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Uniforms {
    projection: mat4x4<f32>,
    camera_offset: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var sprite_texture: texture_2d<f32>;
@group(0) @binding(2) var sprite_sampler: sampler;

@vertex
fn vs_main(vert: VertexInput, inst: SpriteInstance) -> VertexOutput {
    var out: VertexOutput;

    // Quad goes from (0,0) to (1,1). Position sprite so bottom-center
    // aligns with screen_pos (the tile center).
    let offset = vec2<f32>(
        (vert.position.x - 0.5) * inst.size.x,
        (vert.position.y - 1.0) * inst.size.y, // anchor at bottom
    );

    let world_pos = vec2<f32>(
        inst.screen_pos.x + offset.x - u.camera_offset.x,
        inst.screen_pos.y + offset.y - u.camera_offset.y,
    );

    out.clip_pos = u.projection * vec4<f32>(world_pos, inst.z_order, 1.0);
    // Map quad position to atlas UV rect
    out.uv = vec2<f32>(
        mix(inst.uv_rect.x, inst.uv_rect.z, vert.position.x),
        mix(inst.uv_rect.y, inst.uv_rect.w, vert.position.y),
    );
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(sprite_texture, sprite_sampler, in.uv);
    if color.a < 0.1 {
        discard;
    }
    return color;
}
