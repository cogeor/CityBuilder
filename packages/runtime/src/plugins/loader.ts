// @townbuilder/runtime -- Plugin registry and dependency resolver
// Canonical registry for normalized manifests plus deterministic load ordering.

import {
  normalizeManifest,
  validateManifest,
  type PluginManifest,
  type PluginEntry,
} from "./manifest.js";

function manifestProvidesType(manifest: PluginManifest, type: string): boolean {
  if (manifest.content_type === type) return true;
  return type in manifest.contributes;
}

/** Central registry for all discovered plugins. */
export class PluginRegistry {
  private readonly plugins: Map<string, PluginEntry> = new Map();

  /** Register a manifest. Returns false on duplicate or invalid manifest. */
  register(manifest: PluginManifest): boolean {
    const normalized = normalizeManifest(manifest);
    if (!normalized || !validateManifest(normalized)) {
      return false;
    }
    if (this.plugins.has(normalized.id)) {
      return false;
    }

    this.plugins.set(normalized.id, {
      manifest: normalized,
      loaded: false,
      data: null,
    });
    return true;
  }

  /** Retrieve a plugin entry by ID. */
  get(id: string): PluginEntry | undefined {
    return this.plugins.get(id);
  }

  /** Return all normalized plugin manifests. */
  list(): PluginManifest[] {
    return Array.from(this.plugins.values()).map((entry) => entry.manifest);
  }

  /** Return manifests that contribute to a content type. */
  listByType(type: string): PluginManifest[] {
    return Array.from(this.plugins.values())
      .filter((entry) => manifestProvidesType(entry.manifest, type))
      .map((entry) => entry.manifest);
  }

  /** Remove a plugin from the registry. */
  unregister(id: string): boolean {
    return this.plugins.delete(id);
  }

  /** Mark a plugin as loaded with arbitrary payload. */
  setLoaded(id: string, data: unknown): boolean {
    const entry = this.plugins.get(id);
    if (!entry) return false;
    entry.loaded = true;
    entry.data = data;
    return true;
  }

  /** Mark plugin as unloaded and clear runtime payload. */
  setUnloaded(id: string): boolean {
    const entry = this.plugins.get(id);
    if (!entry) return false;
    entry.loaded = false;
    entry.data = null;
    return true;
  }

  /** Check whether a plugin's data has been loaded. */
  isLoaded(id: string): boolean {
    const entry = this.plugins.get(id);
    return entry !== undefined && entry.loaded;
  }

  /** Return number of registered plugins. */
  count(): number {
    return this.plugins.size;
  }

  /** Clear all registry entries. */
  clear(): void {
    this.plugins.clear();
  }
}

/** Validate that declared dependencies are present in the manifest set. */
export function validateDependencies(manifests: PluginManifest[]): string[] {
  const knownIds = new Set(manifests.map((m) => m.id));
  const missing: string[] = [];

  for (const manifest of manifests) {
    for (const dep of manifest.dependencies) {
      if (!knownIds.has(dep) && !missing.includes(dep)) {
        missing.push(dep);
      }
    }
  }

  return missing;
}

/** Topological sort of manifests by dependency order. */
export function resolveDependencies(
  manifests: PluginManifest[],
): PluginManifest[] {
  const byId = new Map<string, PluginManifest>();
  for (const m of manifests) {
    byId.set(m.id, m);
  }

  const state = new Map<string, number>();
  for (const m of manifests) {
    state.set(m.id, 0);
  }

  const sorted: PluginManifest[] = [];

  function visit(id: string): void {
    const s = state.get(id);
    if (s === 2) return;
    if (s === 1) {
      throw new Error(`Circular dependency detected involving plugin "${id}"`);
    }

    const manifest = byId.get(id);
    if (manifest === undefined) return;

    state.set(id, 1);
    for (const dep of manifest.dependencies) {
      visit(dep);
    }
    state.set(id, 2);
    sorted.push(manifest);
  }

  for (const m of manifests) {
    visit(m.id);
  }

  return sorted;
}
