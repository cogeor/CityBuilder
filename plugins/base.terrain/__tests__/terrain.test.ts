import { describe, it, expect } from "vitest";
import {
  TERRAIN_TYPES,
  ELEVATION_RULES,
  getTerrainType,
  terrainForElevation,
  isBuildable,
  computeWaterEdgeMask,
  selectVariant,
  validateTerrainConfig,
} from "../terrain.js";

// ─── TERRAIN_TYPES ───────────────────────────────────────────────────────────

describe("TERRAIN_TYPES", () => {
  it("has 5 entries", () => {
    expect(TERRAIN_TYPES).toHaveLength(5);
  });

  it("grass is buildable", () => {
    const grass = TERRAIN_TYPES.find(t => t.name === "Grass");
    expect(grass).toBeDefined();
    expect(grass!.buildable).toBe(true);
  });

  it("water is not buildable", () => {
    const water = TERRAIN_TYPES.find(t => t.name === "Water");
    expect(water).toBeDefined();
    expect(water!.buildable).toBe(false);
  });

  it("forest is not buildable", () => {
    const forest = TERRAIN_TYPES.find(t => t.name === "Forest");
    expect(forest).toBeDefined();
    expect(forest!.buildable).toBe(false);
  });
});

// ─── getTerrainType ──────────────────────────────────────────────────────────

describe("getTerrainType", () => {
  it("finds by id", () => {
    const grass = getTerrainType(0);
    expect(grass).toBeDefined();
    expect(grass!.name).toBe("Grass");
  });

  it("returns undefined for unknown id", () => {
    expect(getTerrainType(99)).toBeUndefined();
  });
});

// ─── terrainForElevation ─────────────────────────────────────────────────────

describe("terrainForElevation", () => {
  it("returns water at elevation 0", () => {
    expect(terrainForElevation(0)).toBe(1); // Water
  });

  it("returns sand at elevation 1-2", () => {
    expect(terrainForElevation(1)).toBe(2); // Sand
    expect(terrainForElevation(2)).toBe(2);
  });

  it("returns grass at elevation 3-10", () => {
    expect(terrainForElevation(3)).toBe(0);  // Grass
    expect(terrainForElevation(7)).toBe(0);
    expect(terrainForElevation(10)).toBe(0);
  });

  it("returns forest at elevation 11-13", () => {
    expect(terrainForElevation(11)).toBe(3); // Forest
    expect(terrainForElevation(13)).toBe(3);
  });

  it("returns rock at elevation 14-15", () => {
    expect(terrainForElevation(14)).toBe(4); // Rock
    expect(terrainForElevation(15)).toBe(4);
  });

  it("returns grass as default for out-of-range elevation", () => {
    expect(terrainForElevation(100)).toBe(0);
  });
});

// ─── isBuildable ─────────────────────────────────────────────────────────────

describe("isBuildable", () => {
  it("returns true for grass", () => {
    expect(isBuildable(0)).toBe(true);
  });

  it("returns false for water", () => {
    expect(isBuildable(1)).toBe(false);
  });

  it("returns true for sand", () => {
    expect(isBuildable(2)).toBe(true);
  });

  it("returns false for unknown terrain", () => {
    expect(isBuildable(99)).toBe(false);
  });
});

// ─── computeWaterEdgeMask ────────────────────────────────────────────────────

describe("computeWaterEdgeMask", () => {
  it("returns 0 for non-water tile", () => {
    const isWater = () => false;
    expect(computeWaterEdgeMask(isWater, 5, 5)).toBe(0);
  });

  it("detects north edge", () => {
    const isWater = (x: number, y: number) => !(x === 5 && y === 4);
    expect(computeWaterEdgeMask(isWater, 5, 5)).toBe(1); // North
  });

  it("detects all edges when surrounded by land", () => {
    const isWater = (x: number, y: number) => x === 5 && y === 5;
    expect(computeWaterEdgeMask(isWater, 5, 5)).toBe(15); // N|E|S|W = 1|2|4|8
  });

  it("returns 0 when surrounded by water", () => {
    const isWater = () => true;
    expect(computeWaterEdgeMask(isWater, 5, 5)).toBe(0);
  });
});

// ─── selectVariant ───────────────────────────────────────────────────────────

describe("selectVariant", () => {
  it("is deterministic", () => {
    const v1 = selectVariant(0, 10, 20);
    const v2 = selectVariant(0, 10, 20);
    expect(v1).toBe(v2);
  });

  it("returns 0 for single-variant terrain", () => {
    // Water has variants: 1
    expect(selectVariant(1, 10, 20)).toBe(0);
  });

  it("returns a value within variant range", () => {
    // Grass has variants: 4
    for (let x = 0; x < 20; x++) {
      for (let y = 0; y < 20; y++) {
        const v = selectVariant(0, x, y);
        expect(v).toBeGreaterThanOrEqual(0);
        expect(v).toBeLessThan(4);
      }
    }
  });
});

// ─── validateTerrainConfig ───────────────────────────────────────────────────

describe("validateTerrainConfig", () => {
  it("returns no errors for default configuration", () => {
    const errors = validateTerrainConfig();
    expect(errors).toHaveLength(0);
  });
});
