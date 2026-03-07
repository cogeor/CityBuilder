// Terrain tile shader — flat-colored isometric diamonds.

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct InstanceInput {
    @location(1) screen_pos: vec2<f32>,
    @location(2) z_order: f32,
    @location(3) color_id: u32,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) @interpolate(flat) color_id: u32,
};

struct Uniforms {
    projection: mat4x4<f32>,
    camera_offset: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

const TILE_W: f32 = 64.0;
const TILE_H: f32 = 32.0;

fn tile_color(id: u32) -> vec4<f32> {
    switch id {
        case 0u:  { return vec4<f32>(0.35, 0.65, 0.25, 1.0); } // Grass
        case 1u:  { return vec4<f32>(0.15, 0.40, 0.70, 1.0); } // Water
        case 2u:  { return vec4<f32>(0.85, 0.75, 0.50, 1.0); } // Sand
        case 3u:  { return vec4<f32>(0.15, 0.45, 0.15, 1.0); } // Forest
        case 4u:  { return vec4<f32>(0.50, 0.45, 0.40, 1.0); } // Rock
        case 7u:  { return vec4<f32>(0.40, 0.40, 0.42, 1.0); } // Road
        case 11u: { return vec4<f32>(0.45, 0.72, 0.40, 1.0); } // Res zone
        case 12u: { return vec4<f32>(0.40, 0.55, 0.78, 1.0); } // Com zone
        case 13u: { return vec4<f32>(0.75, 0.68, 0.35, 1.0); } // Ind zone
        case 14u: { return vec4<f32>(0.60, 0.45, 0.70, 1.0); } // Civic zone
        case 15u: { return vec4<f32>(0.35, 0.75, 0.35, 1.0); } // Park
        case 16u: { return vec4<f32>(0.55, 0.55, 0.55, 1.0); } // Transport
        case 21u: { return vec4<f32>(0.30, 0.55, 0.28, 1.0); } // Res building
        case 22u: { return vec4<f32>(0.28, 0.40, 0.65, 1.0); } // Com building
        case 23u: { return vec4<f32>(0.60, 0.52, 0.22, 1.0); } // Ind building
        case 24u: { return vec4<f32>(0.45, 0.32, 0.58, 1.0); } // Civic building
        default:  { return vec4<f32>(0.35, 0.65, 0.25, 1.0); } // Grass
    }
}

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = vec2<f32>(
        inst.screen_pos.x + vert.position.x * TILE_W - u.camera_offset.x,
        inst.screen_pos.y + vert.position.y * TILE_H - u.camera_offset.y,
    );
    out.clip_pos = u.projection * vec4<f32>(world_pos, inst.z_order, 1.0);
    out.color_id = inst.color_id;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return tile_color(in.color_id);
}
