// @townbuilder/runtime — Worker manager: main-thread orchestrator
// Manages sim and render Web Workers, routing messages between them
// and the main thread.

import {
  MessageType,
  WIRE_VERSION,
  type CommandResultMessage,
  type TickOutputMessage,
  type PickResponseMessage,
  type SaveResponseMessage,
  type LoadResponseMessage,
  type WorkerMessage,
} from "./types.js";

// ---- MockWorker ----

/**
 * Lightweight mock for the Worker API, used in unit tests where real
 * Web Workers are not available.
 *
 * Stores posted messages for inspection and supports addEventListener.
 */
export class MockWorker {
  /** All messages sent via postMessage, in order. */
  readonly postedMessages: any[] = [];

  /** Registered event listeners by event type. */
  private _listeners: Map<string, Function[]> = new Map();

  /** Whether terminate() has been called. */
  terminated = false;

  postMessage(msg: any): void {
    this.postedMessages.push(msg);
  }

  addEventListener(type: string, listener: Function): void {
    const existing = this._listeners.get(type) ?? [];
    existing.push(listener);
    this._listeners.set(type, existing);
  }

  removeEventListener(type: string, listener: Function): void {
    const existing = this._listeners.get(type);
    if (existing) {
      const idx = existing.indexOf(listener);
      if (idx !== -1) existing.splice(idx, 1);
    }
  }

  terminate(): void {
    this.terminated = true;
  }

  /**
   * Simulate an incoming message event from the worker.
   * This dispatches to all registered "message" listeners.
   */
  simulateMessage(data: any): void {
    const listeners = this._listeners.get("message") ?? [];
    const event = { data } as MessageEvent;
    for (const listener of listeners) {
      listener(event);
    }
  }
}

// ---- WorkerManagerState ----

/** Internal state of the WorkerManager. */
export interface WorkerManagerState {
  simWorker: Worker | null;
  renderWorker: Worker | null;
  simReady: boolean;
  renderReady: boolean;
  speed: number;
  commandSequence: number;
  pendingCommands: Map<number, { resolve: Function; reject: Function }>;
  tickCallbacks: Array<(output: TickOutputMessage) => void>;
  pickCallbacks: Array<(result: PickResponseMessage) => void>;
}

// ---- IWorkerManager ----

/** Public interface for the WorkerManager. */
export interface IWorkerManager {
  startSim(seed: number, width: number, height: number): Promise<void>;
  startRenderer(canvas: any): Promise<void>;
  sendCommand(commandJson: string): Promise<CommandResultMessage>;
  setSpeed(speed: number): void;
  pause(): void;
  resume(): void;
  saveGame(): Promise<Uint8Array>;
  loadGame(data: Uint8Array): Promise<boolean>;
  onTick(callback: (output: TickOutputMessage) => void): void;
  onPick(callback: (result: PickResponseMessage) => void): void;
  shutdown(): void;
}

// ---- WorkerManager ----

/**
 * Main-thread orchestrator for sim and render Web Workers.
 *
 * Owns the lifecycle of both workers, routes incoming messages to
 * the appropriate callbacks and pending-command promises, and provides
 * a Promise-based API for sending commands and save/load operations.
 */
export class WorkerManager implements IWorkerManager {
  state: WorkerManagerState;

  constructor() {
    this.state = {
      simWorker: null,
      renderWorker: null,
      simReady: false,
      renderReady: false,
      speed: 0,
      commandSequence: 0,
      pendingCommands: new Map(),
      tickCallbacks: [],
      pickCallbacks: [],
    };
  }

  // ---- Lifecycle ----

  /**
   * Start the simulation worker with the given world parameters.
   * Sends a HANDSHAKE and waits for HANDSHAKE_ACK before resolving.
   */
  async startSim(seed: number, width: number, height: number): Promise<void> {
    const worker = this.state.simWorker;
    if (!worker) {
      throw new Error("No sim worker assigned");
    }

    return new Promise<void>((resolve, reject) => {
      // Temporarily listen for handshake ack
      const onMessage = (event: MessageEvent) => {
        const msg = event.data;
        if (msg && msg.type === MessageType.HANDSHAKE_ACK) {
          worker.removeEventListener("message", onMessage);
          if (msg.success === false) {
            reject(new Error(msg.error ?? "Sim worker handshake failed"));
            return;
          }
          this.state.simReady = true;
          resolve();
        }
      };
      worker.addEventListener("message", onMessage);

      // Wire up the persistent message handler
      worker.addEventListener("message", (event: MessageEvent) => {
        this.onSimMessage(event);
      });

      // Send handshake
      worker.postMessage({
        type: MessageType.HANDSHAKE,
        wire_version: WIRE_VERSION,
        timestamp: Date.now(),
        seed,
        width,
        height,
      });
    });
  }

  /**
   * Start the render worker with an OffscreenCanvas.
   * Sends a HANDSHAKE and waits for HANDSHAKE_ACK before resolving.
   */
  async startRenderer(canvas: any): Promise<void> {
    const worker = this.state.renderWorker;
    if (!worker) {
      throw new Error("No render worker assigned");
    }

    return new Promise<void>((resolve, reject) => {
      const onMessage = (event: MessageEvent) => {
        const msg = event.data;
        if (msg && msg.type === MessageType.HANDSHAKE_ACK) {
          worker.removeEventListener("message", onMessage);
          if (msg.success === false) {
            reject(new Error(msg.error ?? "Render worker handshake failed"));
            return;
          }
          this.state.renderReady = true;
          resolve();
        }
      };
      worker.addEventListener("message", onMessage);

      // Wire up persistent handler
      worker.addEventListener("message", (event: MessageEvent) => {
        this.onRenderMessage(event);
      });

      worker.postMessage({
        type: MessageType.HANDSHAKE,
        wire_version: WIRE_VERSION,
        timestamp: Date.now(),
      });
    });
  }

  // ---- Commands ----

  /**
   * Send a command to the sim worker and return a Promise that resolves
   * with the CommandResultMessage. Uses incrementing sequence IDs to
   * correlate requests with responses.
   */
  async sendCommand(commandJson: string): Promise<CommandResultMessage> {
    const worker = this.state.simWorker;
    if (!worker) {
      throw new Error("No sim worker available");
    }

    this.state.commandSequence += 1;
    const seqId = this.state.commandSequence;

    return new Promise<CommandResultMessage>((resolve, reject) => {
      this.state.pendingCommands.set(seqId, { resolve, reject });
      worker.postMessage({
        type: MessageType.COMMAND,
        command_json: commandJson,
        sequence_id: seqId,
      });
    });
  }

  // ---- Speed Control ----

  /** Set simulation speed. Sends SET_SPEED to the sim worker. */
  setSpeed(speed: number): void {
    this.state.speed = speed;
    this.state.simWorker?.postMessage({
      type: MessageType.SET_SPEED,
      speed,
    });
  }

  /** Pause the simulation. */
  pause(): void {
    this.state.speed = 0;
    this.state.simWorker?.postMessage({
      type: MessageType.PAUSE,
    });
  }

  /** Resume the simulation at the previously set speed (defaults to 1). */
  resume(): void {
    if (this.state.speed === 0) {
      this.state.speed = 1;
    }
    this.state.simWorker?.postMessage({
      type: MessageType.RESUME,
    });
  }

  // ---- Save / Load ----

  /** Request a save from the sim worker. Returns the serialised data. */
  async saveGame(): Promise<Uint8Array> {
    const worker = this.state.simWorker;
    if (!worker) {
      throw new Error("No sim worker available");
    }

    return new Promise<Uint8Array>((resolve, reject) => {
      const onMessage = (event: MessageEvent) => {
        const msg = event.data as SaveResponseMessage;
        if (msg && msg.type === MessageType.SAVE_RESPONSE) {
          if (msg.success && msg.data) {
            // Decode base64 data back to Uint8Array
            const binary = atob(msg.data);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
              bytes[i] = binary.charCodeAt(i);
            }
            resolve(bytes);
          } else {
            reject(new Error(msg.error ?? "Save failed"));
          }
        }
      };
      worker.addEventListener("message", onMessage);

      worker.postMessage({
        type: MessageType.SAVE_REQUEST,
        slot: "manual",
      });
    });
  }

  /** Load a previously saved game state into the sim worker. */
  async loadGame(data: Uint8Array): Promise<boolean> {
    const worker = this.state.simWorker;
    if (!worker) {
      throw new Error("No sim worker available");
    }

    return new Promise<boolean>((resolve) => {
      const onMessage = (event: MessageEvent) => {
        const msg = event.data as LoadResponseMessage;
        if (msg && msg.type === MessageType.LOAD_RESPONSE) {
          resolve(msg.success);
        }
      };
      worker.addEventListener("message", onMessage);

      // Encode data as base64 for transmission
      let binary = "";
      for (let i = 0; i < data.length; i++) {
        binary += String.fromCharCode(data[i]);
      }
      const encoded = btoa(binary);

      worker.postMessage({
        type: MessageType.LOAD_REQUEST,
        slot: "manual",
        data: encoded,
      });
    });
  }

  // ---- Subscriptions ----

  /** Register a callback to be invoked on every sim tick output. */
  onTick(callback: (output: TickOutputMessage) => void): void {
    this.state.tickCallbacks.push(callback);
  }

  /** Register a callback to be invoked on pick responses. */
  onPick(callback: (result: PickResponseMessage) => void): void {
    this.state.pickCallbacks.push(callback);
  }

  // ---- Message Handlers ----

  /**
   * Route a message received from the sim worker.
   * Dispatches to pending-command promises, tick callbacks, etc.
   */
  onSimMessage(event: MessageEvent): void {
    const msg = event.data;
    if (!msg || typeof msg.type !== "string") return;

    switch (msg.type) {
      case MessageType.TICK_OUTPUT: {
        const tickMsg = msg as TickOutputMessage;
        for (const cb of this.state.tickCallbacks) {
          cb(tickMsg);
        }
        break;
      }

      case MessageType.COMMAND_RESULT: {
        const resultMsg = msg as CommandResultMessage;
        const pending = this.state.pendingCommands.get(resultMsg.sequence_id);
        if (pending) {
          this.state.pendingCommands.delete(resultMsg.sequence_id);
          pending.resolve(resultMsg);
        }
        break;
      }

      case MessageType.PICK_RESPONSE: {
        const pickMsg = msg as PickResponseMessage;
        for (const cb of this.state.pickCallbacks) {
          cb(pickMsg);
        }
        break;
      }

      default:
        // Other message types (diffs, notifications) may be handled in future.
        break;
    }
  }

  /**
   * Route a message received from the render worker.
   * Currently handles PICK_RESPONSE.
   */
  onRenderMessage(event: MessageEvent): void {
    const msg = event.data;
    if (!msg || typeof msg.type !== "string") return;

    switch (msg.type) {
      case MessageType.PICK_RESPONSE: {
        const pickMsg = msg as PickResponseMessage;
        for (const cb of this.state.pickCallbacks) {
          cb(pickMsg);
        }
        break;
      }

      default:
        break;
    }
  }

  // ---- Shutdown ----

  /** Terminate both workers and clean up state. */
  shutdown(): void {
    if (this.state.simWorker) {
      this.state.simWorker.terminate();
      this.state.simWorker = null;
    }
    if (this.state.renderWorker) {
      this.state.renderWorker.terminate();
      this.state.renderWorker = null;
    }
    this.state.simReady = false;
    this.state.renderReady = false;
    this.state.pendingCommands.clear();
    this.state.tickCallbacks = [];
    this.state.pickCallbacks = [];
  }
}
