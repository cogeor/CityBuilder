import { describe, it, expect } from "vitest";
import {
  PluginPrecedence,
  LoadPolicyManager,
  comparePrec,
  type PluginLoadEntry,
} from "../index.js";

// ---- Test Helpers ----

/** Create a minimal PluginLoadEntry with optional overrides. */
function makeEntry(overrides: Partial<PluginLoadEntry> = {}): PluginLoadEntry {
  return {
    id: "plugin.test",
    precedence: PluginPrecedence.Base,
    version: "1.0.0",
    dependencies: [],
    ...overrides,
  };
}

// ---- resolveLoadOrder ----

describe("LoadPolicyManager.resolveLoadOrder", () => {
  it("resolves single plugin with no dependencies", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "solo" }));

    const order = manager.resolveLoadOrder();
    expect(order).toHaveLength(1);
    expect(order[0].id).toBe("solo");
  });

  it("resolves dependencies before dependents", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "ext", dependencies: ["core"] }));
    manager.addPlugin(makeEntry({ id: "core" }));

    const order = manager.resolveLoadOrder();
    const ids = order.map((e) => e.id);
    expect(ids.indexOf("core")).toBeLessThan(ids.indexOf("ext"));
  });

  it("handles multi-level dependency chains", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(
      makeEntry({ id: "c", dependencies: ["b"] }),
    );
    manager.addPlugin(
      makeEntry({ id: "b", dependencies: ["a"] }),
    );
    manager.addPlugin(makeEntry({ id: "a" }));

    const order = manager.resolveLoadOrder();
    const ids = order.map((e) => e.id);
    expect(ids.indexOf("a")).toBeLessThan(ids.indexOf("b"));
    expect(ids.indexOf("b")).toBeLessThan(ids.indexOf("c"));
  });

  it("respects precedence when resolving duplicate ids", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(
      makeEntry({
        id: "plugin.a",
        precedence: PluginPrecedence.Base,
        version: "1.0.0",
      }),
    );
    manager.addPlugin(
      makeEntry({
        id: "plugin.a",
        precedence: PluginPrecedence.Override,
        version: "2.0.0",
      }),
    );

    const order = manager.resolveLoadOrder();
    // Should deduplicate, keeping highest precedence
    expect(order).toHaveLength(1);
    expect(order[0].version).toBe("2.0.0");
    expect(order[0].precedence).toBe(PluginPrecedence.Override);
  });

  it("sorts by precedence when no dependency constraints", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(
      makeEntry({ id: "user", precedence: PluginPrecedence.User }),
    );
    manager.addPlugin(
      makeEntry({ id: "base", precedence: PluginPrecedence.Base }),
    );
    manager.addPlugin(
      makeEntry({ id: "dep", precedence: PluginPrecedence.Dependency }),
    );

    const order = manager.resolveLoadOrder();
    const ids = order.map((e) => e.id);
    // Base (0) should come before Dependency (1) should come before User (2)
    expect(ids.indexOf("base")).toBeLessThan(ids.indexOf("dep"));
    expect(ids.indexOf("dep")).toBeLessThan(ids.indexOf("user"));
  });

  it("returns empty array for empty manager", () => {
    const manager = new LoadPolicyManager();
    expect(manager.resolveLoadOrder()).toEqual([]);
  });
});

// ---- detectCollisions ----

describe("LoadPolicyManager.detectCollisions", () => {
  it("detects same id with different versions", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "plugin.a", version: "1.0.0" }));
    manager.addPlugin(makeEntry({ id: "plugin.a", version: "2.0.0" }));

    const collisions = manager.detectCollisions();
    expect(collisions).toHaveLength(1);
    expect(collisions[0].id).toBe("plugin.a");
    expect(collisions[0].versions).toEqual(["1.0.0", "2.0.0"]);
  });

  it("returns empty array when no collisions", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "plugin.a", version: "1.0.0" }));
    manager.addPlugin(makeEntry({ id: "plugin.b", version: "1.0.0" }));

    expect(manager.detectCollisions()).toEqual([]);
  });

  it("does not report same id same version as collision", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "plugin.a", version: "1.0.0" }));
    manager.addPlugin(makeEntry({ id: "plugin.a", version: "1.0.0" }));

    expect(manager.detectCollisions()).toEqual([]);
  });

  it("detects multiple collisions", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "a", version: "1.0.0" }));
    manager.addPlugin(makeEntry({ id: "a", version: "2.0.0" }));
    manager.addPlugin(makeEntry({ id: "b", version: "1.0.0" }));
    manager.addPlugin(makeEntry({ id: "b", version: "3.0.0" }));

    const collisions = manager.detectCollisions();
    expect(collisions).toHaveLength(2);
    const ids = collisions.map((c) => c.id).sort();
    expect(ids).toEqual(["a", "b"]);
  });
});

// ---- checkpoint / restore ----

describe("LoadPolicyManager checkpoint and restore", () => {
  it("creates a checkpoint and restores it", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "plugin.a" }));
    manager.addPlugin(makeEntry({ id: "plugin.b" }));

    const cp = manager.createCheckpoint();
    expect(cp.id).toBe(1);
    expect(cp.loadedPlugins).toEqual(["plugin.a", "plugin.b"]);

    // Add more plugins after checkpoint
    manager.addPlugin(makeEntry({ id: "plugin.c" }));
    expect(manager.getLoadOrder()).toHaveLength(3);

    // Restore
    const restored = manager.restoreCheckpoint(cp.id);
    expect(restored).toBe(true);
    expect(manager.getLoadOrder()).toHaveLength(2);
    const ids = manager.getLoadOrder().map((e) => e.id);
    expect(ids).toContain("plugin.a");
    expect(ids).toContain("plugin.b");
    expect(ids).not.toContain("plugin.c");
  });

  it("returns false for nonexistent checkpoint", () => {
    const manager = new LoadPolicyManager();
    expect(manager.restoreCheckpoint(999)).toBe(false);
  });

  it("assigns incrementing checkpoint IDs", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "a" }));

    const cp1 = manager.createCheckpoint();
    const cp2 = manager.createCheckpoint();
    expect(cp1.id).toBe(1);
    expect(cp2.id).toBe(2);
  });

  it("checkpoint has a valid timestamp", () => {
    const manager = new LoadPolicyManager();
    const before = Date.now();
    const cp = manager.createCheckpoint();
    const after = Date.now();
    expect(cp.timestamp).toBeGreaterThanOrEqual(before);
    expect(cp.timestamp).toBeLessThanOrEqual(after);
  });
});

// ---- getEffectivePlugin ----

describe("LoadPolicyManager.getEffectivePlugin", () => {
  it("returns highest precedence entry for a given id", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(
      makeEntry({
        id: "plugin.a",
        precedence: PluginPrecedence.Base,
        version: "1.0.0",
      }),
    );
    manager.addPlugin(
      makeEntry({
        id: "plugin.a",
        precedence: PluginPrecedence.Override,
        version: "2.0.0",
      }),
    );
    manager.addPlugin(
      makeEntry({
        id: "plugin.a",
        precedence: PluginPrecedence.User,
        version: "1.5.0",
      }),
    );

    const effective = manager.getEffectivePlugin("plugin.a");
    expect(effective).toBeDefined();
    expect(effective!.version).toBe("2.0.0");
    expect(effective!.precedence).toBe(PluginPrecedence.Override);
  });

  it("returns undefined for unknown plugin id", () => {
    const manager = new LoadPolicyManager();
    expect(manager.getEffectivePlugin("nonexistent")).toBeUndefined();
  });
});

// ---- clear and getLoadOrder ----

describe("LoadPolicyManager.clear and getLoadOrder", () => {
  it("clear removes all entries and checkpoints", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "a" }));
    manager.addPlugin(makeEntry({ id: "b" }));
    manager.createCheckpoint();

    manager.clear();
    expect(manager.getLoadOrder()).toEqual([]);
  });

  it("getLoadOrder returns a copy", () => {
    const manager = new LoadPolicyManager();
    manager.addPlugin(makeEntry({ id: "a" }));

    const order = manager.getLoadOrder();
    order.push(makeEntry({ id: "b" }));

    // Original should not be affected
    expect(manager.getLoadOrder()).toHaveLength(1);
  });
});

// ---- comparePrec ----

describe("comparePrec", () => {
  it("returns negative when a < b", () => {
    expect(comparePrec(PluginPrecedence.Base, PluginPrecedence.Override)).toBeLessThan(0);
  });

  it("returns 0 when equal", () => {
    expect(comparePrec(PluginPrecedence.User, PluginPrecedence.User)).toBe(0);
  });

  it("returns positive when a > b", () => {
    expect(comparePrec(PluginPrecedence.Override, PluginPrecedence.Base)).toBeGreaterThan(0);
  });
});
