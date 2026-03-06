// @townbuilder/runtime — Undo/redo foundation using Command + Memento history.
// Provides a CommandHistory class with bounded undo/redo stacks and periodic
// snapshot checkpoints for bounded replay cost.

// ---- CommandRecord Interface ----

/** A single command record in the undo/redo history. */
export interface CommandRecord {
  readonly id: number;
  readonly type: string;
  readonly doPayload: unknown;
  readonly undoPayload: unknown;
  readonly timestamp: number;
}

// ---- Snapshot Interface ----

/** A periodic checkpoint snapshot for bounded replay cost. */
export interface Snapshot {
  readonly id: number;
  readonly tick: number;
  readonly data: Uint8Array;
}

// ---- CommandHistoryConfig Interface ----

/** Configuration for CommandHistory behaviour. */
export interface CommandHistoryConfig {
  /** Maximum number of undo steps. Default: 100. */
  readonly maxUndoDepth: number;
  /** Create a snapshot every N commands. Default: 50. */
  readonly snapshotInterval: number;
}

/** Default configuration values. */
const DEFAULT_CONFIG: CommandHistoryConfig = {
  maxUndoDepth: 100,
  snapshotInterval: 50,
};

// ---- CommandHistory ----

/** Main-thread undo/redo history manager with periodic snapshots. */
export class CommandHistory {
  readonly config: CommandHistoryConfig;

  private _undoStack: CommandRecord[] = [];
  private _redoStack: CommandRecord[] = [];
  private _snapshots: Snapshot[] = [];
  private _commandsSinceSnapshot: number = 0;
  private _snapshotCallback: (() => Snapshot) | null = null;

  constructor(config?: Partial<CommandHistoryConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Register a callback that produces a snapshot of the current state.
   * When set, a snapshot is automatically created every `snapshotInterval` commands.
   */
  setSnapshotCallback(cb: () => Snapshot): void {
    this._snapshotCallback = cb;
  }

  /**
   * Push a command record onto the undo stack.
   * Clears the redo stack (forking history).
   * Enforces maxUndoDepth by dropping the oldest record when exceeded.
   * Triggers auto-snapshot at the configured interval.
   */
  push(record: CommandRecord): void {
    this._undoStack.push(record);
    this._redoStack.length = 0;

    // Enforce maxUndoDepth — drop oldest when exceeded
    if (this._undoStack.length > this.config.maxUndoDepth) {
      this._undoStack.shift();
    }

    // Auto-snapshot at configured interval
    this._commandsSinceSnapshot++;
    if (
      this._snapshotCallback !== null &&
      this._commandsSinceSnapshot >= this.config.snapshotInterval
    ) {
      const snapshot = this._snapshotCallback();
      this.addSnapshot(snapshot);
      this._commandsSinceSnapshot = 0;
    }
  }

  /** Pop the most recent command from undo stack and push it to redo stack. */
  undo(): CommandRecord | null {
    const record = this._undoStack.pop();
    if (record === undefined) {
      return null;
    }
    this._redoStack.push(record);
    return record;
  }

  /** Pop the most recent command from redo stack and push it to undo stack. */
  redo(): CommandRecord | null {
    const record = this._redoStack.pop();
    if (record === undefined) {
      return null;
    }
    this._undoStack.push(record);
    return record;
  }

  /** Whether the undo stack has any records. */
  canUndo(): boolean {
    return this._undoStack.length > 0;
  }

  /** Whether the redo stack has any records. */
  canRedo(): boolean {
    return this._redoStack.length > 0;
  }

  /**
   * Store a snapshot checkpoint.
   * Trims old snapshots to keep at most maxUndoDepth / snapshotInterval + 1 snapshots.
   */
  addSnapshot(snapshot: Snapshot): void {
    this._snapshots.push(snapshot);

    // Keep a reasonable number of snapshots — roughly one per interval within undo depth
    const maxSnapshots = Math.max(
      1,
      Math.ceil(this.config.maxUndoDepth / this.config.snapshotInterval) + 1,
    );
    while (this._snapshots.length > maxSnapshots) {
      this._snapshots.shift();
    }
  }

  /** Return the most recently stored snapshot, or null if none exist. */
  getLatestSnapshot(): Snapshot | null {
    if (this._snapshots.length === 0) {
      return null;
    }
    return this._snapshots[this._snapshots.length - 1];
  }

  /** Reset all stacks and snapshots. */
  clear(): void {
    this._undoStack.length = 0;
    this._redoStack.length = 0;
    this._snapshots.length = 0;
    this._commandsSinceSnapshot = 0;
  }

  /**
   * Remove a specific command from the undo stack by ID.
   * Used to roll back a pushed record when worker delivery fails.
   * Only searches the undo stack (delivery failure implies the command was never applied).
   */
  removeById(id: number): boolean {
    const idx = this._undoStack.findIndex((r) => r.id === id);
    if (idx === -1) return false;
    this._undoStack.splice(idx, 1);
    this._commandsSinceSnapshot = Math.max(0, this._commandsSinceSnapshot - 1);
    return true;
  }

  /** Number of commands in the undo stack. */
  getUndoCount(): number {
    return this._undoStack.length;
  }

  /** Number of commands in the redo stack. */
  getRedoCount(): number {
    return this._redoStack.length;
  }
}
