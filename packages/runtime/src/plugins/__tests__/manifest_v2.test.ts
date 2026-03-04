import { describe, it, expect } from "vitest";
import {
  ContentType,
  type ManifestV1,
  type ManifestV2,
  isManifestV2,
  migrateV1toV2,
  validateManifestV2,
} from "../index.js";

// ---- Test Helpers ----

/** Create a minimal valid ManifestV1. */
function makeV1(overrides: Partial<ManifestV1> = {}): ManifestV1 {
  return {
    id: "test.plugin",
    name: "Test Plugin",
    version: "1.0.0",
    dependencies: [],
    ...overrides,
  };
}

/** Create a minimal valid ManifestV2. */
function makeV2(overrides: Partial<ManifestV2> = {}): ManifestV2 {
  return {
    id: "test.plugin",
    name: "Test Plugin",
    version: "1.0.0",
    dependencies: [],
    manifestVersion: 2,
    contentTypes: [ContentType.Buildings],
    ...overrides,
  };
}

// ---- ContentType enum ----

describe("ContentType", () => {
  it("has all 11 content types", () => {
    const values = Object.values(ContentType);
    expect(values).toHaveLength(11);
  });

  it("includes all expected content type values", () => {
    expect(ContentType.Buildings).toBe("buildings");
    expect(ContentType.Terrain).toBe("terrain");
    expect(ContentType.Networks).toBe("networks");
    expect(ContentType.Economy).toBe("economy");
    expect(ContentType.Progression).toBe("progression");
    expect(ContentType.Demographics).toBe("demographics");
    expect(ContentType.Events).toBe("events");
    expect(ContentType.Scenarios).toBe("scenarios");
    expect(ContentType.Governance).toBe("governance");
    expect(ContentType.AI).toBe("ai");
    expect(ContentType.Tools).toBe("tools");
  });

  it("enum values are unique strings", () => {
    const values = Object.values(ContentType);
    const unique = new Set(values);
    expect(unique.size).toBe(values.length);
  });
});

// ---- isManifestV2 ----

describe("isManifestV2", () => {
  it("returns true for a valid ManifestV2", () => {
    const v2 = makeV2();
    expect(isManifestV2(v2)).toBe(true);
  });

  it("returns false for a ManifestV1 without manifestVersion", () => {
    const v1 = makeV1();
    expect(isManifestV2(v1)).toBe(false);
  });

  it("returns false when manifestVersion is not 2", () => {
    const manifest = { ...makeV2(), manifestVersion: 1 as any };
    expect(isManifestV2(manifest)).toBe(false);
  });

  it("returns false when contentTypes is missing", () => {
    const manifest = { ...makeV1(), manifestVersion: 2 };
    expect(isManifestV2(manifest as any)).toBe(false);
  });

  it("returns false when contentTypes is not an array", () => {
    const manifest = { ...makeV2(), contentTypes: "buildings" as any };
    expect(isManifestV2(manifest as any)).toBe(false);
  });
});

// ---- migrateV1toV2 ----

describe("migrateV1toV2", () => {
  it("sets manifestVersion to 2", () => {
    const result = migrateV1toV2(makeV1());
    expect(result.manifestVersion).toBe(2);
  });

  it("preserves id, name, and version from v1", () => {
    const v1 = makeV1({ id: "my.plugin", name: "My Plugin", version: "2.5.0" });
    const result = migrateV1toV2(v1);
    expect(result.id).toBe("my.plugin");
    expect(result.name).toBe("My Plugin");
    expect(result.version).toBe("2.5.0");
  });

  it("defaults contentTypes to [Buildings]", () => {
    const result = migrateV1toV2(makeV1());
    expect(result.contentTypes).toEqual([ContentType.Buildings]);
  });

  it("preserves existing dependencies", () => {
    const v1 = makeV1({ dependencies: ["dep.a", "dep.b"] });
    const result = migrateV1toV2(v1);
    expect(result.dependencies).toEqual(["dep.a", "dep.b"]);
  });

  it("defaults dependencies to empty array when undefined", () => {
    const v1 = makeV1();
    delete (v1 as any).dependencies;
    const result = migrateV1toV2(v1);
    expect(result.dependencies).toEqual([]);
  });

  it("result passes isManifestV2 type guard", () => {
    const result = migrateV1toV2(makeV1());
    expect(isManifestV2(result)).toBe(true);
  });
});

// ---- validateManifestV2 ----

describe("validateManifestV2", () => {
  it("passes a valid ManifestV2", () => {
    const result = validateManifestV2(makeV2());
    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it("fails for null input", () => {
    const result = validateManifestV2(null);
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it("fails for undefined input", () => {
    const result = validateManifestV2(undefined);
    expect(result.valid).toBe(false);
  });

  it("fails for non-object input", () => {
    const result = validateManifestV2("string");
    expect(result.valid).toBe(false);
  });

  it("fails when id is missing", () => {
    const manifest = makeV2();
    delete (manifest as any).id;
    const result = validateManifestV2(manifest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("id"))).toBe(true);
  });

  it("fails when id is empty", () => {
    const result = validateManifestV2(makeV2({ id: "" }));
    expect(result.valid).toBe(false);
  });

  it("fails when name is missing", () => {
    const manifest = makeV2();
    delete (manifest as any).name;
    const result = validateManifestV2(manifest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("name"))).toBe(true);
  });

  it("fails when version is not valid semver", () => {
    const result = validateManifestV2(makeV2({ version: "abc" }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("version"))).toBe(true);
  });

  it("passes version with pre-release tag", () => {
    const result = validateManifestV2(makeV2({ version: "1.0.0-beta.1" }));
    expect(result.valid).toBe(true);
  });

  it("fails when manifestVersion is not 2", () => {
    const manifest = { ...makeV2(), manifestVersion: 1 };
    const result = validateManifestV2(manifest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("manifestVersion"))).toBe(true);
  });

  it("fails when contentTypes is not an array", () => {
    const manifest = { ...makeV2(), contentTypes: "buildings" };
    const result = validateManifestV2(manifest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("contentTypes"))).toBe(true);
  });

  it("fails when contentTypes is empty", () => {
    const result = validateManifestV2(makeV2({ contentTypes: [] }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("contentTypes"))).toBe(true);
  });

  it("fails when contentTypes contains invalid type", () => {
    const manifest = { ...makeV2(), contentTypes: ["invalid_type"] };
    const result = validateManifestV2(manifest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("invalid_type"))).toBe(true);
  });

  it("passes with all valid optional fields", () => {
    const result = validateManifestV2(
      makeV2({
        author: "Test Author",
        description: "A test plugin",
        engineVersion: "2.0.0",
        dependencies: ["base.core"],
      }),
    );
    expect(result.valid).toBe(true);
  });

  it("fails when dependencies contains non-string", () => {
    const manifest = { ...makeV2(), dependencies: [42] };
    const result = validateManifestV2(manifest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("dependency"))).toBe(true);
  });

  it("collects multiple errors at once", () => {
    const result = validateManifestV2({
      id: "",
      name: "",
      version: "bad",
      manifestVersion: 1,
      contentTypes: [],
    });
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThanOrEqual(4);
  });
});
