import { describe, it, expect, beforeEach } from "vitest";
import {
  PluginCapability,
  EXCLUSIVE_CAPABILITIES,
  validateCapabilities,
  hasCapability,
  CapabilityManager,
} from "../index.js";

// ---- validateCapabilities ----

describe("validateCapabilities", () => {
  it("passes when no conflicts exist", () => {
    const result = validateCapabilities(
      [PluginCapability.ReadStats, PluginCapability.CustomUI],
      [PluginCapability.ProgressionHooks],
    );
    expect(result.valid).toBe(true);
    expect(result.conflicts).toHaveLength(0);
  });

  it("passes when requesting non-exclusive capabilities already active", () => {
    const result = validateCapabilities(
      [PluginCapability.ReadStats],
      [PluginCapability.ReadStats],
    );
    expect(result.valid).toBe(true);
  });

  it("detects conflict on exclusive ModifyEconomy", () => {
    const result = validateCapabilities(
      [PluginCapability.ModifyEconomy],
      [PluginCapability.ModifyEconomy],
    );
    expect(result.valid).toBe(false);
    expect(result.conflicts).toHaveLength(1);
    expect(result.conflicts[0]).toContain("modify_economy");
  });

  it("detects conflict on exclusive ModifyTerrain", () => {
    const result = validateCapabilities(
      [PluginCapability.ModifyTerrain],
      [PluginCapability.ModifyTerrain],
    );
    expect(result.valid).toBe(false);
    expect(result.conflicts).toHaveLength(1);
    expect(result.conflicts[0]).toContain("modify_terrain");
  });

  it("allows exclusive capability when not already active", () => {
    const result = validateCapabilities(
      [PluginCapability.ModifyEconomy],
      [PluginCapability.ReadStats],
    );
    expect(result.valid).toBe(true);
  });

  it("passes with empty requested capabilities", () => {
    const result = validateCapabilities([], [PluginCapability.ModifyEconomy]);
    expect(result.valid).toBe(true);
    expect(result.conflicts).toHaveLength(0);
  });

  it("passes with empty existing capabilities", () => {
    const result = validateCapabilities(
      [PluginCapability.ModifyEconomy, PluginCapability.ModifyTerrain],
      [],
    );
    expect(result.valid).toBe(true);
  });
});

// ---- hasCapability ----

describe("hasCapability", () => {
  it("returns true when capability is present", () => {
    expect(
      hasCapability(
        [PluginCapability.ReadStats, PluginCapability.CustomUI],
        PluginCapability.ReadStats,
      ),
    ).toBe(true);
  });

  it("returns false when capability is absent", () => {
    expect(
      hasCapability(
        [PluginCapability.ReadStats],
        PluginCapability.ModifyEconomy,
      ),
    ).toBe(false);
  });

  it("returns false for empty capability list", () => {
    expect(hasCapability([], PluginCapability.ReadStats)).toBe(false);
  });
});

// ---- EXCLUSIVE_CAPABILITIES ----

describe("EXCLUSIVE_CAPABILITIES", () => {
  it("contains ModifyEconomy and ModifyTerrain", () => {
    expect(EXCLUSIVE_CAPABILITIES.has(PluginCapability.ModifyEconomy)).toBe(true);
    expect(EXCLUSIVE_CAPABILITIES.has(PluginCapability.ModifyTerrain)).toBe(true);
  });

  it("does not contain non-exclusive capabilities", () => {
    expect(EXCLUSIVE_CAPABILITIES.has(PluginCapability.ReadStats)).toBe(false);
    expect(EXCLUSIVE_CAPABILITIES.has(PluginCapability.CustomUI)).toBe(false);
    expect(EXCLUSIVE_CAPABILITIES.has(PluginCapability.ProgressionHooks)).toBe(false);
  });
});

// ---- CapabilityManager ----

describe("CapabilityManager", () => {
  let manager: CapabilityManager;

  beforeEach(() => {
    manager = new CapabilityManager();
  });

  it("grants capabilities to a plugin", () => {
    const result = manager.grant("plugin.a", [
      PluginCapability.ReadStats,
      PluginCapability.CustomUI,
    ]);
    expect(result.valid).toBe(true);
    expect(manager.getCapabilities("plugin.a")).toEqual([
      PluginCapability.ReadStats,
      PluginCapability.CustomUI,
    ]);
  });

  it("returns empty array for unknown plugin", () => {
    expect(manager.getCapabilities("unknown")).toEqual([]);
  });

  it("revokes all capabilities from a plugin", () => {
    manager.grant("plugin.a", [PluginCapability.ReadStats]);
    manager.revoke("plugin.a");
    expect(manager.getCapabilities("plugin.a")).toEqual([]);
  });

  it("getAllActive returns all capabilities from all plugins", () => {
    manager.grant("plugin.a", [PluginCapability.ReadStats]);
    manager.grant("plugin.b", [PluginCapability.CustomUI]);
    const active = manager.getAllActive();
    expect(active).toContain(PluginCapability.ReadStats);
    expect(active).toContain(PluginCapability.CustomUI);
    expect(active).toHaveLength(2);
  });

  it("prevents granting exclusive capability twice", () => {
    const first = manager.grant("plugin.a", [PluginCapability.ModifyEconomy]);
    expect(first.valid).toBe(true);

    const second = manager.grant("plugin.b", [PluginCapability.ModifyEconomy]);
    expect(second.valid).toBe(false);
    expect(second.conflicts).toHaveLength(1);
    // plugin.b should not have been granted
    expect(manager.getCapabilities("plugin.b")).toEqual([]);
  });

  it("allows granting exclusive capability after revocation", () => {
    manager.grant("plugin.a", [PluginCapability.ModifyTerrain]);
    manager.revoke("plugin.a");

    const result = manager.grant("plugin.b", [PluginCapability.ModifyTerrain]);
    expect(result.valid).toBe(true);
    expect(manager.getCapabilities("plugin.b")).toContain(PluginCapability.ModifyTerrain);
  });

  it("hasConflict returns true for exclusive capability already granted", () => {
    manager.grant("plugin.a", [PluginCapability.ModifyEconomy]);
    expect(manager.hasConflict(PluginCapability.ModifyEconomy)).toBe(true);
  });

  it("hasConflict returns false for non-exclusive capability", () => {
    manager.grant("plugin.a", [PluginCapability.ReadStats]);
    expect(manager.hasConflict(PluginCapability.ReadStats)).toBe(false);
  });

  it("hasConflict returns false when no plugins are active", () => {
    expect(manager.hasConflict(PluginCapability.ModifyEconomy)).toBe(false);
  });

  it("clear removes all granted capabilities", () => {
    manager.grant("plugin.a", [PluginCapability.ReadStats]);
    manager.grant("plugin.b", [PluginCapability.ModifyEconomy]);
    manager.clear();
    expect(manager.getAllActive()).toHaveLength(0);
    expect(manager.getCapabilities("plugin.a")).toEqual([]);
    expect(manager.getCapabilities("plugin.b")).toEqual([]);
  });

  it("granting does not modify the input array", () => {
    const caps = [PluginCapability.ReadStats];
    manager.grant("plugin.a", caps);
    caps.push(PluginCapability.ModifyEconomy);
    // Internal copy should not be affected
    expect(manager.getCapabilities("plugin.a")).toEqual([PluginCapability.ReadStats]);
  });
});
