import { describe, it, expect } from "vitest";
import {
  validateManifest,
  PluginRegistry,
  resolveDependencies,
  validateDependencies,
  type PluginManifest,
} from "../index.js";

// ---- Test Helpers ----

/** Create a minimal valid manifest, optionally overriding fields. */
function makeManifest(overrides: Partial<PluginManifest> = {}): PluginManifest {
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

// ---- validateManifest ----

describe("validateManifest", () => {
  it("accepts a valid manifest", () => {
    const manifest = makeManifest();
    expect(validateManifest(manifest)).toBe(true);
  });

  it("accepts all valid content types", () => {
    const types = ["buildings", "terrain", "economy", "networks", "world"] as const;
    for (const content_type of types) {
      expect(validateManifest(makeManifest({ content_type }))).toBe(true);
    }
  });

  it("accepts both activation modes", () => {
    expect(validateManifest(makeManifest({ activation: "onWorldLoad" }))).toBe(true);
    expect(validateManifest(makeManifest({ activation: "onDemand" }))).toBe(true);
  });

  it("rejects null and undefined", () => {
    expect(validateManifest(null)).toBe(false);
    expect(validateManifest(undefined)).toBe(false);
  });

  it("rejects non-object values", () => {
    expect(validateManifest("string")).toBe(false);
    expect(validateManifest(42)).toBe(false);
    expect(validateManifest(true)).toBe(false);
  });

  it("rejects manifest with missing id", () => {
    const { id: _, ...rest } = makeManifest();
    expect(validateManifest(rest)).toBe(false);
  });

  it("rejects manifest with empty id", () => {
    expect(validateManifest(makeManifest({ id: "" }))).toBe(false);
  });

  it("rejects manifest with missing name", () => {
    const { name: _, ...rest } = makeManifest();
    expect(validateManifest(rest)).toBe(false);
  });

  it("rejects manifest with missing version", () => {
    const { version: _, ...rest } = makeManifest();
    expect(validateManifest(rest)).toBe(false);
  });

  it("rejects manifest with invalid content_type", () => {
    expect(
      validateManifest(makeManifest({ content_type: "invalid" as any })),
    ).toBe(false);
  });

  it("rejects manifest with invalid activation", () => {
    expect(
      validateManifest(makeManifest({ activation: "always" as any })),
    ).toBe(false);
  });

  it("rejects manifest with non-array dependencies", () => {
    expect(
      validateManifest(makeManifest({ dependencies: "dep" as any })),
    ).toBe(false);
  });

  it("rejects manifest with non-string dependency entries", () => {
    expect(
      validateManifest(makeManifest({ dependencies: [42] as any })),
    ).toBe(false);
  });
});

// ---- PluginRegistry ----

describe("PluginRegistry", () => {
  it("registers and retrieves a plugin", () => {
    const registry = new PluginRegistry();
    const manifest = makeManifest({ id: "base.buildings" });

    expect(registry.register(manifest)).toBe(true);

    const entry = registry.get("base.buildings");
    expect(entry).toBeDefined();
    expect(entry!.manifest.id).toBe("base.buildings");
    expect(entry!.loaded).toBe(false);
    expect(entry!.data).toBeNull();
  });

  it("returns undefined for unregistered plugin", () => {
    const registry = new PluginRegistry();
    expect(registry.get("nonexistent")).toBeUndefined();
  });

  it("rejects duplicate registration", () => {
    const registry = new PluginRegistry();
    const manifest = makeManifest({ id: "base.buildings" });

    expect(registry.register(manifest)).toBe(true);
    expect(registry.register(manifest)).toBe(false);
  });

  it("lists all registered plugin manifests", () => {
    const registry = new PluginRegistry();
    registry.register(makeManifest({ id: "a", content_type: "buildings" }));
    registry.register(makeManifest({ id: "b", content_type: "terrain" }));
    registry.register(makeManifest({ id: "c", content_type: "economy" }));

    const all = registry.list();
    expect(all).toHaveLength(3);

    const ids = all.map((m) => m.id).sort();
    expect(ids).toEqual(["a", "b", "c"]);
  });

  it("lists plugins filtered by content type", () => {
    const registry = new PluginRegistry();
    registry.register(makeManifest({ id: "a", content_type: "buildings" }));
    registry.register(makeManifest({ id: "b", content_type: "terrain" }));
    registry.register(makeManifest({ id: "c", content_type: "buildings" }));
    registry.register(makeManifest({ id: "d", content_type: "economy" }));

    const buildings = registry.listByType("buildings");
    expect(buildings).toHaveLength(2);
    expect(buildings.map((m) => m.id).sort()).toEqual(["a", "c"]);

    const terrain = registry.listByType("terrain");
    expect(terrain).toHaveLength(1);
    expect(terrain[0].id).toBe("b");

    const networks = registry.listByType("networks");
    expect(networks).toHaveLength(0);
  });

  it("unregisters a plugin", () => {
    const registry = new PluginRegistry();
    registry.register(makeManifest({ id: "removeme" }));

    expect(registry.count()).toBe(1);
    expect(registry.unregister("removeme")).toBe(true);
    expect(registry.count()).toBe(0);
    expect(registry.get("removeme")).toBeUndefined();
  });

  it("returns false when unregistering nonexistent plugin", () => {
    const registry = new PluginRegistry();
    expect(registry.unregister("ghost")).toBe(false);
  });

  it("tracks loaded state via isLoaded", () => {
    const registry = new PluginRegistry();
    registry.register(makeManifest({ id: "plugin.a" }));

    expect(registry.isLoaded("plugin.a")).toBe(false);
    expect(registry.isLoaded("nonexistent")).toBe(false);

    // Simulate loading
    const entry = registry.get("plugin.a")!;
    entry.loaded = true;
    expect(registry.isLoaded("plugin.a")).toBe(true);
  });

  it("count tracks registered plugins accurately", () => {
    const registry = new PluginRegistry();
    expect(registry.count()).toBe(0);

    registry.register(makeManifest({ id: "a" }));
    expect(registry.count()).toBe(1);

    registry.register(makeManifest({ id: "b" }));
    expect(registry.count()).toBe(2);

    registry.unregister("a");
    expect(registry.count()).toBe(1);
  });
});

// ---- resolveDependencies ----

describe("resolveDependencies", () => {
  it("returns single manifest with no dependencies", () => {
    const m = makeManifest({ id: "solo" });
    const sorted = resolveDependencies([m]);
    expect(sorted).toHaveLength(1);
    expect(sorted[0].id).toBe("solo");
  });

  it("orders dependencies before dependents", () => {
    const core = makeManifest({ id: "core", dependencies: [] });
    const ext = makeManifest({ id: "ext", dependencies: ["core"] });

    // Pass in reverse order to prove sorting works
    const sorted = resolveDependencies([ext, core]);
    const ids = sorted.map((m) => m.id);
    expect(ids.indexOf("core")).toBeLessThan(ids.indexOf("ext"));
  });

  it("handles multi-level dependency chains", () => {
    const a = makeManifest({ id: "a", dependencies: [] });
    const b = makeManifest({ id: "b", dependencies: ["a"] });
    const c = makeManifest({ id: "c", dependencies: ["b"] });

    const sorted = resolveDependencies([c, b, a]);
    const ids = sorted.map((m) => m.id);
    expect(ids.indexOf("a")).toBeLessThan(ids.indexOf("b"));
    expect(ids.indexOf("b")).toBeLessThan(ids.indexOf("c"));
  });

  it("handles diamond dependencies", () => {
    const base = makeManifest({ id: "base", dependencies: [] });
    const left = makeManifest({ id: "left", dependencies: ["base"] });
    const right = makeManifest({ id: "right", dependencies: ["base"] });
    const top = makeManifest({ id: "top", dependencies: ["left", "right"] });

    const sorted = resolveDependencies([top, right, left, base]);
    const ids = sorted.map((m) => m.id);

    // base must come before left and right
    expect(ids.indexOf("base")).toBeLessThan(ids.indexOf("left"));
    expect(ids.indexOf("base")).toBeLessThan(ids.indexOf("right"));
    // left and right must come before top
    expect(ids.indexOf("left")).toBeLessThan(ids.indexOf("top"));
    expect(ids.indexOf("right")).toBeLessThan(ids.indexOf("top"));
  });

  it("throws on circular dependency", () => {
    const a = makeManifest({ id: "a", dependencies: ["b"] });
    const b = makeManifest({ id: "b", dependencies: ["a"] });

    expect(() => resolveDependencies([a, b])).toThrow(/[Cc]ircular/);
  });

  it("throws on self-referencing dependency", () => {
    const self = makeManifest({ id: "self", dependencies: ["self"] });
    expect(() => resolveDependencies([self])).toThrow(/[Cc]ircular/);
  });

  it("handles empty input", () => {
    expect(resolveDependencies([])).toEqual([]);
  });
});

// ---- validateDependencies ----

describe("validateDependencies", () => {
  it("returns empty array when all dependencies are satisfied", () => {
    const core = makeManifest({ id: "core", dependencies: [] });
    const ext = makeManifest({ id: "ext", dependencies: ["core"] });

    expect(validateDependencies([core, ext])).toEqual([]);
  });

  it("detects missing dependencies", () => {
    const ext = makeManifest({ id: "ext", dependencies: ["missing.dep"] });

    const missing = validateDependencies([ext]);
    expect(missing).toContain("missing.dep");
  });

  it("returns unique missing IDs", () => {
    const a = makeManifest({ id: "a", dependencies: ["shared"] });
    const b = makeManifest({ id: "b", dependencies: ["shared"] });

    const missing = validateDependencies([a, b]);
    expect(missing).toEqual(["shared"]);
  });

  it("handles manifests with no dependencies", () => {
    const a = makeManifest({ id: "a", dependencies: [] });
    const b = makeManifest({ id: "b", dependencies: [] });

    expect(validateDependencies([a, b])).toEqual([]);
  });

  it("reports multiple distinct missing dependencies", () => {
    const ext = makeManifest({ id: "ext", dependencies: ["dep1", "dep2"] });

    const missing = validateDependencies([ext]);
    expect(missing).toHaveLength(2);
    expect(missing).toContain("dep1");
    expect(missing).toContain("dep2");
  });
});
