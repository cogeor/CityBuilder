import { describe, it, expect, vi } from "vitest";
import {
  ExtensionPointType,
  ExtensionLifecycle,
  ExtensionRegistry,
  type ExtensionPoint,
} from "../index.js";

// ---- Test Helpers ----

/** Create a minimal extension point with optional overrides. */
function makeExtension(
  overrides: Partial<ExtensionPoint> = {},
): ExtensionPoint {
  return {
    id: "ext.test",
    pluginId: "plugin.test",
    type: ExtensionPointType.Progression,
    lifecycle: ExtensionLifecycle.Registered,
    priority: 10,
    handler: vi.fn(() => "default"),
    ...overrides,
  };
}

// ---- register ----

describe("ExtensionRegistry.register", () => {
  it("adds an extension to the registry", () => {
    const registry = new ExtensionRegistry();
    const ext = makeExtension({ id: "ext.1" });
    registry.register(ext);
    expect(registry.getCount()).toBe(1);
  });

  it("registers multiple extensions of the same type", () => {
    const registry = new ExtensionRegistry();
    registry.register(makeExtension({ id: "ext.1" }));
    registry.register(makeExtension({ id: "ext.2" }));
    expect(registry.getCount()).toBe(2);
    expect(
      registry.getExtensions(ExtensionPointType.Progression),
    ).toHaveLength(2);
  });

  it("registers extensions of different types", () => {
    const registry = new ExtensionRegistry();
    registry.register(
      makeExtension({ id: "ext.1", type: ExtensionPointType.Progression }),
    );
    registry.register(
      makeExtension({ id: "ext.2", type: ExtensionPointType.Scenario }),
    );
    expect(registry.getCount()).toBe(2);
    expect(
      registry.getExtensions(ExtensionPointType.Progression),
    ).toHaveLength(1);
    expect(
      registry.getExtensions(ExtensionPointType.Scenario),
    ).toHaveLength(1);
  });
});

// ---- activate / deactivate lifecycle ----

describe("ExtensionRegistry.activate and deactivate", () => {
  it("activates a registered extension", () => {
    const registry = new ExtensionRegistry();
    const ext = makeExtension({
      id: "ext.1",
      lifecycle: ExtensionLifecycle.Registered,
    });
    registry.register(ext);

    expect(registry.activate("ext.1")).toBe(true);
    expect(ext.lifecycle).toBe(ExtensionLifecycle.Active);
  });

  it("activates an inactive extension", () => {
    const registry = new ExtensionRegistry();
    const ext = makeExtension({
      id: "ext.1",
      lifecycle: ExtensionLifecycle.Inactive,
    });
    registry.register(ext);

    expect(registry.activate("ext.1")).toBe(true);
    expect(ext.lifecycle).toBe(ExtensionLifecycle.Active);
  });

  it("returns false when activating an already active extension", () => {
    const registry = new ExtensionRegistry();
    const ext = makeExtension({
      id: "ext.1",
      lifecycle: ExtensionLifecycle.Active,
    });
    registry.register(ext);

    expect(registry.activate("ext.1")).toBe(false);
  });

  it("returns false when activating a nonexistent extension", () => {
    const registry = new ExtensionRegistry();
    expect(registry.activate("ghost")).toBe(false);
  });

  it("deactivates an active extension", () => {
    const registry = new ExtensionRegistry();
    const ext = makeExtension({
      id: "ext.1",
      lifecycle: ExtensionLifecycle.Active,
    });
    registry.register(ext);

    expect(registry.deactivate("ext.1")).toBe(true);
    expect(ext.lifecycle).toBe(ExtensionLifecycle.Inactive);
  });

  it("returns false when deactivating a non-active extension", () => {
    const registry = new ExtensionRegistry();
    const ext = makeExtension({
      id: "ext.1",
      lifecycle: ExtensionLifecycle.Registered,
    });
    registry.register(ext);

    expect(registry.deactivate("ext.1")).toBe(false);
  });

  it("returns false when deactivating a nonexistent extension", () => {
    const registry = new ExtensionRegistry();
    expect(registry.deactivate("ghost")).toBe(false);
  });
});

// ---- invoke ----

describe("ExtensionRegistry.invoke", () => {
  it("invokes active handlers in priority order", () => {
    const registry = new ExtensionRegistry();
    const callOrder: number[] = [];

    registry.register(
      makeExtension({
        id: "ext.low",
        priority: 20,
        lifecycle: ExtensionLifecycle.Active,
        handler: () => {
          callOrder.push(20);
          return "low";
        },
      }),
    );

    registry.register(
      makeExtension({
        id: "ext.high",
        priority: 5,
        lifecycle: ExtensionLifecycle.Active,
        handler: () => {
          callOrder.push(5);
          return "high";
        },
      }),
    );

    registry.register(
      makeExtension({
        id: "ext.mid",
        priority: 10,
        lifecycle: ExtensionLifecycle.Active,
        handler: () => {
          callOrder.push(10);
          return "mid";
        },
      }),
    );

    const results = registry.invoke(ExtensionPointType.Progression);
    expect(results).toEqual(["high", "mid", "low"]);
    expect(callOrder).toEqual([5, 10, 20]);
  });

  it("skips inactive extensions during invoke", () => {
    const registry = new ExtensionRegistry();
    const inactiveHandler = vi.fn(() => "inactive");
    const activeHandler = vi.fn(() => "active");

    registry.register(
      makeExtension({
        id: "ext.inactive",
        lifecycle: ExtensionLifecycle.Registered,
        handler: inactiveHandler,
      }),
    );

    registry.register(
      makeExtension({
        id: "ext.active",
        lifecycle: ExtensionLifecycle.Active,
        handler: activeHandler,
      }),
    );

    const results = registry.invoke(ExtensionPointType.Progression);
    expect(results).toEqual(["active"]);
    expect(inactiveHandler).not.toHaveBeenCalled();
    expect(activeHandler).toHaveBeenCalledOnce();
  });

  it("passes arguments to handlers", () => {
    const registry = new ExtensionRegistry();
    const handler = vi.fn((...args: unknown[]) => args);

    registry.register(
      makeExtension({
        id: "ext.1",
        lifecycle: ExtensionLifecycle.Active,
        handler,
      }),
    );

    registry.invoke(ExtensionPointType.Progression, "arg1", 42);
    expect(handler).toHaveBeenCalledWith("arg1", 42);
  });

  it("returns empty array when no extensions of type", () => {
    const registry = new ExtensionRegistry();
    const results = registry.invoke(ExtensionPointType.PolicyHook);
    expect(results).toEqual([]);
  });
});

// ---- unload ----

describe("ExtensionRegistry.unload", () => {
  it("removes all extensions for a plugin", () => {
    const registry = new ExtensionRegistry();
    registry.register(makeExtension({ id: "ext.1", pluginId: "plugin.a" }));
    registry.register(makeExtension({ id: "ext.2", pluginId: "plugin.a" }));
    registry.register(makeExtension({ id: "ext.3", pluginId: "plugin.b" }));

    const removed = registry.unload("plugin.a");
    expect(removed).toBe(2);
    expect(registry.getCount()).toBe(1);
  });

  it("returns 0 when plugin has no extensions", () => {
    const registry = new ExtensionRegistry();
    registry.register(makeExtension({ id: "ext.1", pluginId: "plugin.a" }));

    const removed = registry.unload("plugin.nonexistent");
    expect(removed).toBe(0);
    expect(registry.getCount()).toBe(1);
  });

  it("removes extensions across multiple types", () => {
    const registry = new ExtensionRegistry();
    registry.register(
      makeExtension({
        id: "ext.1",
        pluginId: "plugin.a",
        type: ExtensionPointType.Progression,
      }),
    );
    registry.register(
      makeExtension({
        id: "ext.2",
        pluginId: "plugin.a",
        type: ExtensionPointType.Scenario,
      }),
    );

    const removed = registry.unload("plugin.a");
    expect(removed).toBe(2);
    expect(registry.getCount()).toBe(0);
  });
});

// ---- getActiveExtensions ----

describe("ExtensionRegistry.getActiveExtensions", () => {
  it("filters to only active extensions", () => {
    const registry = new ExtensionRegistry();
    registry.register(
      makeExtension({
        id: "ext.active",
        lifecycle: ExtensionLifecycle.Active,
      }),
    );
    registry.register(
      makeExtension({
        id: "ext.registered",
        lifecycle: ExtensionLifecycle.Registered,
      }),
    );
    registry.register(
      makeExtension({
        id: "ext.inactive",
        lifecycle: ExtensionLifecycle.Inactive,
      }),
    );

    const active = registry.getActiveExtensions(ExtensionPointType.Progression);
    expect(active).toHaveLength(1);
    expect(active[0].id).toBe("ext.active");
  });

  it("returns extensions sorted by priority ascending", () => {
    const registry = new ExtensionRegistry();
    registry.register(
      makeExtension({
        id: "ext.3",
        priority: 30,
        lifecycle: ExtensionLifecycle.Active,
      }),
    );
    registry.register(
      makeExtension({
        id: "ext.1",
        priority: 10,
        lifecycle: ExtensionLifecycle.Active,
      }),
    );
    registry.register(
      makeExtension({
        id: "ext.2",
        priority: 20,
        lifecycle: ExtensionLifecycle.Active,
      }),
    );

    const active = registry.getActiveExtensions(ExtensionPointType.Progression);
    expect(active.map((e) => e.id)).toEqual(["ext.1", "ext.2", "ext.3"]);
  });

  it("returns empty array for type with no extensions", () => {
    const registry = new ExtensionRegistry();
    const active = registry.getActiveExtensions(
      ExtensionPointType.SystemModifier,
    );
    expect(active).toEqual([]);
  });
});

// ---- clear and getCount ----

describe("ExtensionRegistry.clear and getCount", () => {
  it("clear removes all extensions", () => {
    const registry = new ExtensionRegistry();
    registry.register(makeExtension({ id: "ext.1" }));
    registry.register(makeExtension({ id: "ext.2" }));
    expect(registry.getCount()).toBe(2);

    registry.clear();
    expect(registry.getCount()).toBe(0);
  });

  it("getCount returns 0 for empty registry", () => {
    const registry = new ExtensionRegistry();
    expect(registry.getCount()).toBe(0);
  });
});
