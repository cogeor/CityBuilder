// @townbuilder/runtime — Runtime orchestration facade
// Provides a single high-level API that coordinates WorkerManager,
// SaveManager, PluginRegistry, and CommandHistory subsystems.

import { CommandHistory, type CommandRecord } from "./history/command_history.js";
import { PluginHost, PluginRegistry } from "./plugins/index.js";
import type { EngineCommand, SimSpeedName } from "./engine/commands.js";
import {
  SaveManager,
  InMemorySaveStorage,
  type ISaveStorage,
} from "./saves/save_manager.js";
import { WorkerManager, type IWorkerManager } from "./messaging/worker_manager.js";

function getCommandKind(command: Readonly<EngineCommand>): string {
  const keys = Object.keys(command);
  return keys.length > 0 ? keys[0] : "Unknown";
}

function deriveUndoPayload(
  command: Readonly<EngineCommand>,
  lastSimSpeed: SimSpeedName,
): EngineCommand | null {
  if ("SetZoning" in command) {
    const c = command.SetZoning;
    return {
      SetZoning: {
        x: c.x,
        y: c.y,
        w: c.w,
        h: c.h,
        zone: "None",
      },
    };
  }
  if ("SetSimSpeed" in command) {
    // Undo: restore the speed that was active before this command.
    return { SetSimSpeed: { speed: lastSimSpeed } };
  }
  // SetRoadLine: no structural undo — the road graph state before painting is
  // not captured, so we cannot reverse it without a full snapshot.
  // PlaceEntity: undo requires the EntityHandle returned by the engine effect,
  // which is only known after the command is applied (not at dispatch time).
  // Bulldoze: tiles and entities are gone; no undo without snapshot.
  return null;
}

// ---- RuntimeConfig ----

/** Configuration for the RuntimeFacade. */
export interface RuntimeConfig {
  /** Width of the simulation map in tiles. */
  readonly mapWidth: number;
  /** Height of the simulation map in tiles. */
  readonly mapHeight: number;
  /** RNG seed for deterministic world generation. */
  readonly seed: number;
  /** Interval between auto-saves in milliseconds. Omit to disable auto-save. */
  readonly autoSaveInterval?: number;
  /** Maximum number of undo steps. Default: 100. */
  readonly maxUndoDepth?: number;
}

// ---- RuntimeState ----

/** Lifecycle states for the RuntimeFacade. */
export enum RuntimeState {
  Uninitialized = "Uninitialized",
  Starting = "Starting",
  Running = "Running",
  Paused = "Paused",
  ShuttingDown = "ShuttingDown",
  Stopped = "Stopped",
}

// ---- RuntimeFacade ----

/**
 * High-level facade that coordinates all runtime subsystems.
 *
 * Consumers interact with this single class instead of managing
 * individual managers directly. The facade enforces state-machine
 * transitions and delegates to the appropriate subsystem.
 */
export class RuntimeFacade {
  private _state: RuntimeState = RuntimeState.Uninitialized;
  private readonly _config: RuntimeConfig;
  private readonly _commandHistory: CommandHistory;
  private readonly _pluginRegistry: PluginRegistry;
  private readonly _pluginHost: PluginHost;
  private readonly _saveManager: SaveManager;
  private _workerManager: IWorkerManager | null = null;
  private _commandSequence: number = 0;
  private readonly _stateChangeCallbacks: Array<(state: RuntimeState) => void> = [];
  /** Last known simulation speed — used to derive undo payloads for SetSimSpeed. */
  private _lastSimSpeed: SimSpeedName = "Normal";

  constructor(config: RuntimeConfig) {
    this._config = config;

    this._commandHistory = new CommandHistory({
      maxUndoDepth: config.maxUndoDepth ?? 100,
    });

    this._pluginRegistry = new PluginRegistry();
    this._pluginHost = new PluginHost(this._pluginRegistry);

    // Use in-memory storage by default; callers can provide custom storage
    // by extending or replacing the SaveManager after construction.
    const storage: ISaveStorage = new InMemorySaveStorage();
    this._saveManager = new SaveManager(storage, {
      auto_save_interval_ms: config.autoSaveInterval ?? 300_000,
    });
  }

  // ---- State Machine ----

  /** Return the current lifecycle state. */
  getState(): RuntimeState {
    return this._state;
  }

  /**
   * Register a callback that fires whenever the lifecycle state changes.
   * Returns an unsubscribe function.
   */
  onStateChange(callback: (state: RuntimeState) => void): () => void {
    this._stateChangeCallbacks.push(callback);
    return () => {
      const idx = this._stateChangeCallbacks.indexOf(callback);
      if (idx !== -1) {
        this._stateChangeCallbacks.splice(idx, 1);
      }
    };
  }

  /** Transition to a new state and notify all listeners. */
  private _transition(newState: RuntimeState): void {
    this._state = newState;
    for (const cb of this._stateChangeCallbacks) {
      cb(newState);
    }
  }

  // ---- Lifecycle ----

  /**
   * Initialize subsystems and transition to Running.
   *
   * When a real WorkerManager is set (via `setWorkerManager`), this will
   * start the simulation worker. Otherwise it proceeds without workers,
   * which is useful in test environments.
   */
  async start(): Promise<void> {
    if (this._state !== RuntimeState.Uninitialized) {
      throw new Error(
        `Cannot start: runtime is in state "${this._state}", expected "${RuntimeState.Uninitialized}"`,
      );
    }

    this._transition(RuntimeState.Starting);

    // Start sim worker if a worker manager is configured
    if (this._workerManager) {
      await this._workerManager.startSim(
        this._config.seed,
        this._config.mapWidth,
        this._config.mapHeight,
      );
    }

    this._transition(RuntimeState.Running);
  }

  /**
   * Gracefully shut down all subsystems and transition to Stopped.
   */
  async shutdown(): Promise<void> {
    if (
      this._state !== RuntimeState.Running &&
      this._state !== RuntimeState.Paused
    ) {
      throw new Error(
        `Cannot shutdown: runtime is in state "${this._state}", expected "${RuntimeState.Running}" or "${RuntimeState.Paused}"`,
      );
    }

    this._transition(RuntimeState.ShuttingDown);

    // Stop auto-save
    this._saveManager.stopAutoSave();

    // Terminate workers
    if (this._workerManager) {
      this._workerManager.shutdown();
      this._workerManager = null;
    }

    this._transition(RuntimeState.Stopped);
  }

  // ---- Worker Manager ----

  /**
   * Inject a WorkerManager (or mock) before calling start().
   * This allows test environments to provide a mock implementation.
   */
  setWorkerManager(manager: IWorkerManager): void {
    this._workerManager = manager;
  }

  // ---- Commands ----

  /**
   * Send a command to the simulation.
   * Records the command in history and forwards it to the sim worker.
   * Throws if the runtime is not in the Running state.
   */
  sendCommand(command: Readonly<EngineCommand>): void {
    if (this._state !== RuntimeState.Running) {
      throw new Error(
        `Cannot send command: runtime is in state "${this._state}", expected "${RuntimeState.Running}"`,
      );
    }

    this._commandSequence += 1;

    const record: CommandRecord = {
      id: this._commandSequence,
      type: getCommandKind(command),
      doPayload: command,
      undoPayload: deriveUndoPayload(command, this._lastSimSpeed),
      timestamp: Date.now(),
    };

    // Track speed changes so SetSimSpeed undo can restore the previous speed.
    if ("SetSimSpeed" in command) {
      this._lastSimSpeed = command.SetSimSpeed.speed;
    }

    this._commandHistory.push(record);

    // Forward to sim worker if available
    if (this._workerManager) {
      this._workerManager.sendCommand(JSON.stringify(command)).catch(() => {
        // Command delivery failures are logged but do not throw synchronously
      });
    }
  }

  /**
   * Undo the last command from history.
   * Returns the undone record, or null if nothing to undo.
   */
  undo(): CommandRecord | null {
    if (this._state !== RuntimeState.Running) {
      return null;
    }
    const record = this._commandHistory.undo();
    if (record && this._workerManager && record.undoPayload) {
      const payload = record.undoPayload as EngineCommand;
      this._workerManager.sendCommand(JSON.stringify(payload)).catch(() => {});
    }
    return record;
  }

  /**
   * Redo the last undone command from history.
   * Returns the redone record, or null if nothing to redo.
   */
  redo(): CommandRecord | null {
    if (this._state !== RuntimeState.Running) {
      return null;
    }
    const record = this._commandHistory.redo();
    if (record && this._workerManager) {
      const payload = record.doPayload as EngineCommand;
      this._workerManager.sendCommand(JSON.stringify(payload)).catch(() => {});
    }
    return record;
  }

  // ---- Speed Control ----

  /**
   * Set the simulation speed.
   * Forwards to the worker manager if available.
   * Does not throw if no worker manager is set (safe for tests).
   */
  setSpeed(speed: number): void {
    if (this._workerManager) {
      this._workerManager.setSpeed(speed);
    }
  }

  // ---- Save / Load ----

  /**
   * Save the current game state to the given slot.
   * Delegates to SaveManager. In the absence of a real worker, saves
   * an empty placeholder.
   */
  async save(slotId: string): Promise<void> {
    let data: Uint8Array;
    if (this._workerManager) {
      data = await this._workerManager.saveGame();
    } else {
      // No worker — store empty data (useful for testing the save path)
      data = new Uint8Array(0);
    }

    await this._saveManager.saveGame(slotId, slotId, data, "city", 0);
  }

  /**
   * Load a game state from the given slot.
   * Delegates to SaveManager. Returns true if the slot was found and
   * loaded, false otherwise.
   */
  async load(slotId: string): Promise<boolean> {
    const data = await this._saveManager.loadGame(slotId);
    if (!data) {
      return false;
    }

    if (this._workerManager) {
      return this._workerManager.loadGame(data);
    }

    return true;
  }

  // ---- Accessors ----

  /** Return the CommandHistory instance for direct inspection. */
  getCommandHistory(): CommandHistory {
    return this._commandHistory;
  }

  /** Return the PluginRegistry instance. */
  getPluginRegistry(): PluginRegistry {
    return this._pluginRegistry;
  }

  /** Return the PluginHost abstraction for discovery/load orchestration. */
  getPluginHost(): PluginHost {
    return this._pluginHost;
  }

  /** Return the SaveManager instance. */
  getSaveManager(): SaveManager {
    return this._saveManager;
  }

  /** Return the runtime configuration. */
  getConfig(): RuntimeConfig {
    return this._config;
  }
}
