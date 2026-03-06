// @townbuilder/runtime -- Plugin host abstraction
// Minimal runtime layer: discover -> normalize -> validate -> resolve -> load.

import { normalizeManifest, validateManifest, type PluginManifest } from "./manifest.js";
import {
  PluginRegistry,
  resolveDependencies,
  validateDependencies,
} from "./loader.js";

/** Abstract source for plugin manifests and payloads. */
export interface IPluginSource {
  discover(): Promise<unknown[]>;
  loadContent(manifest: PluginManifest): Promise<unknown>;
}

/** Activation report for host operations. */
export interface PluginActivationReport {
  readonly registered: string[];
  readonly activated: string[];
  readonly errors: string[];
}

/**
 * Runtime plugin host. Keeps loading policy separate from source transport.
 * This allows web file-picker, OPFS, HTTP, or test in-memory sources.
 */
export class PluginHost {
  private readonly registry: PluginRegistry;

  constructor(registry?: PluginRegistry) {
    this.registry = registry ?? new PluginRegistry();
  }

  getRegistry(): PluginRegistry {
    return this.registry;
  }

  /** Normalize and register manifests from a source. */
  async discoverAndRegister(source: IPluginSource): Promise<PluginActivationReport> {
    const errors: string[] = [];
    const registered: string[] = [];

    const raws = await source.discover();
    for (const raw of raws) {
      const normalized = normalizeManifest(raw);
      if (!normalized || !validateManifest(normalized)) {
        errors.push("invalid manifest encountered during discovery");
        continue;
      }
      if (!this.registry.register(normalized)) {
        errors.push(`failed to register plugin \"${normalized.id}\"`);
        continue;
      }
      registered.push(normalized.id);
    }

    return { registered, activated: [], errors };
  }

  /** Resolve load order for all currently registered plugins. */
  resolveLoadOrder(): PluginManifest[] {
    const manifests = this.registry.list();
    const missing = validateDependencies(manifests);
    if (missing.length > 0) {
      throw new Error(`Missing dependencies: ${missing.join(", ")}`);
    }
    return resolveDependencies(manifests);
  }

  /** Activate all plugins in deterministic order using the provided source. */
  async activateAll(source: IPluginSource): Promise<PluginActivationReport> {
    const errors: string[] = [];
    const activated: string[] = [];

    const ordered = this.resolveLoadOrder();
    for (const manifest of ordered) {
      try {
        const payload = await source.loadContent(manifest);
        if (!this.registry.setLoaded(manifest.id, payload)) {
          errors.push(`failed to mark plugin \"${manifest.id}\" as loaded`);
          continue;
        }
        activated.push(manifest.id);
      } catch (err: unknown) {
        errors.push(
          `failed to load plugin \"${manifest.id}\": ${err instanceof Error ? err.message : String(err)}`,
        );
      }
    }

    return { registered: this.registry.list().map((m) => m.id), activated, errors };
  }
}

/**
 * Basic in-memory source for tests and bootstrap prototypes.
 */
export class InMemoryPluginSource implements IPluginSource {
  private readonly manifests: unknown[];
  private readonly payloads: Readonly<Record<string, unknown>>;

  constructor(manifests: unknown[], payloads?: Readonly<Record<string, unknown>>) {
    this.manifests = manifests;
    this.payloads = payloads ?? {};
  }

  async discover(): Promise<unknown[]> {
    return this.manifests;
  }

  async loadContent(manifest: PluginManifest): Promise<unknown> {
    if (manifest.id in this.payloads) {
      return this.payloads[manifest.id];
    }
    return { manifestId: manifest.id, contributes: manifest.contributes };
  }
}
