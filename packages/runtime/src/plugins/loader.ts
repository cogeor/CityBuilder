// @townbuilder/runtime — Plugin registry and dependency resolver
// Manages plugin lifecycle: registration, lookup, and load-order resolution.

import type { PluginManifest, PluginEntry } from "./manifest.js";

// ---- Plugin Registry ----

/** Central registry for all loaded plugins. */
export class PluginRegistry {
  private readonly plugins: Map<string, PluginEntry> = new Map();

  /**
   * Register a plugin manifest. Creates an unloaded PluginEntry.
   * Returns false if a plugin with the same ID is already registered.
   */
  register(manifest: PluginManifest): boolean {
    if (this.plugins.has(manifest.id)) {
      return false;
    }

    const entry: PluginEntry = {
      manifest,
      loaded: false,
      data: null,
    };

    this.plugins.set(manifest.id, entry);
    return true;
  }

  /** Retrieve a plugin entry by ID, or undefined if not found. */
  get(id: string): PluginEntry | undefined {
    return this.plugins.get(id);
  }

  /** Return an array of all registered plugin manifests. */
  list(): PluginManifest[] {
    return Array.from(this.plugins.values()).map((entry) => entry.manifest);
  }

  /** Return manifests filtered by content_type. */
  listByType(type: string): PluginManifest[] {
    return Array.from(this.plugins.values())
      .filter((entry) => entry.manifest.content_type === type)
      .map((entry) => entry.manifest);
  }

  /**
   * Remove a plugin from the registry.
   * Returns false if the plugin was not registered.
   */
  unregister(id: string): boolean {
    return this.plugins.delete(id);
  }

  /** Check whether a plugin's data has been loaded. */
  isLoaded(id: string): boolean {
    const entry = this.plugins.get(id);
    return entry !== undefined && entry.loaded;
  }

  /** Return the number of registered plugins. */
  count(): number {
    return this.plugins.size;
  }
}

// ---- Dependency Resolution ----

/**
 * Validate that all declared dependencies exist within the given manifest set.
 * Returns an array of missing dependency IDs (empty if all satisfied).
 */
export function validateDependencies(manifests: PluginManifest[]): string[] {
  const knownIds = new Set(manifests.map((m) => m.id));
  const missing: string[] = [];

  for (const manifest of manifests) {
    for (const dep of manifest.dependencies) {
      if (!knownIds.has(dep)) {
        if (!missing.includes(dep)) {
          missing.push(dep);
        }
      }
    }
  }

  return missing;
}

/**
 * Topological sort of plugin manifests by dependency order.
 * Returns manifests ordered so that dependencies come before dependents.
 * Throws an Error if a circular dependency is detected.
 */
export function resolveDependencies(
  manifests: PluginManifest[],
): PluginManifest[] {
  // Build lookup and adjacency
  const byId = new Map<string, PluginManifest>();
  for (const m of manifests) {
    byId.set(m.id, m);
  }

  // Track visit state: 0 = unvisited, 1 = in-progress, 2 = done
  const state = new Map<string, number>();
  for (const m of manifests) {
    state.set(m.id, 0);
  }

  const sorted: PluginManifest[] = [];

  function visit(id: string): void {
    const s = state.get(id);
    if (s === 2) return; // already processed
    if (s === 1) {
      throw new Error(`Circular dependency detected involving plugin "${id}"`);
    }

    const manifest = byId.get(id);
    if (manifest === undefined) return; // external dep, skip

    state.set(id, 1); // mark in-progress

    for (const dep of manifest.dependencies) {
      visit(dep);
    }

    state.set(id, 2); // mark done
    sorted.push(manifest);
  }

  for (const m of manifests) {
    visit(m.id);
  }

  return sorted;
}
