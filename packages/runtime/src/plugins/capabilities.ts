// @townbuilder/runtime — Plugin capability model
// Defines the capabilities that plugins can request and enforces
// exclusivity constraints to prevent conflicting modifications.

// ---- Capability Enum ----

/** Capabilities that a plugin can request from the engine. */
export enum PluginCapability {
  ReadStats = "read_stats",
  ProgressionHooks = "progression_hooks",
  ScenarioHooks = "scenario_hooks",
  AIHooks = "ai_hooks",
  ToolingHooks = "tooling_hooks",
  ModifyEconomy = "modify_economy",
  ModifyTerrain = "modify_terrain",
  CustomUI = "custom_ui",
}

// ---- Capability Requirement ----

/** Declares a capability requirement with its necessity and reason. */
export interface CapabilityRequirement {
  /** Which capability is being requested. */
  capability: PluginCapability;
  /** Whether the capability is required (true) or optional (false). */
  required: boolean;
  /** Human-readable reason for needing this capability. */
  reason: string;
}

// ---- Exclusive Capabilities ----

/**
 * Capabilities that are mutually exclusive: only one plugin at a time
 * can hold each of these capabilities (e.g., two plugins cannot both
 * modify the economy simultaneously).
 */
export const EXCLUSIVE_CAPABILITIES: ReadonlySet<PluginCapability> = new Set([
  PluginCapability.ModifyEconomy,
  PluginCapability.ModifyTerrain,
]);

// ---- Validation Functions ----

/**
 * Validate that a set of requested capabilities does not conflict with
 * already-active capabilities. Exclusive capabilities can only be held
 * by one plugin at a time.
 *
 * @param requested - Capabilities the new plugin wants
 * @param existingActive - Capabilities already granted to other plugins
 * @returns Object with valid flag and array of conflict descriptions
 */
export function validateCapabilities(
  requested: PluginCapability[],
  existingActive: PluginCapability[],
): { valid: boolean; conflicts: string[] } {
  const conflicts: string[] = [];

  for (const cap of requested) {
    if (EXCLUSIVE_CAPABILITIES.has(cap) && existingActive.includes(cap)) {
      conflicts.push(
        `Capability "${cap}" is exclusive and already granted to another plugin`,
      );
    }
  }

  return { valid: conflicts.length === 0, conflicts };
}

/**
 * Check whether a capability list includes a specific capability.
 *
 * @param capabilities - The list to search
 * @param check - The capability to look for
 * @returns true if the capability is present
 */
export function hasCapability(
  capabilities: PluginCapability[],
  check: PluginCapability,
): boolean {
  return capabilities.includes(check);
}

// ---- Capability Manager ----

/**
 * Manages granted capabilities across all active plugins.
 * Enforces exclusivity constraints when granting new capabilities.
 */
export class CapabilityManager {
  private readonly granted: Map<string, PluginCapability[]> = new Map();

  /**
   * Grant capabilities to a plugin. Checks for exclusive conflicts
   * against all currently active capabilities.
   *
   * @param pluginId - The plugin requesting capabilities
   * @param capabilities - The capabilities to grant
   * @returns Object with valid flag and any conflict descriptions
   */
  grant(
    pluginId: string,
    capabilities: PluginCapability[],
  ): { valid: boolean; conflicts: string[] } {
    const allActive = this.getAllActive();
    const result = validateCapabilities(capabilities, allActive);

    if (result.valid) {
      this.granted.set(pluginId, [...capabilities]);
    }

    return result;
  }

  /**
   * Revoke all capabilities from a plugin.
   *
   * @param pluginId - The plugin whose capabilities to revoke
   */
  revoke(pluginId: string): void {
    this.granted.delete(pluginId);
  }

  /**
   * Get the capabilities granted to a specific plugin.
   *
   * @param pluginId - The plugin to look up
   * @returns Array of granted capabilities (empty if none)
   */
  getCapabilities(pluginId: string): PluginCapability[] {
    return this.granted.get(pluginId) ?? [];
  }

  /**
   * Get all currently active capabilities across all plugins.
   *
   * @returns Flat array of all granted capabilities
   */
  getAllActive(): PluginCapability[] {
    const result: PluginCapability[] = [];
    for (const caps of this.granted.values()) {
      result.push(...caps);
    }
    return result;
  }

  /**
   * Check whether an exclusive capability is already granted to any plugin.
   *
   * @param capability - The capability to check
   * @returns true if the capability is exclusive and already granted
   */
  hasConflict(capability: PluginCapability): boolean {
    if (!EXCLUSIVE_CAPABILITIES.has(capability)) {
      return false;
    }
    return this.getAllActive().includes(capability);
  }

  /** Remove all granted capabilities for all plugins. */
  clear(): void {
    this.granted.clear();
  }
}
