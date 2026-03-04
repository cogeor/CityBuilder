// @townbuilder/runtime — Plugin load-order and override policy
// Manages plugin load ordering via topological sort with precedence levels,
// collision detection, and checkpoint/restore for safe plugin state rollback.

// ---- Plugin Precedence ----

/** Precedence levels for plugin load ordering. Higher values override lower. */
export enum PluginPrecedence {
  Base = 0,
  Dependency = 1,
  User = 2,
  Override = 3,
}

// ---- Interfaces ----

/** An entry describing a plugin in the load order. */
export interface PluginLoadEntry {
  /** Unique plugin identifier. */
  id: string;
  /** Precedence level for override resolution. */
  precedence: PluginPrecedence;
  /** Semantic version string. */
  version: string;
  /** IDs of plugins this one depends on. */
  dependencies: string[];
}

/** A snapshot of the load state at a point in time. */
export interface LoadCheckpoint {
  /** Unique checkpoint identifier. */
  id: number;
  /** Timestamp when the checkpoint was created (ms since epoch). */
  timestamp: number;
  /** Ordered list of plugin IDs that were loaded at checkpoint time. */
  loadedPlugins: string[];
}

// ---- Load Policy Manager ----

/** Manages plugin load ordering, override resolution, and state checkpoints. */
export class LoadPolicyManager {
  private loadOrder: PluginLoadEntry[] = [];
  private checkpoints: LoadCheckpoint[] = [];
  private nextCheckpointId: number = 1;

  constructor() {
    // Initialized with empty state
  }

  /** Add a plugin entry to the load order. */
  addPlugin(entry: PluginLoadEntry): void {
    this.loadOrder.push(entry);
  }

  /**
   * Resolve the final load order using topological sort by dependencies,
   * then stable-sorted by precedence within each dependency level.
   * Returns the ordered list of plugin entries.
   */
  resolveLoadOrder(): PluginLoadEntry[] {
    // Build lookup
    const byId = new Map<string, PluginLoadEntry>();
    for (const entry of this.loadOrder) {
      // If multiple entries with same id, highest precedence wins
      const existing = byId.get(entry.id);
      if (!existing || entry.precedence > existing.precedence) {
        byId.set(entry.id, entry);
      }
    }

    // Deduplicated entries
    const entries = Array.from(byId.values());

    // Topological sort via DFS
    const state = new Map<string, number>(); // 0=unvisited, 1=in-progress, 2=done
    for (const e of entries) {
      state.set(e.id, 0);
    }

    const sorted: PluginLoadEntry[] = [];

    const visit = (id: string): void => {
      const s = state.get(id);
      if (s === 2) return;
      if (s === 1) {
        throw new Error(`Circular dependency detected involving plugin "${id}"`);
      }

      const entry = byId.get(id);
      if (!entry) return; // external dep, skip

      state.set(id, 1);

      for (const dep of entry.dependencies) {
        visit(dep);
      }

      state.set(id, 2);
      sorted.push(entry);
    };

    // Visit in precedence order so that base plugins come first when no deps constrain order
    const byPrecedence = [...entries].sort((a, b) => a.precedence - b.precedence);
    for (const entry of byPrecedence) {
      visit(entry.id);
    }

    return sorted;
  }

  /**
   * Get the effective plugin entry for a given id.
   * When multiple entries share the same id, the one with the highest precedence wins.
   */
  getEffectivePlugin(id: string): PluginLoadEntry | undefined {
    let best: PluginLoadEntry | undefined;
    for (const entry of this.loadOrder) {
      if (entry.id === id) {
        if (!best || entry.precedence > best.precedence) {
          best = entry;
        }
      }
    }
    return best;
  }

  /**
   * Detect version collisions: plugins with the same id but different versions.
   * Returns an array of collision records.
   */
  detectCollisions(): Array<{ id: string; versions: string[] }> {
    const versionMap = new Map<string, Set<string>>();
    for (const entry of this.loadOrder) {
      const versions = versionMap.get(entry.id);
      if (versions) {
        versions.add(entry.version);
      } else {
        versionMap.set(entry.id, new Set([entry.version]));
      }
    }

    const collisions: Array<{ id: string; versions: string[] }> = [];
    for (const [id, versions] of versionMap) {
      if (versions.size > 1) {
        collisions.push({ id, versions: Array.from(versions).sort() });
      }
    }
    return collisions;
  }

  /**
   * Create a checkpoint of the current load state.
   * Returns the checkpoint for reference.
   */
  createCheckpoint(): LoadCheckpoint {
    const checkpoint: LoadCheckpoint = {
      id: this.nextCheckpointId++,
      timestamp: Date.now(),
      loadedPlugins: this.loadOrder.map((e) => e.id),
    };
    this.checkpoints.push(checkpoint);
    return checkpoint;
  }

  /**
   * Restore the load order to a previous checkpoint.
   * Returns true if the checkpoint was found and restored, false otherwise.
   */
  restoreCheckpoint(checkpointId: number): boolean {
    const checkpoint = this.checkpoints.find((cp) => cp.id === checkpointId);
    if (!checkpoint) return false;

    // Rebuild loadOrder to only contain plugins that were present at checkpoint time
    const allowedIds = new Set(checkpoint.loadedPlugins);
    this.loadOrder = this.loadOrder.filter((e) => allowedIds.has(e.id));

    // Remove any checkpoints created after the restored one
    this.checkpoints = this.checkpoints.filter((cp) => cp.id <= checkpointId);

    return true;
  }

  /** Get the current (unresolved) load order entries. */
  getLoadOrder(): PluginLoadEntry[] {
    return [...this.loadOrder];
  }

  /** Clear all entries and checkpoints. */
  clear(): void {
    this.loadOrder = [];
    this.checkpoints = [];
    this.nextCheckpointId = 1;
  }
}

// ---- Utility ----

/**
 * Compare two precedence values.
 * Returns negative if a < b, 0 if equal, positive if a > b.
 */
export function comparePrec(a: PluginPrecedence, b: PluginPrecedence): number {
  return a - b;
}
