import { describe, expect, it } from "vitest";
import {
  CompatibilityValidator,
  DependencyValidator,
  EngineVersionValidator,
  SafetyValidator,
  SchemaValidator,
  TrustValidator,
  ValidationPipeline,
  ValidationSeverity,
} from "../index.js";

function makeManifest(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: "test.plugin",
    name: "Test Plugin",
    version: "1.0.0",
    description: "test",
    author: "TownBuilder",
    dependencies: [],
    activation: "onWorldLoad",
    runtime: "data",
    contributes: { buildings: ["data/buildings.json"] },
    ...overrides,
  };
}

describe("validators", () => {
  it("SchemaValidator accepts canonical manifest", () => {
    const r = new SchemaValidator().validate(makeManifest());
    expect(r.valid).toBe(true);
  });

  it("SchemaValidator rejects invalid semver", () => {
    const r = new SchemaValidator().validate(makeManifest({ version: "latest" }));
    expect(r.valid).toBe(false);
    expect(r.severity).toBe(ValidationSeverity.Error);
  });

  it("DependencyValidator finds missing deps", () => {
    const r = new DependencyValidator(["base.world"]).validate(
      makeManifest({ dependencies: ["base.world", "base.economy"] }),
    );
    expect(r.valid).toBe(false);
    expect(r.message).toContain("base.economy");
  });

  it("EngineVersionValidator reads engine_compat", () => {
    const r = new EngineVersionValidator("1.2.0").validate(
      makeManifest({ engine_compat: ">=1.0.0" }),
    );
    expect(r.valid).toBe(true);
  });

  it("TrustValidator enforces author when trust list is set", () => {
    const validator = new TrustValidator(["Trusted Studio"]);
    const r = validator.validate(makeManifest({ author: "Unknown" }));
    expect(r.valid).toBe(false);
  });

  it("SafetyValidator rejects non-deterministic snippets", () => {
    const r = new SafetyValidator().validate(
      makeManifest({ notes: "uses Math.random for generation" }),
    );
    expect(r.valid).toBe(false);
    expect(r.message).toContain("Math.random");
  });

  it("CompatibilityValidator checks duplicate ids and exclusive types", () => {
    const validator = new CompatibilityValidator(["test.plugin"], ["economy"]);
    const r = validator.validate(
      makeManifest({ contributes: { economy: ["data/economy.json"] } }),
    );
    expect(r.valid).toBe(false);
    expect(r.message).toContain("already active");
    expect(r.message).toContain("exclusive");
  });
});

describe("ValidationPipeline", () => {
  it("short-circuits on first error in validate", () => {
    const pipeline = new ValidationPipeline([
      new SchemaValidator(),
      new SafetyValidator(),
      new TrustValidator(["TownBuilder"]),
    ]);

    const results = pipeline.validate(makeManifest({ version: "bad" }));
    expect(results).toHaveLength(1);
    expect(results[0].valid).toBe(false);
  });

  it("runs all validators in validateAll", () => {
    const pipeline = new ValidationPipeline()
      .addValidator(new SchemaValidator())
      .addValidator(new SafetyValidator())
      .addValidator(new TrustValidator(["TownBuilder"]));

    const results = pipeline.validateAll(makeManifest({ script: "Date.now()" }));
    expect(results).toHaveLength(3);
    expect(results.some((r) => r.valid === false)).toBe(true);
  });
});
