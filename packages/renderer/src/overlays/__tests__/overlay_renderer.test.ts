import { describe, it, expect } from "vitest";
import {
  OverlayType,
  ZoneDisplayType,
  StatusIconType,
  GRADIENT_GREEN_RED,
  GRADIENT_BLUE_BROWN,
  GRADIENT_COOL_HOT,
  ZONE_COLORS,
  DEFAULT_OVERLAY_CONFIGS,
  OVERLAY_SPRITE_ID,
  STATUS_ICON_SPRITES,
  sampleGradient,
  normalizeHeatmapValue,
  OverlayRenderer,
  type GradientStop,
} from "../overlay_renderer.js";
import { INSTANCE_BYTE_SIZE, unpackInstance } from "../../types/types.js";
import type { CameraState } from "../../projection/projection.js";

/** Helper: create a simple camera centered at origin. */
function makeCamera(opts?: Partial<CameraState>): CameraState {
  return {
    x: 0,
    y: 0,
    zoom: 1,
    viewportWidth: 800,
    viewportHeight: 600,
    ...opts,
  };
}

/** Helper: unpack all instances from a Float32Array. */
function unpackAll(buffer: Float32Array, count: number) {
  const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
  const result = [];
  for (let i = 0; i < count; i++) {
    result.push(unpackInstance(view, i * INSTANCE_BYTE_SIZE));
  }
  return result;
}

describe("overlay_renderer", () => {
  // ─── sampleGradient ─────────────────────────────────────────────────────

  describe("sampleGradient", () => {
    it("at value 0 returns first stop color", () => {
      const color = sampleGradient(GRADIENT_GREEN_RED, 0.0);
      expect(color.r).toBe(0);
      expect(color.g).toBe(200);
      expect(color.b).toBe(0);
      expect(color.a).toBe(160);
    });

    it("at value 1 returns last stop color", () => {
      const color = sampleGradient(GRADIENT_GREEN_RED, 1.0);
      expect(color.r).toBe(200);
      expect(color.g).toBe(0);
      expect(color.b).toBe(0);
      expect(color.a).toBe(160);
    });

    it("at 0.5 interpolates between stops", () => {
      const color = sampleGradient(GRADIENT_GREEN_RED, 0.5);
      expect(color.r).toBe(200);
      expect(color.g).toBe(200);
      expect(color.b).toBe(0);
      expect(color.a).toBe(160);
    });

    it("with empty gradient returns black transparent", () => {
      const color = sampleGradient([], 0.5);
      expect(color.r).toBe(0);
      expect(color.g).toBe(0);
      expect(color.b).toBe(0);
      expect(color.a).toBe(0);
    });

    it("clamps values below 0", () => {
      const color = sampleGradient(GRADIENT_GREEN_RED, -0.5);
      expect(color.r).toBe(0);
      expect(color.g).toBe(200);
      expect(color.b).toBe(0);
    });

    it("clamps values above 1", () => {
      const color = sampleGradient(GRADIENT_GREEN_RED, 1.5);
      expect(color.r).toBe(200);
      expect(color.g).toBe(0);
      expect(color.b).toBe(0);
    });

    it("interpolates mid-range correctly for green-red gradient", () => {
      // At 0.25, between stop 0 (green) and stop 0.5 (yellow)
      const color = sampleGradient(GRADIENT_GREEN_RED, 0.25);
      expect(color.r).toBe(100); // halfway from 0 to 200
      expect(color.g).toBe(200); // stays 200
      expect(color.b).toBe(0);
    });

    it("handles single-stop gradient", () => {
      const single: GradientStop[] = [{ value: 0.5, color: { r: 100, g: 50, b: 25, a: 200 } }];
      const color = sampleGradient(single, 0.0);
      expect(color.r).toBe(100);
      expect(color.g).toBe(50);
      expect(color.b).toBe(25);
      expect(color.a).toBe(200);
    });
  });

  // ─── normalizeHeatmapValue ──────────────────────────────────────────────

  describe("normalizeHeatmapValue", () => {
    it("at 0 returns 0", () => {
      expect(normalizeHeatmapValue(0)).toBe(0);
    });

    it("at 65535 returns 1", () => {
      expect(normalizeHeatmapValue(65535)).toBe(1);
    });

    it("at midpoint returns approximately 0.5", () => {
      const result = normalizeHeatmapValue(32768);
      expect(result).toBeGreaterThan(0.49);
      expect(result).toBeLessThan(0.51);
    });

    it("with custom maxValue", () => {
      expect(normalizeHeatmapValue(50, 100)).toBeCloseTo(0.5);
    });

    it("clamps negative values to 0", () => {
      expect(normalizeHeatmapValue(-10)).toBe(0);
    });
  });

  // ─── OverlayRenderer ───────────────────────────────────────────────────

  describe("OverlayRenderer", () => {
    it("starts with None overlay", () => {
      const renderer = new OverlayRenderer(makeCamera());
      expect(renderer.getOverlay()).toBe(OverlayType.None);
    });

    it("setOverlay changes active overlay", () => {
      const renderer = new OverlayRenderer(makeCamera());
      renderer.setOverlay(OverlayType.Traffic);
      expect(renderer.getOverlay()).toBe(OverlayType.Traffic);
    });

    it("toggleOverlay enables overlay", () => {
      const renderer = new OverlayRenderer(makeCamera());
      renderer.toggleOverlay(OverlayType.Power);
      expect(renderer.getOverlay()).toBe(OverlayType.Power);
    });

    it("toggleOverlay disables if already active", () => {
      const renderer = new OverlayRenderer(makeCamera());
      renderer.setOverlay(OverlayType.Crime);
      renderer.toggleOverlay(OverlayType.Crime);
      expect(renderer.getOverlay()).toBe(OverlayType.None);
    });
  });

  // ─── buildHeatmapInstances ──────────────────────────────────────────────

  describe("buildHeatmapInstances", () => {
    it("generates instances for visible tiles", () => {
      // Small camera viewing a small map area
      const camera = makeCamera({ x: 1, y: 1, zoom: 1, viewportWidth: 400, viewportHeight: 300 });
      const renderer = new OverlayRenderer(camera);
      renderer.setOverlay(OverlayType.Traffic);

      const result = renderer.buildHeatmapInstances(
        (_x, _y) => 32768, // mid value
        10,
        10,
      );

      expect(result.count).toBeGreaterThan(0);
      expect(result.instances.byteLength).toBe(result.count * INSTANCE_BYTE_SIZE);
      expect(result.stats.activeOverlay).toBe(OverlayType.Traffic);
      expect(result.stats.tileCount).toBe(result.count);
    });

    it("uses correct gradient colors", () => {
      // Tiny camera: just center on tile (0,0)
      const camera = makeCamera({ x: 0, y: 0, zoom: 0.1, viewportWidth: 200, viewportHeight: 200 });
      const renderer = new OverlayRenderer(camera);
      renderer.setOverlay(OverlayType.Traffic); // Uses GREEN_RED gradient

      const result = renderer.buildHeatmapInstances(
        (_x, _y) => 0, // value 0 => green
        100,
        100,
      );

      expect(result.count).toBeGreaterThan(0);
      const instances = unpackAll(result.instances, result.count);
      // First instance should have green-ish tint (r=0, g=200, b=0)
      const first = instances[0];
      expect(first.tint_r).toBe(0);
      expect(first.tint_g).toBe(200);
      expect(first.tint_b).toBe(0);
    });

    it("returns empty when overlay is None", () => {
      const renderer = new OverlayRenderer(makeCamera());
      // activeOverlay is None by default
      const result = renderer.buildHeatmapInstances(() => 100, 10, 10);
      expect(result.count).toBe(0);
      expect(result.instances.byteLength).toBe(0);
    });

    it("returns empty when overlay is Zoning (no gradient)", () => {
      const renderer = new OverlayRenderer(makeCamera());
      renderer.setOverlay(OverlayType.Zoning);
      const result = renderer.buildHeatmapInstances(() => 100, 10, 10);
      expect(result.count).toBe(0);
    });
  });

  // ─── buildZoningInstances ───────────────────────────────────────────────

  describe("buildZoningInstances", () => {
    it("generates colored tiles for zoned areas", () => {
      const camera = makeCamera({ x: 1, y: 1, zoom: 1, viewportWidth: 400, viewportHeight: 300 });
      const renderer = new OverlayRenderer(camera);

      const result = renderer.buildZoningInstances(
        (_x, _y) => ZoneDisplayType.Residential,
        10,
        10,
      );

      expect(result.count).toBeGreaterThan(0);
      expect(result.stats.activeOverlay).toBe(OverlayType.Zoning);

      const instances = unpackAll(result.instances, result.count);
      // Should be green-ish for residential
      expect(instances[0].tint_r).toBe(50);
      expect(instances[0].tint_g).toBe(200);
      expect(instances[0].tint_b).toBe(50);
    });

    it("skips None zones", () => {
      const camera = makeCamera({ x: 1, y: 1, zoom: 1, viewportWidth: 400, viewportHeight: 300 });
      const renderer = new OverlayRenderer(camera);

      const result = renderer.buildZoningInstances(
        (_x, _y) => ZoneDisplayType.None,
        10,
        10,
      );

      expect(result.count).toBe(0);
    });

    it("uses OVERLAY_SPRITE_ID for zone tiles", () => {
      const camera = makeCamera({ x: 0, y: 0, zoom: 0.1, viewportWidth: 200, viewportHeight: 200 });
      const renderer = new OverlayRenderer(camera);

      const result = renderer.buildZoningInstances(
        (_x, _y) => ZoneDisplayType.Commercial,
        100,
        100,
      );

      expect(result.count).toBeGreaterThan(0);
      const instances = unpackAll(result.instances, result.count);
      expect(instances[0].sprite_id).toBe(OVERLAY_SPRITE_ID);
    });
  });

  // ─── buildStatusIcons ───────────────────────────────────────────────────

  describe("buildStatusIcons", () => {
    it("generates icon instances", () => {
      const renderer = new OverlayRenderer(makeCamera());
      const result = renderer.buildStatusIcons([
        { tileX: 5, tileY: 5, icon: StatusIconType.NoPower },
        { tileX: 10, tileY: 10, icon: StatusIconType.OnFire },
      ]);

      expect(result.count).toBe(2);
      expect(result.instances.byteLength).toBe(2 * INSTANCE_BYTE_SIZE);

      const instances = unpackAll(result.instances, result.count);
      expect(instances[0].sprite_id).toBe(STATUS_ICON_SPRITES[StatusIconType.NoPower]);
      expect(instances[1].sprite_id).toBe(STATUS_ICON_SPRITES[StatusIconType.OnFire]);
    });

    it("handles empty array", () => {
      const renderer = new OverlayRenderer(makeCamera());
      const result = renderer.buildStatusIcons([]);
      expect(result.count).toBe(0);
      expect(result.instances.byteLength).toBe(0);
    });

    it("uses correct status icon sprite IDs", () => {
      const renderer = new OverlayRenderer(makeCamera());
      const result = renderer.buildStatusIcons([
        { tileX: 0, tileY: 0, icon: StatusIconType.NoWater },
        { tileX: 1, tileY: 1, icon: StatusIconType.Abandoned },
      ]);

      const instances = unpackAll(result.instances, result.count);
      expect(instances[0].sprite_id).toBe(9601); // NoWater
      expect(instances[1].sprite_id).toBe(9603); // Abandoned
    });
  });

  // ─── Constants and Presets ──────────────────────────────────────────────

  describe("constants", () => {
    it("OVERLAY_SPRITE_ID is 9500", () => {
      expect(OVERLAY_SPRITE_ID).toBe(9500);
    });

    it("DEFAULT_OVERLAY_CONFIGS has all overlay types", () => {
      expect(DEFAULT_OVERLAY_CONFIGS[OverlayType.None].name).toBe("None");
      expect(DEFAULT_OVERLAY_CONFIGS[OverlayType.Traffic].name).toBe("Traffic");
      expect(DEFAULT_OVERLAY_CONFIGS[OverlayType.Zoning].name).toBe("Zoning");
    });

    it("ZONE_COLORS has all zone types", () => {
      expect(ZONE_COLORS[ZoneDisplayType.None].a).toBe(0);
      expect(ZONE_COLORS[ZoneDisplayType.Residential].g).toBe(200);
      expect(ZONE_COLORS[ZoneDisplayType.Commercial].b).toBe(200);
      expect(ZONE_COLORS[ZoneDisplayType.Industrial].r).toBe(200);
    });

    it("gradient presets have 3 stops each", () => {
      expect(GRADIENT_GREEN_RED.length).toBe(3);
      expect(GRADIENT_BLUE_BROWN.length).toBe(3);
      expect(GRADIENT_COOL_HOT.length).toBe(3);
    });
  });
});
