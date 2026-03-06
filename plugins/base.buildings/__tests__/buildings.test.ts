import { describe, it, expect } from "vitest";
import {
  BASE_BUILDINGS,
  getArchetypeById,
  getArchetypesByTag,
  validateArchetype,
  type ArchetypeDefinition,
} from "../index.js";

describe("BASE_BUILDINGS", () => {
  it("has 5 entries", () => {
    expect(BASE_BUILDINGS).toHaveLength(5);
  });

  it("all archetypes have unique IDs", () => {
    const ids = BASE_BUILDINGS.map((a) => a.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length);
  });
});

describe("archetype properties", () => {
  it("small house has correct properties", () => {
    const house = getArchetypeById(100);
    expect(house).toBeDefined();
    expect(house!.name).toBe("Small House");
    expect(house!.tags).toContain("residential");
    expect(house!.footprint).toEqual({ w: 1, h: 1 });
    expect(house!.baseCost).toBe(15000);
    expect(house!.livingSpacePerPerson).toBe(40);
    expect(house!.workspacePerJob).toBe(0);
    expect(house!.floors).toBe(2);
    expect(house!.maxLevel).toBe(3);
  });

  it("power plant supplies power", () => {
    const plant = getArchetypeById(200);
    expect(plant).toBeDefined();
    expect(plant!.name).toBe("Coal Power Plant");
    expect(plant!.powerSupply).toBe(500);
    expect(plant!.powerDemand).toBe(0);
    expect(plant!.tags).toContain("utility");
    expect(plant!.tags).toContain("power");
    expect(plant!.pollution).toBe(30);
  });

  it("hospital has health tag", () => {
    const hosp = getArchetypeById(300);
    expect(hosp).toBeDefined();
    expect(hosp!.name).toBe("Hospital");
    expect(hosp!.tags).toContain("health");
    expect(hosp!.tags).toContain("civic");
    expect(hosp!.serviceRadius).toBe(30);
  });

  it("hospital footprint is 3x3 — regression guard against 5x5 divergence", () => {
    // Canonical footprint from SimCity tiles 409-417 is 3x3.
    // This test prevents Rust (3x3) vs TypeScript (5x5) drift from reappearing.
    const hosp = getArchetypeById(300);
    expect(hosp).toBeDefined();
    expect(hosp!.footprint).toEqual({ w: 3, h: 3 });
  });

  it("shop has commercial tag", () => {
    const s = getArchetypeById(400);
    expect(s).toBeDefined();
    expect(s!.name).toBe("Corner Shop");
    expect(s!.tags).toContain("commercial");
    expect(s!.footprint).toEqual({ w: 1, h: 2 });
  });

  it("school has education tag", () => {
    const sch = getArchetypeById(500);
    expect(sch).toBeDefined();
    expect(sch!.name).toBe("Elementary School");
    expect(sch!.tags).toContain("education");
    expect(sch!.tags).toContain("civic");
    expect(sch!.serviceRadius).toBe(20);
  });
});

describe("getArchetypeById", () => {
  it("finds archetype by id", () => {
    const result = getArchetypeById(100);
    expect(result).toBeDefined();
    expect(result!.id).toBe(100);
    expect(result!.name).toBe("Small House");
  });

  it("returns undefined for unknown id", () => {
    const result = getArchetypeById(999);
    expect(result).toBeUndefined();
  });
});

describe("getArchetypesByTag", () => {
  it("returns residential buildings", () => {
    const residential = getArchetypesByTag("residential");
    expect(residential).toHaveLength(1);
    expect(residential[0].name).toBe("Small House");
  });

  it("returns civic buildings", () => {
    const civic = getArchetypesByTag("civic");
    expect(civic).toHaveLength(2);
    const names = civic.map((a) => a.name).sort();
    expect(names).toEqual(["Elementary School", "Hospital"]);
  });

  it("returns commercial buildings", () => {
    const commercial = getArchetypesByTag("commercial");
    expect(commercial).toHaveLength(1);
    expect(commercial[0].name).toBe("Corner Shop");
  });

  it("returns empty array for unused tag", () => {
    const industrial = getArchetypesByTag("industrial");
    expect(industrial).toHaveLength(0);
  });
});

describe("validateArchetype", () => {
  it("returns no errors for valid archetype", () => {
    const house = getArchetypeById(100)!;
    const errors = validateArchetype(house);
    expect(errors).toHaveLength(0);
  });

  it("returns no errors for all base buildings", () => {
    for (const building of BASE_BUILDINGS) {
      const errors = validateArchetype(building);
      expect(errors).toHaveLength(0);
    }
  });

  it("catches invalid id", () => {
    const invalid: ArchetypeDefinition = {
      ...getArchetypeById(100)!,
      id: -1,
    };
    const errors = validateArchetype(invalid);
    expect(errors).toContain("Invalid id");
  });

  it("catches missing name", () => {
    const invalid: ArchetypeDefinition = {
      ...getArchetypeById(100)!,
      name: "",
    };
    const errors = validateArchetype(invalid);
    expect(errors).toContain("Missing name");
  });

  it("catches invalid footprint", () => {
    const invalid: ArchetypeDefinition = {
      ...getArchetypeById(100)!,
      footprint: { w: 0, h: 1 },
    };
    const errors = validateArchetype(invalid);
    expect(errors).toContain("Invalid footprint");
  });

  it("catches negative base cost", () => {
    const invalid: ArchetypeDefinition = {
      ...getArchetypeById(100)!,
      baseCost: -100,
    };
    const errors = validateArchetype(invalid);
    expect(errors).toContain("Negative base cost");
  });

  it("catches invalid coverage ratio", () => {
    const invalid: ArchetypeDefinition = {
      ...getArchetypeById(100)!,
      coverageRatio: 0,
    };
    const errors = validateArchetype(invalid);
    expect(errors).toContain("Coverage ratio must be (0, 1]");
  });

  it("catches multiple errors at once", () => {
    const invalid: ArchetypeDefinition = {
      ...getArchetypeById(100)!,
      id: 0,
      name: "",
      baseCost: -1,
    };
    const errors = validateArchetype(invalid);
    expect(errors.length).toBeGreaterThanOrEqual(3);
    expect(errors).toContain("Invalid id");
    expect(errors).toContain("Missing name");
    expect(errors).toContain("Negative base cost");
  });
});
