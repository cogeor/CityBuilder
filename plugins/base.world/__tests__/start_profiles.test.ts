import { describe, it, expect } from "vitest";
import {
  EraPreset,
  START_PROFILES,
  getProfileById,
  getProfilesForMapSize,
  getProfilesForEra,
  validateProfileCompatibility,
} from "../start_profiles.js";

// ─── START_PROFILES ─────────────────────────────────────────────────────────

describe("START_PROFILES", () => {
  it("has 3 entries", () => {
    expect(START_PROFILES).toHaveLength(3);
  });

  it("all profiles have unique ids", () => {
    const ids = START_PROFILES.map(p => p.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("all profiles have non-empty name and description", () => {
    for (const p of START_PROFILES) {
      expect(p.name.length).toBeGreaterThan(0);
      expect(p.description.length).toBeGreaterThan(0);
    }
  });

  it("all profiles have positive startingTreasury", () => {
    for (const p of START_PROFILES) {
      expect(p.startingTreasury).toBeGreaterThan(0);
    }
  });

  it("all profiles have at least one compatible map size", () => {
    for (const p of START_PROFILES) {
      expect(p.compatibleMapSizes.length).toBeGreaterThan(0);
    }
  });

  it("all profiles have valid era values", () => {
    const validEras = Object.values(EraPreset);
    for (const p of START_PROFILES) {
      expect(validEras).toContain(p.era);
    }
  });
});

// ─── getProfileById ─────────────────────────────────────────────────────────

describe("getProfileById", () => {
  it("finds existing profile", () => {
    const profile = getProfileById("modern_standard");
    expect(profile).toBeDefined();
    expect(profile!.name).toBe("Modern City");
    expect(profile!.era).toBe(EraPreset.Modern);
  });

  it("returns undefined for unknown id", () => {
    expect(getProfileById("nonexistent")).toBeUndefined();
  });

  it("finds post-war profile", () => {
    const profile = getProfileById("post_war_rebuild");
    expect(profile).toBeDefined();
    expect(profile!.startingTreasury).toBe(25_000_00);
    expect(profile!.startingPopulation).toBe(500);
  });
});

// ─── getProfilesForMapSize ──────────────────────────────────────────────────

describe("getProfilesForMapSize", () => {
  it("filters correctly for small — all profiles support small", () => {
    const profiles = getProfilesForMapSize("small");
    expect(profiles).toHaveLength(3);
  });

  it("filters correctly for medium", () => {
    const profiles = getProfilesForMapSize("medium");
    expect(profiles).toHaveLength(2);
    const ids = profiles.map(p => p.id);
    expect(ids).toContain("modern_standard");
    expect(ids).toContain("post_war_rebuild");
  });

  it("filters correctly for large", () => {
    const profiles = getProfilesForMapSize("large");
    expect(profiles).toHaveLength(1);
    expect(profiles[0].id).toBe("modern_standard");
  });

  it("returns empty array for unknown map size", () => {
    const profiles = getProfilesForMapSize("tiny");
    expect(profiles).toHaveLength(0);
  });
});

// ─── getProfilesForEra ──────────────────────────────────────────────────────

describe("getProfilesForEra", () => {
  it("filters correctly for Modern era", () => {
    const profiles = getProfilesForEra(EraPreset.Modern);
    expect(profiles).toHaveLength(1);
    expect(profiles[0].id).toBe("modern_standard");
  });

  it("filters correctly for PostWar era", () => {
    const profiles = getProfilesForEra(EraPreset.PostWar);
    expect(profiles).toHaveLength(1);
    expect(profiles[0].id).toBe("post_war_rebuild");
  });

  it("filters correctly for Victorian era", () => {
    const profiles = getProfilesForEra(EraPreset.Victorian);
    expect(profiles).toHaveLength(1);
    expect(profiles[0].id).toBe("victorian_era");
  });
});

// ─── validateProfileCompatibility ───────────────────────────────────────────

describe("validateProfileCompatibility", () => {
  it("returns true for compatible map size", () => {
    const profile = getProfileById("modern_standard")!;
    expect(validateProfileCompatibility(profile, "small")).toBe(true);
    expect(validateProfileCompatibility(profile, "medium")).toBe(true);
    expect(validateProfileCompatibility(profile, "large")).toBe(true);
  });

  it("returns false for incompatible map size", () => {
    const profile = getProfileById("victorian_era")!;
    expect(validateProfileCompatibility(profile, "large")).toBe(false);
    expect(validateProfileCompatibility(profile, "medium")).toBe(false);
  });

  it("returns false for unknown map size", () => {
    const profile = getProfileById("modern_standard")!;
    expect(validateProfileCompatibility(profile, "enormous")).toBe(false);
  });
});
