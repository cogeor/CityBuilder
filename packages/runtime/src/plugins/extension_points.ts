// @townbuilder/runtime — Plugin extension points and runtime extension registry
// Allows plugins to register typed handlers at well-known extension points,
// with lifecycle management, priority ordering, and batch invocation.

// ---- Extension Point Type ----

/** The kind of extension a plugin provides. */
export enum ExtensionPointType {
  Progression = "progression",
  Scenario = "scenario",
  EventChain = "event_chain",
  PolicyHook = "policy_hook",
  SystemModifier = "system_modifier",
}

// ---- Extension Lifecycle ----

/** Lifecycle state of a registered extension. */
export enum ExtensionLifecycle {
  Registered = "registered",
  Active = "active",
  Inactive = "inactive",
  Unloaded = "unloaded",
}

// ---- Extension Point ----

/** A single extension point registered by a plugin. */
export interface ExtensionPoint {
  /** Unique identifier for this extension point. */
  id: string;
  /** ID of the plugin that registered this extension. */
  pluginId: string;
  /** The type of extension point. */
  type: ExtensionPointType;
  /** Current lifecycle state. */
  lifecycle: ExtensionLifecycle;
  /** Priority for invocation ordering; lower values = higher priority. */
  priority: number;
  /** The handler function invoked when the extension point fires. */
  handler: (...args: unknown[]) => unknown;
}

// ---- Extension Registry ----

/** Central registry for all plugin extension points. */
export class ExtensionRegistry {
  private extensions: Map<ExtensionPointType, ExtensionPoint[]> = new Map();

  /**
   * Register a new extension point.
   * Adds it to the type-keyed bucket with its initial lifecycle state.
   */
  register(extension: ExtensionPoint): void {
    const bucket = this.extensions.get(extension.type);
    if (bucket) {
      bucket.push(extension);
    } else {
      this.extensions.set(extension.type, [extension]);
    }
  }

  /**
   * Activate an extension by ID.
   * Sets lifecycle to Active if currently Registered or Inactive.
   * Returns true if the transition succeeded.
   */
  activate(extensionId: string): boolean {
    const ext = this.findById(extensionId);
    if (!ext) return false;
    if (
      ext.lifecycle === ExtensionLifecycle.Registered ||
      ext.lifecycle === ExtensionLifecycle.Inactive
    ) {
      ext.lifecycle = ExtensionLifecycle.Active;
      return true;
    }
    return false;
  }

  /**
   * Deactivate an extension by ID.
   * Sets lifecycle to Inactive if currently Active.
   * Returns true if the transition succeeded.
   */
  deactivate(extensionId: string): boolean {
    const ext = this.findById(extensionId);
    if (!ext) return false;
    if (ext.lifecycle === ExtensionLifecycle.Active) {
      ext.lifecycle = ExtensionLifecycle.Inactive;
      return true;
    }
    return false;
  }

  /**
   * Unload all extensions belonging to a plugin.
   * Removes them from the registry entirely.
   * Returns the count of extensions removed.
   */
  unload(pluginId: string): number {
    let removed = 0;
    for (const [type, bucket] of this.extensions) {
      const before = bucket.length;
      const filtered = bucket.filter((ext) => ext.pluginId !== pluginId);
      removed += before - filtered.length;
      if (filtered.length === 0) {
        this.extensions.delete(type);
      } else {
        this.extensions.set(type, filtered);
      }
    }
    return removed;
  }

  /** Get all extensions of a given type, regardless of lifecycle. */
  getExtensions(type: ExtensionPointType): ExtensionPoint[] {
    return this.extensions.get(type) ?? [];
  }

  /**
   * Get all active extensions of a given type, sorted by priority (ascending).
   * Lower priority values come first.
   */
  getActiveExtensions(type: ExtensionPointType): ExtensionPoint[] {
    const bucket = this.extensions.get(type) ?? [];
    return bucket
      .filter((ext) => ext.lifecycle === ExtensionLifecycle.Active)
      .sort((a, b) => a.priority - b.priority);
  }

  /**
   * Invoke all active handlers for a given extension point type.
   * Handlers are called in priority order (ascending).
   * Returns an array of results from each handler.
   */
  invoke(type: ExtensionPointType, ...args: unknown[]): unknown[] {
    const active = this.getActiveExtensions(type);
    return active.map((ext) => ext.handler(...args));
  }

  /** Get the total number of registered extensions across all types. */
  getCount(): number {
    let count = 0;
    for (const bucket of this.extensions.values()) {
      count += bucket.length;
    }
    return count;
  }

  /** Remove all registered extensions. */
  clear(): void {
    this.extensions.clear();
  }

  // ---- Private helpers ----

  private findById(extensionId: string): ExtensionPoint | undefined {
    for (const bucket of this.extensions.values()) {
      const found = bucket.find((ext) => ext.id === extensionId);
      if (found) return found;
    }
    return undefined;
  }
}
