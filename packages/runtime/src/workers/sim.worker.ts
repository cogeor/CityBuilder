// @townbuilder/runtime -- Sim worker: WASM-authoritative tick loop
// TS worker owns orchestration/messaging only. World simulation lives in Rust/WASM.

import { MessageType, WIRE_VERSION } from "../messaging/types.js";

export enum SimSpeed {
  PAUSED = 0,
  NORMAL = 1,
  FAST = 2,
  ULTRA = 4,
}

export interface TickResult {
  readonly tick: number;
  readonly events: string[];
  readonly population: number;
  readonly treasury: number;
}

interface WasmGameHandleLike {
  tick(): string | TickResult;
  apply_command_json(json: string): string | boolean;
  save(): Uint8Array;
  get_tick(): number;
  get_population(): number;
  get_treasury(): number;
}

interface WasmModuleLike {
  GameHandle?: {
    new?: (seed: number, width: number, height: number) => WasmGameHandleLike;
  };
  load_game?: (data: Uint8Array) => WasmGameHandleLike;
}

type WasmModuleLoader = () => Promise<WasmModuleLike>;

export interface SimWorkerState {
  game: WasmGameHandleLike | null;
  wasm: WasmModuleLike | null;
  speed: SimSpeed;
  running: boolean;
  tickInterval: number | null;
}

export const TICK_INTERVAL_MS = 50;

function parseTickResult(raw: string | TickResult): TickResult {
  if (typeof raw !== "string") {
    return raw;
  }
  const parsed = JSON.parse(raw) as {
    tick?: number;
    population?: number;
    treasury?: number;
    event_count?: number;
    error?: string;
  };
  if (parsed.error) {
    throw new Error(parsed.error);
  }
  return {
    tick: parsed.tick ?? 0,
    events: [],
    population: parsed.population ?? 0,
    treasury: parsed.treasury ?? 0,
  };
}

function parseCommandResult(raw: string | boolean): { success: boolean; error?: string } {
  if (typeof raw === "boolean") {
    return { success: raw };
  }
  const parsed = JSON.parse(raw) as { ok?: string; error?: string };
  if (parsed.error) {
    return { success: false, error: parsed.error };
  }
  return { success: typeof parsed.ok === "string" };
}

async function loadWasmModule(): Promise<WasmModuleLike> {
  const globalPath = (globalThis as { __TOWNBUILDER_ENGINE_WASM_PATH__?: string })
    .__TOWNBUILDER_ENGINE_WASM_PATH__;
  const modulePath = globalPath ?? "@townbuilder/engine-wasm";
  const moduleRef = await import(modulePath as string);
  return moduleRef as WasmModuleLike;
}

export class SimWorker {
  state: SimWorkerState;
  postResponse: (msg: any) => void;
  private readonly _wasmLoader: WasmModuleLoader;

  constructor(postResponse?: (msg: any) => void, wasmLoader?: WasmModuleLoader) {
    this.state = {
      game: null,
      wasm: null,
      speed: SimSpeed.PAUSED,
      running: false,
      tickInterval: null,
    };
    this.postResponse = postResponse ?? ((_msg: any) => {});
    this._wasmLoader = wasmLoader ?? loadWasmModule;
  }

  async handleMessage(msg: any): Promise<void> {
    if (!msg || typeof msg.type !== "string") return;

    switch (msg.type) {
      case MessageType.HANDSHAKE: {
        try {
          await this.initGame(msg.seed ?? 0, msg.width ?? 128, msg.height ?? 128);
          this.postResponse({
            type: MessageType.HANDSHAKE_ACK,
            wire_version: WIRE_VERSION,
            timestamp: Date.now(),
            success: true,
          });
        } catch (err: unknown) {
          this.postResponse({
            type: MessageType.HANDSHAKE_ACK,
            wire_version: WIRE_VERSION,
            timestamp: Date.now(),
            success: false,
            error: err instanceof Error ? err.message : String(err),
          });
        }
        break;
      }

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
        this._handleSaveRequest();
        break;

      case MessageType.LOAD_REQUEST:
        this._handleLoadRequest(msg.data);
        break;

      default:
        break;
    }
  }

  async initGame(seed: number, width: number, height: number): Promise<void> {
    const wasm = await this._wasmLoader();
    const ctor = wasm.GameHandle?.new;
    if (!ctor) {
      throw new Error("WASM module missing GameHandle constructor");
    }
    const game = ctor(seed, width, height);
    if (!game) {
      throw new Error("WASM engine init returned no game handle");
    }
    this.state.wasm = wasm;
    this.state.game = game;
    this.state.running = true;
  }

  startTickLoop(): void {
    this.stopTickLoop();
    if (this.state.speed === SimSpeed.PAUSED) return;

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
    }, TICK_INTERVAL_MS) as unknown as number;
  }

  stopTickLoop(): void {
    if (this.state.tickInterval !== null) {
      clearInterval(this.state.tickInterval);
      this.state.tickInterval = null;
    }
  }

  setSpeed(speed: SimSpeed): void {
    this.state.speed = speed;
    if (!this.state.running) return;
    if (speed === SimSpeed.PAUSED) {
      this.stopTickLoop();
      return;
    }
    this.startTickLoop();
  }

  executeTick(): TickResult {
    if (!this.state.game) {
      return { tick: 0, events: [], population: 0, treasury: 0 };
    }
    return parseTickResult(this.state.game.tick());
  }

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
      const result = parseCommandResult(this.state.game.apply_command_json(commandJson));
      this.postResponse({
        type: MessageType.COMMAND_RESULT,
        success: result.success,
        error: result.error,
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

  save(): Uint8Array {
    if (!this.state.game) {
      return new Uint8Array(0);
    }
    return this.state.game.save();
  }

  load(data: Uint8Array): boolean {
    const loader = this.state.wasm?.load_game;
    if (!loader) {
      return false;
    }
    const game = loader(data);
    this.state.game = game;
    this.state.running = true;
    return true;
  }

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

  private _handleSaveRequest(): void {
    try {
      const data = this.save();
      this.postResponse({
        type: MessageType.SAVE_RESPONSE,
        success: true,
        data: _uint8ArrayToBase64(data),
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

  private _handleLoadRequest(dataStr: string): void {
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

export function _uint8ArrayToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

export function _base64ToUint8Array(base64: string): Uint8Array {
  if (base64.length === 0) return new Uint8Array(0);
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

declare const self: any;

if (
  typeof self !== "undefined" &&
  typeof self.postMessage === "function" &&
  typeof self.addEventListener === "function"
) {
  const worker = new SimWorker((msg: any) => self.postMessage(msg));
  self.addEventListener("message", (event: MessageEvent) => {
    void worker.handleMessage(event.data);
  });
}
