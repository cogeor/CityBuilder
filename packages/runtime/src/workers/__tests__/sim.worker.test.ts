import { beforeEach, describe, expect, it } from "vitest";
import { MessageType, WIRE_VERSION } from "../../messaging/types.js";
import { SimSpeed, SimWorker, TICK_INTERVAL_MS } from "../sim.worker.js";

class FakeGameHandle {
  private tickCount = 0;

  tick(): string {
    this.tickCount += 1;
    return JSON.stringify({
      tick: this.tickCount,
      population: 100,
      treasury: 10000,
      event_count: 0,
    });
  }

  apply_command_json(_json: string): string {
    return JSON.stringify({ ok: "applied" });
  }

  save(): Uint8Array {
    return new Uint8Array([1, 2, 3]);
  }

  get_tick(): number {
    return this.tickCount;
  }

  get_population(): number {
    return 100;
  }

  get_treasury(): number {
    return 10000;
  }
}

function makeWorker(posted: any[]): SimWorker {
  const wasm = {
    GameHandle: {
      new: () => new FakeGameHandle(),
    },
    load_game: (_data: Uint8Array) => new FakeGameHandle(),
  };
  return new SimWorker((msg: any) => posted.push(msg), async () => wasm);
}

describe("SimWorker", () => {
  let posted: any[];
  let worker: SimWorker;

  beforeEach(() => {
    posted = [];
    worker = makeWorker(posted);
  });

  it("creates with default state", () => {
    expect(worker.state.game).toBeNull();
    expect(worker.state.speed).toBe(SimSpeed.PAUSED);
    expect(worker.state.running).toBe(false);
    expect(worker.state.tickInterval).toBeNull();
  });

  it("initGame sets up game from injected WASM module", async () => {
    await worker.initGame(42, 64, 64);
    expect(worker.state.game).toBeTruthy();
    expect(worker.state.running).toBe(true);
  });

  it("executeTick returns zeros when game is not initialised", () => {
    const result = worker.executeTick();
    expect(result.tick).toBe(0);
    expect(result.population).toBe(0);
    expect(result.treasury).toBe(0);
  });

  it("handleMessage HANDSHAKE sends HANDSHAKE_ACK", async () => {
    await worker.handleMessage({
      type: MessageType.HANDSHAKE,
      wire_version: WIRE_VERSION,
      timestamp: Date.now(),
      seed: 1,
      width: 32,
      height: 32,
    });

    expect(posted).toHaveLength(1);
    expect(posted[0].type).toBe(MessageType.HANDSHAKE_ACK);
    expect(posted[0].success).toBe(true);
  });

  it("handleMessage COMMAND sends COMMAND_RESULT", async () => {
    await worker.initGame(0, 32, 32);

    await worker.handleMessage({
      type: MessageType.COMMAND,
      command_json: '{"SetZoning":{"x":1,"y":1,"w":1,"h":1,"zone":"Residential"}}',
      sequence_id: 42,
    });

    expect(posted).toHaveLength(1);
    expect(posted[0].type).toBe(MessageType.COMMAND_RESULT);
    expect(posted[0].success).toBe(true);
    expect(posted[0].sequence_id).toBe(42);
  });

  it("save/load requests succeed after initialisation", async () => {
    await worker.initGame(0, 32, 32);

    await worker.handleMessage({ type: MessageType.SAVE_REQUEST, slot: "auto" });
    expect(posted[0].type).toBe(MessageType.SAVE_RESPONSE);
    expect(posted[0].success).toBe(true);

    await worker.handleMessage({
      type: MessageType.LOAD_REQUEST,
      slot: "auto",
      data: posted[0].data,
    });
    expect(posted[1].type).toBe(MessageType.LOAD_RESPONSE);
    expect(posted[1].success).toBe(true);
  });

  it("PAUSE and RESUME map to speed values", async () => {
    await worker.initGame(0, 32, 32);
    worker.setSpeed(SimSpeed.FAST);
    expect(worker.state.speed).toBe(SimSpeed.FAST);

    await worker.handleMessage({ type: MessageType.PAUSE });
    expect(worker.state.speed).toBe(SimSpeed.PAUSED);

    await worker.handleMessage({ type: MessageType.RESUME });
    expect(worker.state.speed).toBe(SimSpeed.NORMAL);

    worker.stopTickLoop();
  });
});

describe("SimSpeed enum", () => {
  it("has correct numeric values", () => {
    expect(SimSpeed.PAUSED).toBe(0);
    expect(SimSpeed.NORMAL).toBe(1);
    expect(SimSpeed.FAST).toBe(2);
    expect(SimSpeed.ULTRA).toBe(4);
  });
});

describe("TICK_INTERVAL_MS", () => {
  it("is 50ms", () => {
    expect(TICK_INTERVAL_MS).toBe(50);
  });
});
