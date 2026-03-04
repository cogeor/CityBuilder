// @townbuilder/runtime — Sim worker: WASM tick loop in Web Worker
// Loads the simulation engine (WASM or mock) and drives the tick loop,
// communicating with the main thread via structured messages.

import { MessageType, WIRE_VERSION } from "../messaging/types.js";

// ---- SimSpeed Enum ----

/** Simulation speed multiplier. Maps to ticks-per-interval. */
export enum SimSpeed {
  PAUSED = 0,
  NORMAL = 1,
  FAST = 2,
  ULTRA = 4,
}

// ---- Interfaces ----

/** Result produced by a single simulation tick. */
export interface TickResult {
  readonly tick: number;
  readonly events: string[];
  readonly population: number;
  readonly treasury: number;
}

/** Internal state of the simulation worker. */
export interface SimWorkerState {
  game: any; // GameHandle from WASM — typed as any until wasm-bindgen types exist
  speed: SimSpeed;
  running: boolean;
  tickInterval: number | null;
}

// ---- Constants ----

/** Base tick interval in milliseconds. At NORMAL speed this gives 20 ticks/sec. */
export const TICK_INTERVAL_MS = 50;

// ---- MockGameHandle ----

/**
 * Lightweight stand-in for the real WASM GameHandle.
 * Used in tests and when the WASM module is not available.
 */
export class MockGameHandle {
  tick_counter = 0;
  pop = 100;
  treasury = 10000;

  tick(): TickResult {
    this.tick_counter += 1;
    return {
      tick: this.tick_counter,
      events: [],
      population: this.pop,
      treasury: this.treasury,
    };
  }

  apply_command_json(_json: string): boolean {
    // Mock: commands are accepted but have no effect
    return true;
  }

  save(): Uint8Array {
    // Mock: return an empty save blob
    return new Uint8Array(0);
  }

  load(_data: Uint8Array): boolean {
    // Mock: loads always succeed
    return true;
  }

  get_tick(): number {
    return this.tick_counter;
  }

  get_population(): number {
    return this.pop;
  }

  get_treasury(): number {
    return this.treasury;
  }
}

// ---- SimWorker Class ----

/**
 * Manages the simulation tick loop inside a Web Worker.
 *
 * Message routing:
 *   main thread  -->  handleMessage()  -->  postResponse()  -->  main thread
 *
 * The class is decoupled from the global `self` / `postMessage` so it can be
 * tested without a real Worker environment.
 */
export class SimWorker {
  state: SimWorkerState;

  /**
   * Optional callback used to send messages back to the main thread.
   * In a real worker this is wired to `self.postMessage`.
   * In tests it can be replaced with a spy.
   */
  postResponse: (msg: any) => void;

  constructor(postResponse?: (msg: any) => void) {
    this.state = {
      game: null,
      speed: SimSpeed.PAUSED,
      running: false,
      tickInterval: null,
    };
    this.postResponse = postResponse ?? ((_msg: any) => {});
  }

  // ---- Message Router ----

  /** Dispatch an incoming message to the appropriate handler. */
  handleMessage(msg: any): void {
    if (!msg || typeof msg.type !== "string") return;

    switch (msg.type) {
      case MessageType.HANDSHAKE:
        this.postResponse({
          type: MessageType.HANDSHAKE_ACK,
          wire_version: WIRE_VERSION,
          timestamp: Date.now(),
        });
        break;

      case MessageType.COMMAND:
        this.applyCommand(msg.command_json, msg.sequence_id);
        break;

      case MessageType.SET_SPEED:
        this.setSpeed(msg.speed as SimSpeed);
        break;

      case MessageType.PAUSE:
        this.setSpeed(SimSpeed.PAUSED);
        break;

      case MessageType.RESUME:
        this.setSpeed(SimSpeed.NORMAL);
        break;

      case MessageType.SAVE_REQUEST:
        this._handleSaveRequest(msg.slot);
        break;

      case MessageType.LOAD_REQUEST:
        this._handleLoadRequest(msg.slot, msg.data);
        break;

      default:
        // Unknown message types are silently ignored.
        break;
    }
  }

  // ---- Game Lifecycle ----

  /**
   * Initialise the simulation game with the given parameters.
   * Uses MockGameHandle until real WASM bindings are available.
   */
  initGame(_seed: number, _width: number, _height: number): void {
    // TODO: Replace with real WASM init:
    //   const wasm = await import("@townbuilder/engine-wasm");
    //   this.state.game = wasm.GameHandle.new(seed, width, height);
    this.state.game = new MockGameHandle();
    this.state.running = true;
  }

  // ---- Tick Loop ----

  /**
   * Start the periodic tick loop. Each interval fires `speed` ticks
   * (so FAST = 2 ticks per interval, ULTRA = 4).
   */
  startTickLoop(): void {
    this.stopTickLoop(); // clear any existing loop

    if (this.state.speed === SimSpeed.PAUSED) return;

    const intervalMs = TICK_INTERVAL_MS;

    // Use `setInterval` — available in Worker global scope.
    // We store the id so we can clear it later.
    this.state.tickInterval = setInterval(() => {
      const ticksThisInterval = this.state.speed as number;
      for (let i = 0; i < ticksThisInterval; i++) {
        const result = this.executeTick();
        this.postResponse({
          type: MessageType.TICK_OUTPUT,
          tick: result.tick,
          events_json: JSON.stringify(result.events),
          population: result.population,
          treasury: result.treasury,
        });
      }
    }, intervalMs) as unknown as number;
  }

  /** Stop the tick loop if one is running. */
  stopTickLoop(): void {
    if (this.state.tickInterval !== null) {
      clearInterval(this.state.tickInterval);
      this.state.tickInterval = null;
    }
  }

  // ---- Speed Control ----

  /** Change simulation speed and restart the tick loop accordingly. */
  setSpeed(speed: SimSpeed): void {
    this.state.speed = speed;
    if (this.state.running) {
      if (speed === SimSpeed.PAUSED) {
        this.stopTickLoop();
      } else {
        this.startTickLoop();
      }
    }
  }

  // ---- Simulation Operations ----

  /** Execute a single simulation tick and return the result. */
  executeTick(): TickResult {
    if (!this.state.game) {
      return { tick: 0, events: [], population: 0, treasury: 0 };
    }
    return this.state.game.tick();
  }

  /** Apply a command to the simulation and post the result back. */
  applyCommand(commandJson: string, sequenceId: number): void {
    if (!this.state.game) {
      this.postResponse({
        type: MessageType.COMMAND_RESULT,
        success: false,
        error: "Game not initialised",
        sequence_id: sequenceId,
      });
      return;
    }

    try {
      const success = this.state.game.apply_command_json(commandJson);
      this.postResponse({
        type: MessageType.COMMAND_RESULT,
        success,
        sequence_id: sequenceId,
      });
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      this.postResponse({
        type: MessageType.COMMAND_RESULT,
        success: false,
        error: errorMsg,
        sequence_id: sequenceId,
      });
    }
  }

  /** Serialise the current game state. */
  save(): Uint8Array {
    if (!this.state.game) {
      return new Uint8Array(0);
    }
    return this.state.game.save();
  }

  /** Load a previously saved game state. */
  load(data: Uint8Array): boolean {
    if (!this.state.game) {
      return false;
    }
    return this.state.game.load(data);
  }

  /** Return a snapshot of key simulation statistics. */
  getStats(): { tick: number; population: number; treasury: number } {
    if (!this.state.game) {
      return { tick: 0, population: 0, treasury: 0 };
    }
    return {
      tick: this.state.game.get_tick(),
      population: this.state.game.get_population(),
      treasury: this.state.game.get_treasury(),
    };
  }

  // ---- Private Helpers ----

  private _handleSaveRequest(slot: string): void {
    try {
      const data = this.save();
      // Convert to base64 string for transmission
      const encoded = _uint8ArrayToBase64(data);
      this.postResponse({
        type: MessageType.SAVE_RESPONSE,
        success: true,
        data: encoded,
      });
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      this.postResponse({
        type: MessageType.SAVE_RESPONSE,
        success: false,
        error: errorMsg,
      });
    }
  }

  private _handleLoadRequest(slot: string, dataStr: string): void {
    try {
      const data = _base64ToUint8Array(dataStr ?? "");
      const success = this.load(data);
      this.postResponse({
        type: MessageType.LOAD_RESPONSE,
        success,
      });
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      this.postResponse({
        type: MessageType.LOAD_RESPONSE,
        success: false,
        error: errorMsg,
      });
    }
  }
}

// ---- Base64 Utilities ----

/** Convert a Uint8Array to a base64 string (works in Worker scope). */
export function _uint8ArrayToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/** Convert a base64 string back to a Uint8Array. */
export function _base64ToUint8Array(base64: string): Uint8Array {
  if (base64.length === 0) return new Uint8Array(0);
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

// ---- Worker Bootstrap ----

// When running in a real Web Worker context, wire up the message handler.
// This block is guarded so it doesn't execute during unit tests.
declare const self: any;

if (
  typeof self !== "undefined" &&
  typeof self.postMessage === "function" &&
  typeof self.addEventListener === "function"
) {
  const worker = new SimWorker((msg: any) => self.postMessage(msg));
  // Auto-init game on startup (seed=0 until handshake provides real params)
  worker.initGame(0, 128, 128);

  self.addEventListener("message", (event: MessageEvent) => {
    worker.handleMessage(event.data);
  });
}
