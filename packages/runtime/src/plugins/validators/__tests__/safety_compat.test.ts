import { describe, expect, it } from "vitest";
import {
  CompatibilityValidator,
  SafetyValidator,
  ValidationSeverity,
} from "../index.js";

function makeManifest(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: "plugin.test",
    name: "Plugin Test",
    version: "1.0.0",
    author: "Author",
    dependencies: [],
    activation: "onWorldLoad",
    runtime: "data",
    contributes: { buildings: ["data/buildings.json"] },
    ...overrides,
  };
}

describe("SafetyValidator", () => {
  it("passes clean manifests", () => {
    const result = new SafetyValidator().validate(makeManifest());
    expect(result.valid).toBe(true);
  });

  it("flags non-deterministic references", () => {
    const result = new SafetyValidator().validate(
      makeManifest({ source: "new Date(); Math.random();" }),
    );
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
    expect(result.message).toContain("Math.random");
  });
});

describe("CompatibilityValidator", () => {
  it("passes when there are no collisions", () => {
    const validator = new CompatibilityValidator(["plugin.base"], ["terrain"]);
    const result = validator.validate(makeManifest({ id: "plugin.extra" }));
    expect(result.valid).toBe(true);
  });

  it("rejects duplicate id", () => {
    const validator = new CompatibilityValidator(["plugin.test"], []);
    const result = validator.validate(makeManifest());
    expect(result.valid).toBe(false);
    expect(result.message).toContain("already active");
  });

  it("rejects exclusive type conflicts", () => {
    const validator = new CompatibilityValidator(["plugin.base"], ["economy"]);
    const result = validator.validate(makeManifest({ contributes: { economy: ["data/economy.json"] } }));
    expect(result.valid).toBe(false);
    expect(result.message).toContain("exclusive");
  });
});
