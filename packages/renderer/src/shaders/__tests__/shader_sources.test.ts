import { describe, it, expect } from "vitest";
import {
  PALETTE_VERTEX_GLSL,
  PALETTE_FRAGMENT_GLSL,
  PALETTE_VERTEX_WGSL,
  PALETTE_FRAGMENT_WGSL,
  MaterialChannel,
  UNIFORM_NAMES,
  DEFAULT_LIGHTING,
  NIGHT_LIGHTING,
  computeLighting,
} from "../shader_sources.js";

// ── GLSL shader string tests ────────────────────────────────────────────────

describe("GLSL shaders", () => {
  it("PALETTE_VERTEX_GLSL contains version 300 es", () => {
    expect(PALETTE_VERTEX_GLSL).toContain("#version 300 es");
  });

  it("PALETTE_FRAGMENT_GLSL contains sampler2D uniforms", () => {
    expect(PALETTE_FRAGMENT_GLSL).toContain("uniform sampler2D u_atlas");
    expect(PALETTE_FRAGMENT_GLSL).toContain("uniform sampler2D u_mask");
    expect(PALETTE_FRAGMENT_GLSL).toContain("uniform sampler2D u_palette");
  });

  it("GLSL shaders contain material mask sampling logic", () => {
    expect(PALETTE_FRAGMENT_GLSL).toContain("samplePalette");
    expect(PALETTE_FRAGMENT_GLSL).toContain("mask.r");
    expect(PALETTE_FRAGMENT_GLSL).toContain("mask.g");
    expect(PALETTE_FRAGMENT_GLSL).toContain("mask.b");
    expect(PALETTE_FRAGMENT_GLSL).toContain("mask.a");
    expect(PALETTE_FRAGMENT_GLSL).toContain("totalWeight");
  });
});

// ── WGSL shader string tests ────────────────────────────────────────────────

describe("WGSL shaders", () => {
  it("PALETTE_VERTEX_WGSL contains @vertex fn", () => {
    expect(PALETTE_VERTEX_WGSL).toContain("@vertex");
    expect(PALETTE_VERTEX_WGSL).toContain("fn vs_main");
  });

  it("PALETTE_FRAGMENT_WGSL contains @fragment fn", () => {
    expect(PALETTE_FRAGMENT_WGSL).toContain("@fragment");
    expect(PALETTE_FRAGMENT_WGSL).toContain("fn fs_main");
  });

  it("WGSL shaders contain palette lookup function", () => {
    expect(PALETTE_FRAGMENT_WGSL).toContain("fn sample_palette");
    expect(PALETTE_FRAGMENT_WGSL).toContain("textureSample");
    expect(PALETTE_FRAGMENT_WGSL).toContain("palette_texture");
  });
});

// ── Enum & constants tests ──────────────────────────────────────────────────

describe("MaterialChannel", () => {
  it("has 4 values (0-3)", () => {
    expect(MaterialChannel.Roof).toBe(0);
    expect(MaterialChannel.Wall).toBe(1);
    expect(MaterialChannel.Glass).toBe(2);
    expect(MaterialChannel.Vegetation).toBe(3);
  });
});

describe("UNIFORM_NAMES", () => {
  it("has all expected keys", () => {
    expect(UNIFORM_NAMES).toHaveProperty("projection", "u_projection");
    expect(UNIFORM_NAMES).toHaveProperty("time", "u_time");
    expect(UNIFORM_NAMES).toHaveProperty("atlas", "u_atlas");
    expect(UNIFORM_NAMES).toHaveProperty("mask", "u_mask");
    expect(UNIFORM_NAMES).toHaveProperty("palette", "u_palette");
    expect(UNIFORM_NAMES).toHaveProperty("sunColor", "u_sun_color");
    expect(UNIFORM_NAMES).toHaveProperty("sunIntensity", "u_sun_intensity");
    expect(UNIFORM_NAMES).toHaveProperty("emissivePower", "u_emissive_power");
  });
});

describe("Lighting constants", () => {
  it("DEFAULT_LIGHTING has correct structure", () => {
    expect(DEFAULT_LIGHTING.sunColor).toHaveLength(3);
    expect(DEFAULT_LIGHTING.sunIntensity).toBe(1.0);
    expect(DEFAULT_LIGHTING.emissivePower).toBe(0.0);
    expect(DEFAULT_LIGHTING.sunColor[0]).toBeCloseTo(1.0);
    expect(DEFAULT_LIGHTING.sunColor[1]).toBeCloseTo(0.98);
    expect(DEFAULT_LIGHTING.sunColor[2]).toBeCloseTo(0.92);
  });

  it("NIGHT_LIGHTING has lower intensity", () => {
    expect(NIGHT_LIGHTING.sunIntensity).toBeLessThan(
      DEFAULT_LIGHTING.sunIntensity,
    );
    expect(NIGHT_LIGHTING.emissivePower).toBeGreaterThan(0);
    expect(NIGHT_LIGHTING.sunIntensity).toBe(0.4);
    expect(NIGHT_LIGHTING.emissivePower).toBe(0.8);
  });
});

// ── computeLighting tests ───────────────────────────────────────────────────

describe("computeLighting", () => {
  it("at noon returns day values", () => {
    const result = computeLighting(12);
    expect(result.sunIntensity).toBe(DEFAULT_LIGHTING.sunIntensity);
    expect(result.emissivePower).toBe(DEFAULT_LIGHTING.emissivePower);
    expect(result.sunColor[0]).toBeCloseTo(DEFAULT_LIGHTING.sunColor[0]);
    expect(result.sunColor[1]).toBeCloseTo(DEFAULT_LIGHTING.sunColor[1]);
    expect(result.sunColor[2]).toBeCloseTo(DEFAULT_LIGHTING.sunColor[2]);
  });

  it("at midnight returns night values", () => {
    const result = computeLighting(0);
    expect(result.sunIntensity).toBe(NIGHT_LIGHTING.sunIntensity);
    expect(result.emissivePower).toBe(NIGHT_LIGHTING.emissivePower);
    expect(result.sunColor[0]).toBeCloseTo(NIGHT_LIGHTING.sunColor[0]);
    expect(result.sunColor[1]).toBeCloseTo(NIGHT_LIGHTING.sunColor[1]);
    expect(result.sunColor[2]).toBeCloseTo(NIGHT_LIGHTING.sunColor[2]);
  });

  it("at sunrise (7:00) returns interpolated values", () => {
    const result = computeLighting(7);
    // t = (7-6)/2 = 0.5 => midpoint between night and day
    const expectedIntensity =
      NIGHT_LIGHTING.sunIntensity +
      (DEFAULT_LIGHTING.sunIntensity - NIGHT_LIGHTING.sunIntensity) * 0.5;
    expect(result.sunIntensity).toBeCloseTo(expectedIntensity);
    expect(result.sunIntensity).toBeGreaterThan(NIGHT_LIGHTING.sunIntensity);
    expect(result.sunIntensity).toBeLessThan(DEFAULT_LIGHTING.sunIntensity);
    expect(result.emissivePower).toBeCloseTo(
      NIGHT_LIGHTING.emissivePower * 0.5,
    );
  });

  it("at sunset (19:00) returns interpolated values", () => {
    const result = computeLighting(19);
    // t = (19-18)/2 = 0.5 => midpoint between day and night
    const expectedIntensity =
      DEFAULT_LIGHTING.sunIntensity +
      (NIGHT_LIGHTING.sunIntensity - DEFAULT_LIGHTING.sunIntensity) * 0.5;
    expect(result.sunIntensity).toBeCloseTo(expectedIntensity);
    expect(result.sunIntensity).toBeGreaterThan(NIGHT_LIGHTING.sunIntensity);
    expect(result.sunIntensity).toBeLessThan(DEFAULT_LIGHTING.sunIntensity);
    expect(result.emissivePower).toBeCloseTo(
      NIGHT_LIGHTING.emissivePower * 0.5,
    );
  });

  it("intensity ranges between night and day values", () => {
    for (let h = 0; h < 24; h++) {
      const result = computeLighting(h);
      expect(result.sunIntensity).toBeGreaterThanOrEqual(
        NIGHT_LIGHTING.sunIntensity,
      );
      expect(result.sunIntensity).toBeLessThanOrEqual(
        DEFAULT_LIGHTING.sunIntensity,
      );
      expect(result.emissivePower).toBeGreaterThanOrEqual(0);
      expect(result.emissivePower).toBeLessThanOrEqual(
        NIGHT_LIGHTING.emissivePower,
      );
    }
  });
});
