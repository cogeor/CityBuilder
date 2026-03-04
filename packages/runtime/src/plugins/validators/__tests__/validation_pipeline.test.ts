import { describe, it, expect } from "vitest";
import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
  SchemaValidator,
  DependencyValidator,
  EngineVersionValidator,
  TrustValidator,
  ValidationPipeline,
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

// ---- SchemaValidator ----

describe("SchemaValidator", () => {
  const validator = new SchemaValidator();

  it("passes for a valid manifest", () => {
    const result = validator.validate(makeManifest());
    expect(result.valid).toBe(true);
    expect(result.stage).toBe(ValidationStage.Schema);
  });

  it("fails when id is missing", () => {
    const manifest = makeManifest();
    delete manifest.id;
    const result = validator.validate(manifest);
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
    expect(result.message).toContain("id");
  });

  it("fails when id is empty string", () => {
    const result = validator.validate(makeManifest({ id: "" }));
    expect(result.valid).toBe(false);
    expect(result.message).toContain("id");
  });

  it("fails when version is missing", () => {
    const manifest = makeManifest();
    delete manifest.version;
    const result = validator.validate(manifest);
    expect(result.valid).toBe(false);
    expect(result.message).toContain("version");
  });

  it("fails when version doesn't match semver", () => {
    const result = validator.validate(makeManifest({ version: "abc" }));
    expect(result.valid).toBe(false);
    expect(result.message).toContain("semver");
  });

  it("passes version with pre-release tag", () => {
    const result = validator.validate(makeManifest({ version: "1.0.0-beta.1" }));
    expect(result.valid).toBe(true);
  });

  it("fails when name is missing", () => {
    const manifest = makeManifest();
    delete manifest.name;
    const result = validator.validate(manifest);
    expect(result.valid).toBe(false);
    expect(result.message).toContain("name");
  });

  it("fails when name is empty string", () => {
    const result = validator.validate(makeManifest({ name: "" }));
    expect(result.valid).toBe(false);
  });

  it("fails for null input", () => {
    const result = validator.validate(null);
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
  });

  it("fails for non-object input", () => {
    const result = validator.validate("string");
    expect(result.valid).toBe(false);
  });
});

// ---- DependencyValidator ----

describe("DependencyValidator", () => {
  it("passes when all dependencies are available", () => {
    const validator = new DependencyValidator(["base.core", "base.terrain"]);
    const result = validator.validate(
      makeManifest({ dependencies: ["base.core", "base.terrain"] }),
    );
    expect(result.valid).toBe(true);
    expect(result.stage).toBe(ValidationStage.Dependency);
  });

  it("fails when a dependency is missing", () => {
    const validator = new DependencyValidator(["base.core"]);
    const result = validator.validate(
      makeManifest({ dependencies: ["base.core", "missing.plugin"] }),
    );
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
    expect(result.message).toContain("missing.plugin");
  });

  it("passes with no dependencies declared", () => {
    const validator = new DependencyValidator(["base.core"]);
    const result = validator.validate(makeManifest({ dependencies: [] }));
    expect(result.valid).toBe(true);
  });

  it("passes when dependencies field is absent", () => {
    const validator = new DependencyValidator(["base.core"]);
    const manifest = makeManifest();
    delete manifest.dependencies;
    const result = validator.validate(manifest);
    expect(result.valid).toBe(true);
  });

  it("reports all missing dependencies", () => {
    const validator = new DependencyValidator([]);
    const result = validator.validate(
      makeManifest({ dependencies: ["dep.a", "dep.b"] }),
    );
    expect(result.valid).toBe(false);
    expect(result.metadata?.missing).toEqual(["dep.a", "dep.b"]);
  });
});

// ---- EngineVersionValidator ----

describe("EngineVersionValidator", () => {
  it("passes when major versions match", () => {
    const validator = new EngineVersionValidator("2.3.1");
    const result = validator.validate(makeManifest({ engine: "2.0.0" }));
    expect(result.valid).toBe(true);
    expect(result.stage).toBe(ValidationStage.EngineVersion);
  });

  it("fails when major versions differ", () => {
    const validator = new EngineVersionValidator("2.3.1");
    const result = validator.validate(makeManifest({ engine: "3.0.0" }));
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
    expect(result.message).toContain("mismatch");
  });

  it("passes when no engine field is declared", () => {
    const validator = new EngineVersionValidator("2.3.1");
    const result = validator.validate(makeManifest());
    expect(result.valid).toBe(true);
  });

  it("handles engine version with prefix operators", () => {
    const validator = new EngineVersionValidator("2.3.1");
    const result = validator.validate(makeManifest({ engine: ">=2.0.0" }));
    expect(result.valid).toBe(true);
  });

  it("fails for unparseable engine version", () => {
    const validator = new EngineVersionValidator("2.3.1");
    const result = validator.validate(makeManifest({ engine: "abc" }));
    expect(result.valid).toBe(false);
    expect(result.message).toContain("parse");
  });

  it("fails for empty engine string", () => {
    const validator = new EngineVersionValidator("2.3.1");
    const result = validator.validate(makeManifest({ engine: "" }));
    expect(result.valid).toBe(false);
  });
});

// ---- TrustValidator ----

describe("TrustValidator", () => {
  it("passes when author is in trusted list", () => {
    const validator = new TrustValidator(["Test Author", "Other Author"]);
    const result = validator.validate(makeManifest({ author: "Test Author" }));
    expect(result.valid).toBe(true);
    expect(result.stage).toBe(ValidationStage.Trust);
  });

  it("fails when author is not in trusted list", () => {
    const validator = new TrustValidator(["Trusted Corp"]);
    const result = validator.validate(makeManifest({ author: "Unknown Author" }));
    expect(result.valid).toBe(false);
    expect(result.severity).toBe(ValidationSeverity.Error);
    expect(result.message).toContain("Unknown Author");
  });

  it("trusts all when trusted list is empty", () => {
    const validator = new TrustValidator([]);
    const result = validator.validate(makeManifest({ author: "Anyone" }));
    expect(result.valid).toBe(true);
    expect(result.message).toContain("all sources trusted");
  });

  it("fails when author is missing and trusted list is non-empty", () => {
    const validator = new TrustValidator(["Trusted Corp"]);
    const manifest = makeManifest();
    delete manifest.author;
    const result = validator.validate(manifest);
    expect(result.valid).toBe(false);
  });

  it("fails for null input", () => {
    const validator = new TrustValidator(["x"]);
    const result = validator.validate(null);
    expect(result.valid).toBe(false);
  });
});

// ---- ValidationPipeline ----

describe("ValidationPipeline", () => {
  it("runs validators in order", () => {
    const pipeline = new ValidationPipeline();
    const order: string[] = [];

    const makeTrackingValidator = (stage: ValidationStage): IPluginValidator => ({
      stage,
      validate(): ValidationResult {
        order.push(stage);
        return {
          stage,
          valid: true,
          severity: ValidationSeverity.Info,
          message: "ok",
        };
      },
    });

    pipeline.addValidator(makeTrackingValidator(ValidationStage.Schema));
    pipeline.addValidator(makeTrackingValidator(ValidationStage.Dependency));
    pipeline.addValidator(makeTrackingValidator(ValidationStage.Trust));

    pipeline.validate(makeManifest());
    expect(order).toEqual([
      ValidationStage.Schema,
      ValidationStage.Dependency,
      ValidationStage.Trust,
    ]);
  });

  it("short-circuits on first error", () => {
    const pipeline = new ValidationPipeline();

    // First validator: passes
    pipeline.addValidator(new SchemaValidator());

    // Second validator: always fails with Error
    const failValidator: IPluginValidator = {
      stage: ValidationStage.Dependency,
      validate(): ValidationResult {
        return {
          stage: ValidationStage.Dependency,
          valid: false,
          severity: ValidationSeverity.Error,
          message: "forced failure",
        };
      },
    };
    pipeline.addValidator(failValidator);

    // Third validator: should not be reached
    let thirdCalled = false;
    const thirdValidator: IPluginValidator = {
      stage: ValidationStage.Trust,
      validate(): ValidationResult {
        thirdCalled = true;
        return {
          stage: ValidationStage.Trust,
          valid: true,
          severity: ValidationSeverity.Info,
          message: "should not run",
        };
      },
    };
    pipeline.addValidator(thirdValidator);

    const results = pipeline.validate(makeManifest());
    expect(results).toHaveLength(2);
    expect(results[1].valid).toBe(false);
    expect(thirdCalled).toBe(false);
  });

  it("validateAll runs all validators regardless of errors", () => {
    const pipeline = new ValidationPipeline();

    pipeline.addValidator(new SchemaValidator());

    const failValidator: IPluginValidator = {
      stage: ValidationStage.Dependency,
      validate(): ValidationResult {
        return {
          stage: ValidationStage.Dependency,
          valid: false,
          severity: ValidationSeverity.Error,
          message: "forced failure",
        };
      },
    };
    pipeline.addValidator(failValidator);

    let thirdCalled = false;
    const thirdValidator: IPluginValidator = {
      stage: ValidationStage.Trust,
      validate(): ValidationResult {
        thirdCalled = true;
        return {
          stage: ValidationStage.Trust,
          valid: true,
          severity: ValidationSeverity.Info,
          message: "ran anyway",
        };
      },
    };
    pipeline.addValidator(thirdValidator);

    const results = pipeline.validateAll(makeManifest());
    expect(results).toHaveLength(3);
    expect(thirdCalled).toBe(true);
  });

  it("returns empty array with no validators", () => {
    const pipeline = new ValidationPipeline();
    expect(pipeline.validate(makeManifest())).toEqual([]);
    expect(pipeline.validateAll(makeManifest())).toEqual([]);
  });

  it("getValidators returns a copy of the chain", () => {
    const pipeline = new ValidationPipeline();
    const sv = new SchemaValidator();
    pipeline.addValidator(sv);

    const validators = pipeline.getValidators();
    expect(validators).toHaveLength(1);
    expect(validators[0]).toBe(sv);

    // Mutating the returned array should not affect the pipeline
    validators.pop();
    expect(pipeline.getValidators()).toHaveLength(1);
  });

  it("does not short-circuit on warning", () => {
    const pipeline = new ValidationPipeline();

    const warnValidator: IPluginValidator = {
      stage: ValidationStage.Schema,
      validate(): ValidationResult {
        return {
          stage: ValidationStage.Schema,
          valid: false,
          severity: ValidationSeverity.Warning,
          message: "just a warning",
        };
      },
    };
    pipeline.addValidator(warnValidator);

    let secondCalled = false;
    const secondValidator: IPluginValidator = {
      stage: ValidationStage.Dependency,
      validate(): ValidationResult {
        secondCalled = true;
        return {
          stage: ValidationStage.Dependency,
          valid: true,
          severity: ValidationSeverity.Info,
          message: "ok",
        };
      },
    };
    pipeline.addValidator(secondValidator);

    const results = pipeline.validate(makeManifest());
    expect(results).toHaveLength(2);
    expect(secondCalled).toBe(true);
  });

  it("full pipeline with real validators passes valid manifest", () => {
    const pipeline = new ValidationPipeline();
    pipeline.addValidator(new SchemaValidator());
    pipeline.addValidator(new DependencyValidator(["base.core"]));
    pipeline.addValidator(new EngineVersionValidator("1.0.0"));
    pipeline.addValidator(new TrustValidator(["Test Author"]));

    const manifest = makeManifest({
      dependencies: ["base.core"],
      engine: "1.0.0",
      author: "Test Author",
    });

    const results = pipeline.validate(manifest);
    expect(results.every((r) => r.valid)).toBe(true);
    expect(results).toHaveLength(4);
  });
});
