import { describe, it, expect } from "vitest";
import {
  MAP_SIZES,
  COUNTRY_PRESETS,
  TIME_DEFAULTS,
  WORLD_DEFAULTS,
  getMapSize,
  getCountryPreset,
} from "../presets.js";

// ─── MAP_SIZES ───────────────────────────────────────────────────────────────

describe("MAP_SIZES", () => {
  it("has 3 entries", () => {
    expect(MAP_SIZES).toHaveLength(3);
  });

  it("small map is 128x128", () => {
    const small = MAP_SIZES.find(m => m.id === "small");
    expect(small).toBeDefined();
    expect(small!.width).toBe(128);
    expect(small!.height).toBe(128);
  });

  it("medium map is 192x192", () => {
    const medium = MAP_SIZES.find(m => m.id === "medium");
    expect(medium).toBeDefined();
    expect(medium!.width).toBe(192);
    expect(medium!.height).toBe(192);
  });

  it("large map is 256x256", () => {
    const large = MAP_SIZES.find(m => m.id === "large");
    expect(large).toBeDefined();
    expect(large!.width).toBe(256);
    expect(large!.height).toBe(256);
  });
});

// ─── COUNTRY_PRESETS ─────────────────────────────────────────────────────────

describe("COUNTRY_PRESETS", () => {
  it("has 3 entries", () => {
    expect(COUNTRY_PRESETS).toHaveLength(3);
  });

  it("generic preset has default values", () => {
    const generic = COUNTRY_PRESETS.find(c => c.id === "generic");
    expect(generic).toBeDefined();
    expect(generic!.populationGrowthModifier).toBe(1.0);
    expect(generic!.industrialModifier).toBe(1.0);
    expect(generic!.defaultTaxRate).toBe(0.09);
  });

  it("US preset has modified growth", () => {
    const us = COUNTRY_PRESETS.find(c => c.id === "us");
    expect(us).toBeDefined();
    expect(us!.populationGrowthModifier).toBe(1.1);
    expect(us!.industrialModifier).toBe(0.9);
  });

  it("France preset has euro currency", () => {
    const france = COUNTRY_PRESETS.find(c => c.id === "france");
    expect(france).toBeDefined();
    expect(france!.currency).toBe("Euro");
    expect(france!.currencySymbol).toBe("\u20ac");
    expect(france!.language).toBe("fr");
  });
});

// ─── getMapSize ──────────────────────────────────────────────────────────────

describe("getMapSize", () => {
  it("finds by id", () => {
    const small = getMapSize("small");
    expect(small).toBeDefined();
    expect(small!.name).toBe("Small");
  });

  it("returns undefined for unknown id", () => {
    expect(getMapSize("tiny")).toBeUndefined();
  });
});

// ─── getCountryPreset ────────────────────────────────────────────────────────

describe("getCountryPreset", () => {
  it("finds by id", () => {
    const us = getCountryPreset("us");
    expect(us).toBeDefined();
    expect(us!.name).toBe("United States");
  });

  it("returns undefined for unknown id", () => {
    expect(getCountryPreset("japan")).toBeUndefined();
  });
});

// ─── WORLD_DEFAULTS ──────────────────────────────────────────────────────────

describe("WORLD_DEFAULTS", () => {
  it("has positive starting treasury", () => {
    expect(WORLD_DEFAULTS.startingTreasury).toBeGreaterThan(0);
  });

  it("starting treasury is $50,000 in cents", () => {
    expect(WORLD_DEFAULTS.startingTreasury).toBe(50_000_00);
  });

  it("maxEntities is 16384", () => {
    expect(WORLD_DEFAULTS.maxEntities).toBe(16384);
  });
});

// ─── TIME_DEFAULTS ───────────────────────────────────────────────────────────

describe("TIME_DEFAULTS", () => {
  it("starts at hour 8", () => {
    expect(TIME_DEFAULTS.startHour).toBe(8);
  });

  it("daylight starts at 6 and ends at 20", () => {
    expect(TIME_DEFAULTS.daylightStart).toBe(6);
    expect(TIME_DEFAULTS.daylightEnd).toBe(20);
  });
});
