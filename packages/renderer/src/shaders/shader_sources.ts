/**
 * Shader sources for palette-swap sprite rendering pipeline.
 *
 * Provides GLSL 300 es (WebGL2) and WGSL (WebGPU) vertex/fragment shaders
 * that support instanced sprite drawing with material-mask-based palette
 * recoloring and time-of-day lighting.
 */

// ── GLSL 300 es (WebGL2) ────────────────────────────────────────────────────

/**
 * Vertex shader: transform instances to screen coordinates.
 * Receives per-instance data via attributes.
 */
export const PALETTE_VERTEX_GLSL = `#version 300 es
precision highp float;

// Per-vertex (quad)
layout(location = 0) in vec2 a_quad_pos;   // unit quad [0,1]x[0,1]
layout(location = 1) in vec2 a_quad_uv;    // UV for this quad vertex

// Per-instance
layout(location = 2) in vec2 a_screen_pos;  // screen position
layout(location = 3) in vec2 a_sprite_size; // sprite width/height in pixels
layout(location = 4) in vec4 a_uv_rect;    // UV rect: x, y, w, h
layout(location = 5) in vec4 a_tint;       // RGBA tint color (0-1)
layout(location = 6) in float a_z_order;   // depth for z-sorting
layout(location = 7) in float a_palette_id; // palette index for color swap
layout(location = 8) in vec4 a_mask_params; // material mask parameters

uniform mat4 u_projection;
uniform float u_time;       // game time for animated effects

out vec2 v_uv;
out vec4 v_tint;
out float v_palette_id;
out vec4 v_mask_params;

void main() {
    vec2 pos = a_screen_pos + a_quad_pos * a_sprite_size;
    gl_Position = u_projection * vec4(pos, a_z_order, 1.0);

    v_uv = a_uv_rect.xy + a_quad_uv * a_uv_rect.zw;
    v_tint = a_tint;
    v_palette_id = a_palette_id;
    v_mask_params = a_mask_params;
}
`;

/**
 * Fragment shader: sample atlas texture, apply material mask, palette swap.
 *
 * Material channels (stored in mask texture):
 * - R channel: roof material
 * - G channel: wall material
 * - B channel: glass/window material
 * - A channel: vegetation material
 *
 * Palette swap works by:
 * 1. Sample base color from atlas
 * 2. Sample material mask
 * 3. For each material channel with weight > 0, look up palette color
 * 4. Blend palette colors based on mask weights
 * 5. Apply tint and time-of-day lighting
 */
export const PALETTE_FRAGMENT_GLSL = `#version 300 es
precision highp float;

uniform sampler2D u_atlas;      // sprite atlas texture
uniform sampler2D u_mask;       // material mask texture (optional)
uniform sampler2D u_palette;    // palette lookup texture (N palettes x M colors)
uniform vec3 u_sun_color;       // time-of-day sun color
uniform float u_sun_intensity;  // time-of-day sun intensity (0-1)
uniform float u_emissive_power; // night-time emissive glow strength

in vec2 v_uv;
in vec4 v_tint;
in float v_palette_id;
in vec4 v_mask_params;

out vec4 fragColor;

vec3 samplePalette(float paletteId, float materialIndex) {
    // Palette texture: row = palette ID, column = material index
    vec2 palUV = vec2(materialIndex / 4.0 + 0.125, (paletteId + 0.5) / 64.0);
    return texture(u_palette, palUV).rgb;
}

void main() {
    vec4 baseColor = texture(u_atlas, v_uv);
    if (baseColor.a < 0.01) discard;

    vec3 finalColor = baseColor.rgb;

    // Material mask application (if mask params are active)
    if (v_mask_params.x + v_mask_params.y + v_mask_params.z + v_mask_params.w > 0.0) {
        vec4 mask = texture(u_mask, v_uv);

        vec3 roofColor = samplePalette(v_palette_id, 0.0);
        vec3 wallColor = samplePalette(v_palette_id, 1.0);
        vec3 glassColor = samplePalette(v_palette_id, 2.0);
        vec3 vegColor = samplePalette(v_palette_id, 3.0);

        float totalWeight = mask.r + mask.g + mask.b + mask.a;
        if (totalWeight > 0.0) {
            vec3 paletteColor = (
                roofColor * mask.r +
                wallColor * mask.g +
                glassColor * mask.b +
                vegColor * mask.a
            ) / totalWeight;

            // Blend base color with palette based on mask strength
            float maskStrength = min(totalWeight, 1.0);
            finalColor = mix(baseColor.rgb, paletteColor, maskStrength);
        }
    }

    // Apply tint
    finalColor *= v_tint.rgb;

    // Time-of-day lighting
    finalColor *= u_sun_color * u_sun_intensity;

    // Emissive for glass at night (B channel of mask = glass)
    if (u_emissive_power > 0.0 && v_mask_params.z > 0.0) {
        vec4 mask = texture(u_mask, v_uv);
        finalColor += vec3(1.0, 0.95, 0.8) * mask.b * u_emissive_power;
    }

    fragColor = vec4(finalColor, baseColor.a * v_tint.a);
}
`;

// ── WGSL (WebGPU) ───────────────────────────────────────────────────────────

export const PALETTE_VERTEX_WGSL = `
struct VertexInput {
    @location(0) quad_pos: vec2<f32>,
    @location(1) quad_uv: vec2<f32>,
    @location(2) screen_pos: vec2<f32>,
    @location(3) sprite_size: vec2<f32>,
    @location(4) uv_rect: vec4<f32>,
    @location(5) tint: vec4<f32>,
    @location(6) z_order: f32,
    @location(7) palette_id: f32,
    @location(8) mask_params: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) palette_id: f32,
    @location(3) mask_params: vec4<f32>,
};

struct Uniforms {
    projection: mat4x4<f32>,
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let pos = input.screen_pos + input.quad_pos * input.sprite_size;
    output.position = uniforms.projection * vec4<f32>(pos, input.z_order, 1.0);
    output.uv = input.uv_rect.xy + input.quad_uv * input.uv_rect.zw;
    output.tint = input.tint;
    output.palette_id = input.palette_id;
    output.mask_params = input.mask_params;
    return output;
}
`;

export const PALETTE_FRAGMENT_WGSL = `
struct FragUniforms {
    sun_color: vec3<f32>,
    sun_intensity: f32,
    emissive_power: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(1) @binding(0) var<uniform> frag_uniforms: FragUniforms;
@group(1) @binding(1) var atlas_texture: texture_2d<f32>;
@group(1) @binding(2) var atlas_sampler: sampler;
@group(1) @binding(3) var mask_texture: texture_2d<f32>;
@group(1) @binding(4) var palette_texture: texture_2d<f32>;

fn sample_palette(palette_id: f32, material_index: f32) -> vec3<f32> {
    let pal_uv = vec2<f32>(material_index / 4.0 + 0.125, (palette_id + 0.5) / 64.0);
    return textureSample(palette_texture, atlas_sampler, pal_uv).rgb;
}

@fragment
fn fs_main(
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) palette_id: f32,
    @location(3) mask_params: vec4<f32>,
) -> @location(0) vec4<f32> {
    let base_color = textureSample(atlas_texture, atlas_sampler, uv);
    if (base_color.a < 0.01) { discard; }

    var final_color = base_color.rgb;

    let mask_sum = mask_params.x + mask_params.y + mask_params.z + mask_params.w;
    if (mask_sum > 0.0) {
        let mask = textureSample(mask_texture, atlas_sampler, uv);

        let roof_color = sample_palette(palette_id, 0.0);
        let wall_color = sample_palette(palette_id, 1.0);
        let glass_color = sample_palette(palette_id, 2.0);
        let veg_color = sample_palette(palette_id, 3.0);

        let total_weight = mask.r + mask.g + mask.b + mask.a;
        if (total_weight > 0.0) {
            let palette_color = (
                roof_color * mask.r +
                wall_color * mask.g +
                glass_color * mask.b +
                veg_color * mask.a
            ) / total_weight;

            let mask_strength = min(total_weight, 1.0);
            final_color = mix(base_color.rgb, palette_color, mask_strength);
        }
    }

    final_color = final_color * tint.rgb;
    final_color = final_color * frag_uniforms.sun_color * frag_uniforms.sun_intensity;

    if (frag_uniforms.emissive_power > 0.0 && mask_params.z > 0.0) {
        let mask = textureSample(mask_texture, atlas_sampler, uv);
        final_color = final_color + vec3<f32>(1.0, 0.95, 0.8) * mask.b * frag_uniforms.emissive_power;
    }

    return vec4<f32>(final_color, base_color.a * tint.a);
}
`;

// ── Constants & Helpers ─────────────────────────────────────────────────────

/** Material mask channel indices */
export enum MaterialChannel {
  Roof = 0,
  Wall = 1,
  Glass = 2,
  Vegetation = 3,
}

/** Shader uniform names */
export const UNIFORM_NAMES = {
  projection: "u_projection",
  time: "u_time",
  atlas: "u_atlas",
  mask: "u_mask",
  palette: "u_palette",
  sunColor: "u_sun_color",
  sunIntensity: "u_sun_intensity",
  emissivePower: "u_emissive_power",
} as const;

/** Default time-of-day values */
export const DEFAULT_LIGHTING = {
  sunColor: [1.0, 0.98, 0.92] as readonly [number, number, number],
  sunIntensity: 1.0,
  emissivePower: 0.0,
};

/** Night lighting values */
export const NIGHT_LIGHTING = {
  sunColor: [0.3, 0.3, 0.5] as readonly [number, number, number],
  sunIntensity: 0.4,
  emissivePower: 0.8,
};

/**
 * Compute time-of-day lighting from game hour (0-23).
 */
export function computeLighting(gameHour: number): {
  sunColor: [number, number, number];
  sunIntensity: number;
  emissivePower: number;
} {
  // Sunrise: 6-8, sunset: 18-20
  if (gameHour >= 8 && gameHour < 18) {
    return {
      ...DEFAULT_LIGHTING,
      sunColor: [...DEFAULT_LIGHTING.sunColor],
    };
  } else if (gameHour >= 20 || gameHour < 6) {
    return {
      ...NIGHT_LIGHTING,
      sunColor: [...NIGHT_LIGHTING.sunColor],
    };
  } else if (gameHour >= 6 && gameHour < 8) {
    // Sunrise transition
    const t = (gameHour - 6) / 2;
    return {
      sunColor: [
        NIGHT_LIGHTING.sunColor[0] +
          (DEFAULT_LIGHTING.sunColor[0] - NIGHT_LIGHTING.sunColor[0]) * t,
        NIGHT_LIGHTING.sunColor[1] +
          (DEFAULT_LIGHTING.sunColor[1] - NIGHT_LIGHTING.sunColor[1]) * t,
        NIGHT_LIGHTING.sunColor[2] +
          (DEFAULT_LIGHTING.sunColor[2] - NIGHT_LIGHTING.sunColor[2]) * t,
      ],
      sunIntensity:
        NIGHT_LIGHTING.sunIntensity +
        (DEFAULT_LIGHTING.sunIntensity - NIGHT_LIGHTING.sunIntensity) * t,
      emissivePower: NIGHT_LIGHTING.emissivePower * (1 - t),
    };
  } else {
    // Sunset transition (18-20)
    const t = (gameHour - 18) / 2;
    return {
      sunColor: [
        DEFAULT_LIGHTING.sunColor[0] +
          (NIGHT_LIGHTING.sunColor[0] - DEFAULT_LIGHTING.sunColor[0]) * t,
        DEFAULT_LIGHTING.sunColor[1] +
          (NIGHT_LIGHTING.sunColor[1] - DEFAULT_LIGHTING.sunColor[1]) * t,
        DEFAULT_LIGHTING.sunColor[2] +
          (NIGHT_LIGHTING.sunColor[2] - DEFAULT_LIGHTING.sunColor[2]) * t,
      ],
      sunIntensity:
        DEFAULT_LIGHTING.sunIntensity +
        (NIGHT_LIGHTING.sunIntensity - DEFAULT_LIGHTING.sunIntensity) * t,
      emissivePower: NIGHT_LIGHTING.emissivePower * t,
    };
  }
}
