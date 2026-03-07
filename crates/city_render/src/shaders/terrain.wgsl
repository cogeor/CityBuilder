// Isometric terrain tile shader — instanced pattern rendering.
//
// Each instance is a terrain tile rendered as a flat isometric diamond.
// Patterns (stripes/solid) are defined in a uniform buffer array.

struct VertexInput {
    @location(0) position: vec2<f32>,   // unit quad vertex [-0.5, 0.5]
};

struct InstanceInput {
    @location(1) screen_pos: vec2<f32>, // screen position (pixels)
    @location(2) z_order: f32,          // depth sort key (0..1)
    @location(3) pattern_id: u32,       // index into pattern array
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local_uv: vec2<f32>,     // local position within tile [-0.5, 0.5]
    @location(1) @interpolate(flat) pattern_id: u32,
};

struct Uniforms {
    projection: mat4x4<f32>,  // ortho projection
    camera_offset: vec2<f32>, // camera pan offset in pixels
    _padding: vec2<f32>,
};

// Pattern definition: base_color, stripe_color, stripe_params
// stripe_params.x = angle (radians), .y = width (px), .z = spacing (px)
struct Pattern {
    base_color: vec4<f32>,
    stripe_color: vec4<f32>,
    stripe_params: vec4<f32>,
};

const MAX_PATTERNS: u32 = 32u;

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<uniform> patterns: array<Pattern, 32>;

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
    out.local_uv = vert.position; // pass through [-0.5, 0.5]
    out.pattern_id = inst.pattern_id;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pid = min(in.pattern_id, MAX_PATTERNS - 1u);
    let pat = patterns[pid];

    let base = pat.base_color;
    let stripe_col = pat.stripe_color;
    let angle = pat.stripe_params.x;
    let width = pat.stripe_params.y;
    let spacing = pat.stripe_params.z;

    // No stripes if stripe alpha is 0 or spacing is 0
    if stripe_col.a <= 0.0 || spacing <= 0.0 {
        return base;
    }

    // Compute stripe pattern in pixel space
    // UV is [-0.5, 0.5], scale to tile pixel dimensions
    let px = in.local_uv.x * TILE_W;
    let py = in.local_uv.y * TILE_H;

    // Rotate coordinates by stripe angle
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    let rotated = px * cos_a + py * sin_a;

    // Stripe: modulo spacing, check if within stripe width
    let period = width + spacing;
    let pos_in_period = rotated - floor(rotated / period) * period;
    let in_stripe = step(0.0, pos_in_period) * step(pos_in_period, width);

    // Blend: base where no stripe, stripe color where stripe
    let color = mix(base, vec4<f32>(stripe_col.rgb, base.a), in_stripe * stripe_col.a);
    return color;
}
