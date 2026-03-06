import { describe, it, expect } from "vitest";
import {
  TAX_BRACKETS,
  WORKSPACE_DENSITY,
  GROWTH_MODIFIERS,
  DEPARTMENTS,
  getTaxBracket,
  getWorkspaceDensity,
  getDepartment,
  validateEconomyConfig,
} from "../economy.js";

describe("TAX_BRACKETS", () => {
  it("has 3 entries", () => {
    expect(TAX_BRACKETS).toHaveLength(3);
  });

  it("all brackets have valid rate ranges (min <= max)", () => {
    for (const bracket of TAX_BRACKETS) {
      expect(bracket.minRate).toBeLessThanOrEqual(bracket.maxRate);
    }
  });
});

describe("getTaxBracket", () => {
  it("finds bracket by category", () => {
    const res = getTaxBracket("residential");
    expect(res).toBeDefined();
    expect(res!.category).toBe("residential");
    expect(res!.defaultRate).toBe(0.09);
  });

  it("returns undefined for unknown category", () => {
    expect(getTaxBracket("agricultural")).toBeUndefined();
  });
});

describe("getWorkspaceDensity", () => {
  it("returns correct value for known archetype tag", () => {
    expect(getWorkspaceDensity("commercial")).toBe(20);
    expect(getWorkspaceDensity("industrial")).toBe(50);
  });

  it("returns default value (25) for unknown tag", () => {
    expect(getWorkspaceDensity("unknown_tag")).toBe(25);
  });
});

describe("DEPARTMENTS", () => {
  it("has 7 entries", () => {
    expect(DEPARTMENTS).toHaveLength(7);
  });
});

describe("getDepartment", () => {
  it("finds department by id", () => {
    const police = getDepartment("police");
    expect(police).toBeDefined();
    expect(police!.name).toBe("Police");
  });

  it("returns undefined for unknown id", () => {
    expect(getDepartment("unknown")).toBeUndefined();
  });
});

describe("WORKSPACE_DENSITY", () => {
  it("has entries for all expected archetype tags", () => {
    const tags = WORKSPACE_DENSITY.map((w) => w.archetypeTag);
    expect(tags).toContain("commercial");
    expect(tags).toContain("industrial");
    expect(tags).toContain("civic");
    expect(tags).toContain("education");
    expect(tags).toContain("health");
  });
});

describe("GROWTH_MODIFIERS", () => {
  it("has expected growth factors", () => {
    const factors = GROWTH_MODIFIERS.map((g) => g.factor);
    expect(factors).toContain("tax_rate");
    expect(factors).toContain("employment");
    expect(factors).toContain("services");
    expect(factors).toContain("pollution");
    expect(factors).toContain("crime");
  });
});

describe("validateEconomyConfig", () => {
  it("returns no errors for default configuration", () => {
    const errors = validateEconomyConfig();
    expect(errors).toHaveLength(0);
  });
});
