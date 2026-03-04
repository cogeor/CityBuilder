// @townbuilder/runtime — Tests for SimWorker
import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  SimWorker,
  SimSpeed,
  MockGameHandle,
  TICK_INTERVAL_MS,
} from "../sim.worker.js";
import { MessageType, WIRE_VERSION } from "../../messaging/types.js";

describe("SimWorker", () => {
  let worker: SimWorker;
  let posted: any[];

  beforeEach(() => {
    posted = [];
    worker = new SimWorker((msg: any) => posted.push(msg));
  });

  // ---- Test 1: Default state ----
  it("creates with default state", () => {
    expect(worker.state.game).toBeNull();
    expect(worker.state.speed).toBe(SimSpeed.PAUSED);
    expect(worker.state.running).toBe(false);
    expect(worker.state.tickInterval).toBeNull();
  });

  // ---- Test 2: initGame sets up game ----
  it("initGame sets up game with MockGameHandle", () => {
    worker.initGame(42, 64, 64);

    expect(worker.state.game).toBeInstanceOf(MockGameHandle);
    expect(worker.state.running).toBe(true);
  });

  // ---- Test 3: setSpeed changes speed ----
  it("setSpeed changes speed", () => {
    worker.initGame(0, 32, 32);
    expect(worker.state.speed).toBe(SimSpeed.PAUSED);

    worker.setSpeed(SimSpeed.NORMAL);
    expect(worker.state.speed).toBe(SimSpeed.NORMAL);

    worker.setSpeed(SimSpeed.FAST);
    expect(worker.state.speed).toBe(SimSpeed.FAST);

    worker.setSpeed(SimSpeed.ULTRA);
    expect(worker.state.speed).toBe(SimSpeed.ULTRA);

    // Cleanup any intervals
    worker.stopTickLoop();
  });

  // ---- Test 4: executeTick increments tick ----
  it("executeTick increments tick counter", () => {
    worker.initGame(0, 32, 32);

    const result1 = worker.executeTick();
    expect(result1.tick).toBe(1);
    expect(result1.events).toEqual([]);
    expect(result1.population).toBe(100);
    expect(result1.treasury).toBe(10000);

    const result2 = worker.executeTick();
    expect(result2.tick).toBe(2);

    const result3 = worker.executeTick();
    expect(result3.tick).toBe(3);
  });

  // ---- Test 5: executeTick without game returns zeros ----
  it("executeTick returns zeros when no game is initialised", () => {
    const result = worker.executeTick();
    expect(result.tick).toBe(0);
    expect(result.population).toBe(0);
    expect(result.treasury).toBe(0);
  });

  // ---- Test 6: handleMessage HANDSHAKE ----
  it("handleMessage HANDSHAKE sends HANDSHAKE_ACK", () => {
    worker.handleMessage({
      type: MessageType.HANDSHAKE,
      wire_version: WIRE_VERSION,
      timestamp: Date.now(),
    });

    expect(posted).toHaveLength(1);
    expect(posted[0].type).toBe(MessageType.HANDSHAKE_ACK);
    expect(posted[0].wire_version).toBe(WIRE_VERSION);
    expect(typeof posted[0].timestamp).toBe("number");
  });

  // ---- Test 7: handleMessage COMMAND ----
  it("handleMessage COMMAND sends COMMAND_RESULT", () => {
    worker.initGame(0, 32, 32);

    worker.handleMessage({
      type: MessageType.COMMAND,
      command_json: '{"type":"PlaceBuilding","x":5,"y":5}',
      sequence_id: 42,
    });

    expect(posted).toHaveLength(1);
    expect(posted[0].type).toBe(MessageType.COMMAND_RESULT);
    expect(posted[0].success).toBe(true);
    expect(posted[0].sequence_id).toBe(42);
  });

  // ---- Test 8: COMMAND without game returns error ----
  it("handleMessage COMMAND without game returns failure", () => {
    worker.handleMessage({
      type: MessageType.COMMAND,
      command_json: '{"type":"PlaceBuilding"}',
      sequence_id: 7,
    });

    expect(posted).toHaveLength(1);
    expect(posted[0].type).toBe(MessageType.COMMAND_RESULT);
    expect(posted[0].success).toBe(false);
    expect(posted[0].error).toBe("Game not initialised");
    expect(posted[0].sequence_id).toBe(7);
  });

  // ---- Test 9: save returns data ----
  it("save returns Uint8Array data", () => {
    worker.initGame(0, 32, 32);

    const data = worker.save();
    expect(data).toBeInstanceOf(Uint8Array);
  });

  // ---- Test 10: save without game returns empty array ----
  it("save without game returns empty Uint8Array", () => {
    const data = worker.save();
    expect(data).toBeInstanceOf(Uint8Array);
    expect(data.length).toBe(0);
  });

  // ---- Test 11: getStats returns current state ----
  it("getStats returns current simulation statistics", () => {
    worker.initGame(0, 32, 32);

    // Initial stats
    const stats0 = worker.getStats();
    expect(stats0.tick).toBe(0);
    expect(stats0.population).toBe(100);
    expect(stats0.treasury).toBe(10000);

    // After some ticks
    worker.executeTick();
    worker.executeTick();
    const stats2 = worker.getStats();
    expect(stats2.tick).toBe(2);
  });

  // ---- Test 12: getStats without game returns zeros ----
  it("getStats without game returns zeros", () => {
    const stats = worker.getStats();
    expect(stats.tick).toBe(0);
    expect(stats.population).toBe(0);
    expect(stats.treasury).toBe(0);
  });

  // ---- Test 13: PAUSE / RESUME ----
  it("PAUSE sets speed to PAUSED, RESUME sets speed to NORMAL", () => {
    worker.initGame(0, 32, 32);
    worker.setSpeed(SimSpeed.FAST);
    expect(worker.state.speed).toBe(SimSpeed.FAST);

    worker.handleMessage({ type: MessageType.PAUSE });
    expect(worker.state.speed).toBe(SimSpeed.PAUSED);

    worker.handleMessage({ type: MessageType.RESUME });
    expect(worker.state.speed).toBe(SimSpeed.NORMAL);

    // Cleanup
    worker.stopTickLoop();
  });

  // ---- Test 14: SET_SPEED message ----
  it("handleMessage SET_SPEED changes speed", () => {
    worker.initGame(0, 32, 32);

    worker.handleMessage({
      type: MessageType.SET_SPEED,
      speed: SimSpeed.ULTRA,
    });
    expect(worker.state.speed).toBe(SimSpeed.ULTRA);

    // Cleanup
    worker.stopTickLoop();
  });

  // ---- Test 15: load returns boolean ----
  it("load returns boolean success value", () => {
    worker.initGame(0, 32, 32);

    const result = worker.load(new Uint8Array(0));
    expect(result).toBe(true);
  });

  // ---- Test 16: load without game returns false ----
  it("load without game returns false", () => {
    const result = worker.load(new Uint8Array(0));
    expect(result).toBe(false);
  });

  // ---- Test 17: SAVE_REQUEST message ----
  it("handleMessage SAVE_REQUEST sends SAVE_RESPONSE", () => {
    worker.initGame(0, 32, 32);

    worker.handleMessage({
      type: MessageType.SAVE_REQUEST,
      slot: "auto",
    });

    expect(posted).toHaveLength(1);
    expect(posted[0].type).toBe(MessageType.SAVE_RESPONSE);
    expect(posted[0].success).toBe(true);
    expect(typeof posted[0].data).toBe("string");
  });

  // ---- Test 18: LOAD_REQUEST message ----
  it("handleMessage LOAD_REQUEST sends LOAD_RESPONSE", () => {
    worker.initGame(0, 32, 32);

    worker.handleMessage({
      type: MessageType.LOAD_REQUEST,
      slot: "auto",
      data: "", // empty base64
    });

    expect(posted).toHaveLength(1);
    expect(posted[0].type).toBe(MessageType.LOAD_RESPONSE);
    expect(posted[0].success).toBe(true);
  });

  // ---- Test 19: unknown messages are silently ignored ----
  it("silently ignores unknown message types", () => {
    worker.handleMessage({ type: "UNKNOWN_TYPE" });
    expect(posted).toHaveLength(0);
  });

  // ---- Test 20: null/invalid messages are ignored ----
  it("ignores null or invalid messages", () => {
    worker.handleMessage(null);
    worker.handleMessage(undefined);
    worker.handleMessage({});
    worker.handleMessage({ type: 123 }); // type must be string
    expect(posted).toHaveLength(0);
  });
});

describe("MockGameHandle", () => {
  it("tick increments counter and returns result", () => {
    const game = new MockGameHandle();
    expect(game.get_tick()).toBe(0);

    const r = game.tick();
    expect(r.tick).toBe(1);
    expect(game.get_tick()).toBe(1);
  });

  it("apply_command_json returns true", () => {
    const game = new MockGameHandle();
    expect(game.apply_command_json("{}")).toBe(true);
  });

  it("save returns Uint8Array", () => {
    const game = new MockGameHandle();
    expect(game.save()).toBeInstanceOf(Uint8Array);
  });

  it("load returns true", () => {
    const game = new MockGameHandle();
    expect(game.load(new Uint8Array(0))).toBe(true);
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
  it("is 50ms (20 ticks/sec at normal)", () => {
    expect(TICK_INTERVAL_MS).toBe(50);
  });
});
