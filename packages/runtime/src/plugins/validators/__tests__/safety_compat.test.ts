import { describe, it, expect } from "vitest";
import {
  SafetyValidator,
  CompatibilityValidator,
  ValidationStage,
  ValidationSeverity,
} from "../index.js";

// ---- Test Helpers ----

/** Create a minimal manifest-like object for testing. */
function makeManifest(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: "test.plugin",
    name: "Test Plugin",
    version: "1.0.0",
    description: "A test plugin",
    author: "Test Author",
    dependencies: [],
    activation: "onWorldLoad",
    content_type: "buildings",
    data_path: "data/buildings.json",
    ...overrides,
  };
}

// ---- SafetyValidator ----

describe("SafetyValidator", () => {
  const validator = new SafetyValidator();

  it("passes for a clean manifest with no unsafe references", () => {
    const result = validator.validate(makeManifest());
    expect(result.valid).toBe(true);
    expect(result.stage).toBe(ValidationStage.Trust);
    expect(result.severity).toBe(ValidationSeverity.Info);
  });

  it("fails when manifest contains Math.random reference", () => {
    const result = validator.validate(
      makeManifest({ code: "const x = Math.random();" }),
    );
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
    expect(result.message).toContain("Math.random");
  });

  it("fails when manifest contains Date.now reference", () => {
    const result = validator.validate(
      makeManifest({ code: "const t = Date.now();" }),
    );
    expect(result.valid).toBe(false);
    expect(result.message).toContain("Date.now");
  });

  it("fails when manifest contains new Date reference", () => {
    const result = validator.validate(
      makeManifest({ init: "const d = new Date();" }),
    );
    expect(result.valid).toBe(false);
    expect(result.message).toContain("new Date");
  });

  it("detects unsafe references in nested objects", () => {
    const result = validator.validate(
      makeManifest({
        hooks: {
          onLoad: "return Math.random();",
        },
      }),
    );
    expect(result.valid).toBe(false);
    expect(result.message).toContain("Math.random");
  });

  it("detects unsafe references in arrays", () => {
    const result = validator.validate(
      makeManifest({
        scripts: ["safe_code()", "Date.now()"],
      }),
    );
    expect(result.valid).toBe(false);
    expect(result.message).toContain("Date.now");
  });

  it("reports multiple violations at once", () => {
    const result = validator.validate(
      makeManifest({
        code: "Math.random(); Date.now();",
      }),
    );
    expect(result.valid).toBe(false);
    expect(result.metadata?.violations).toContain("Math.random");
    expect(result.metadata?.violations).toContain("Date.now");
  });

  it("fails for null input", () => {
    const result = validator.validate(null);
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
  });

  it("passes when unsafe words appear in non-code context", () => {
    // "random" alone (without Math.) should not trigger
    const result = validator.validate(
      makeManifest({ description: "Uses random placement algorithms" }),
    );
    expect(result.valid).toBe(true);
  });
});

// ---- CompatibilityValidator ----

describe("CompatibilityValidator", () => {
  it("passes when no conflicts exist", () => {
    const validator = new CompatibilityValidator(["plugin.a"], ["buildings"]);
    const result = validator.validate(
      makeManifest({ id: "plugin.b", content_type: "networks" }),
    );
    expect(result.valid).toBe(true);
    expect(result.stage).toBe(ValidationStage.Dependency);
  });

  it("detects duplicate plugin ID", () => {
    const validator = new CompatibilityValidator(["test.plugin"], []);
    const result = validator.validate(makeManifest({ id: "test.plugin" }));
    expect(result.valid).toBe(false);
    expect(result.message).toContain("test.plugin");
    expect(result.message).toContain("already active");
  });

  it("detects exclusive content type conflict for v1 content_type", () => {
    const validator = new CompatibilityValidator(["plugin.a"], ["economy"]);
    const result = validator.validate(
      makeManifest({ id: "plugin.b", content_type: "economy" }),
    );
    expect(result.valid).toBe(false);
    expect(result.message).toContain("economy");
  });

  it("detects exclusive content type conflict for v2 contentTypes", () => {
    const validator = new CompatibilityValidator(["plugin.a"], ["terrain"]);
    const manifest = {
      id: "plugin.b",
      name: "Test",
      version: "1.0.0",
      manifestVersion: 2,
      contentTypes: ["terrain"],
    };
    const result = validator.validate(manifest);
    expect(result.valid).toBe(false);
    expect(result.message).toContain("terrain");
  });

  it("allows non-exclusive content type overlap", () => {
    const validator = new CompatibilityValidator(["plugin.a"], ["buildings"]);
    const result = validator.validate(
      makeManifest({ id: "plugin.b", content_type: "buildings" }),
    );
    expect(result.valid).toBe(true);
  });

  it("passes with empty active plugins", () => {
    const validator = new CompatibilityValidator([], []);
    const result = validator.validate(makeManifest());
    expect(result.valid).toBe(true);
  });

  it("fails for null input", () => {
    const validator = new CompatibilityValidator([], []);
    const result = validator.validate(null);
    expect(result.valid).toBe(false);
  });

  it("detects governance as exclusive content type", () => {
    const validator = new CompatibilityValidator(["plugin.a"], ["governance"]);
    const result = validator.validate(
      makeManifest({ id: "plugin.b", content_type: "governance" }),
    );
    expect(result.valid).toBe(false);
    expect(result.message).toContain("governance");
  });
});
