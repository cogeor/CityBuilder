// @townbuilder/runtime — Tests for WorkerManager
import { describe, it, expect, beforeEach } from "vitest";
import {
  WorkerManager,
  MockWorker,
  type WorkerManagerState,
} from "../worker_manager.js";
import {
  MessageType,
  WIRE_VERSION,
  type TickOutputMessage,
  type CommandResultMessage,
  type PickResponseMessage,
} from "../types.js";

describe("WorkerManager", () => {
  let manager: WorkerManager;

  beforeEach(() => {
    manager = new WorkerManager();
  });

  // ---- Test 1: creates with null workers ----
  it("creates with null workers and default state", () => {
    expect(manager.state.simWorker).toBeNull();
    expect(manager.state.renderWorker).toBeNull();
    expect(manager.state.simReady).toBe(false);
    expect(manager.state.renderReady).toBe(false);
    expect(manager.state.speed).toBe(0);
    expect(manager.state.commandSequence).toBe(0);
    expect(manager.state.pendingCommands.size).toBe(0);
    expect(manager.state.tickCallbacks).toHaveLength(0);
    expect(manager.state.pickCallbacks).toHaveLength(0);
  });

  // ---- Test 2: sendCommand increments sequence ----
  it("sendCommand increments command sequence ID", () => {
    const mock = new MockWorker();
    manager.state.simWorker = mock as unknown as Worker;

    // Fire off two commands (don't await — we just check the sequence)
    const _p1 = manager.sendCommand('{"type":"PlaceBuilding"}');
    const _p2 = manager.sendCommand('{"type":"Bulldoze"}');

    expect(manager.state.commandSequence).toBe(2);
    expect(mock.postedMessages).toHaveLength(2);
    expect(mock.postedMessages[0].sequence_id).toBe(1);
    expect(mock.postedMessages[1].sequence_id).toBe(2);
  });

  // ---- Test 3: setSpeed sends SET_SPEED message ----
  it("setSpeed sends SET_SPEED message to sim worker", () => {
    const mock = new MockWorker();
    manager.state.simWorker = mock as unknown as Worker;

    manager.setSpeed(2);

    expect(manager.state.speed).toBe(2);
    expect(mock.postedMessages).toHaveLength(1);
    expect(mock.postedMessages[0].type).toBe(MessageType.SET_SPEED);
    expect(mock.postedMessages[0].speed).toBe(2);
  });

  // ---- Test 4: pause sends PAUSE message ----
  it("pause sends PAUSE message and sets speed to 0", () => {
    const mock = new MockWorker();
    manager.state.simWorker = mock as unknown as Worker;
    manager.state.speed = 2;

    manager.pause();

    expect(manager.state.speed).toBe(0);
    expect(mock.postedMessages).toHaveLength(1);
    expect(mock.postedMessages[0].type).toBe(MessageType.PAUSE);
  });

  // ---- Test 5: resume sends RESUME message ----
  it("resume sends RESUME message and sets speed to 1 if paused", () => {
    const mock = new MockWorker();
    manager.state.simWorker = mock as unknown as Worker;
    manager.state.speed = 0;

    manager.resume();

    expect(manager.state.speed).toBe(1);
    expect(mock.postedMessages).toHaveLength(1);
    expect(mock.postedMessages[0].type).toBe(MessageType.RESUME);
  });

  // ---- Test 6: resume preserves non-zero speed ----
  it("resume preserves existing non-zero speed", () => {
    const mock = new MockWorker();
    manager.state.simWorker = mock as unknown as Worker;
    manager.state.speed = 4;

    manager.resume();

    expect(manager.state.speed).toBe(4);
    expect(mock.postedMessages[0].type).toBe(MessageType.RESUME);
  });

  // ---- Test 7: onTick registers callback ----
  it("onTick registers a callback", () => {
    const cb = (_output: TickOutputMessage) => {};
    manager.onTick(cb);

    expect(manager.state.tickCallbacks).toHaveLength(1);
    expect(manager.state.tickCallbacks[0]).toBe(cb);
  });

  // ---- Test 8: onPick registers callback ----
  it("onPick registers a callback", () => {
    const cb = (_result: PickResponseMessage) => {};
    manager.onPick(cb);

    expect(manager.state.pickCallbacks).toHaveLength(1);
    expect(manager.state.pickCallbacks[0]).toBe(cb);
  });

  // ---- Test 9: shutdown terminates workers ----
  it("shutdown terminates workers and resets state", () => {
    const simMock = new MockWorker();
    const renderMock = new MockWorker();
    manager.state.simWorker = simMock as unknown as Worker;
    manager.state.renderWorker = renderMock as unknown as Worker;
    manager.state.simReady = true;
    manager.state.renderReady = true;
    manager.state.tickCallbacks.push(() => {});
    manager.state.pickCallbacks.push(() => {});

    manager.shutdown();

    expect(simMock.terminated).toBe(true);
    expect(renderMock.terminated).toBe(true);
    expect(manager.state.simWorker).toBeNull();
    expect(manager.state.renderWorker).toBeNull();
    expect(manager.state.simReady).toBe(false);
    expect(manager.state.renderReady).toBe(false);
    expect(manager.state.pendingCommands.size).toBe(0);
    expect(manager.state.tickCallbacks).toHaveLength(0);
    expect(manager.state.pickCallbacks).toHaveLength(0);
  });

  // ---- Test 10: onSimMessage routes TICK_OUTPUT ----
  it("onSimMessage routes TICK_OUTPUT to tick callbacks", () => {
    const received: TickOutputMessage[] = [];
    manager.onTick((output) => received.push(output));

    const tickMsg: TickOutputMessage = {
      type: MessageType.TICK_OUTPUT,
      tick: 42,
      events_json: "[]",
      population: 500,
      treasury: 20000,
    };

    manager.onSimMessage({ data: tickMsg } as MessageEvent);

    expect(received).toHaveLength(1);
    expect(received[0].tick).toBe(42);
    expect(received[0].population).toBe(500);
    expect(received[0].treasury).toBe(20000);
  });

  // ---- Test 11: onSimMessage routes COMMAND_RESULT ----
  it("onSimMessage routes COMMAND_RESULT to pending command", async () => {
    const mock = new MockWorker();
    manager.state.simWorker = mock as unknown as Worker;

    // Start a command (creates pending promise at seqId 1)
    const promise = manager.sendCommand('{"type":"Zone"}');

    // Simulate the worker sending back a COMMAND_RESULT
    const resultMsg: CommandResultMessage = {
      type: MessageType.COMMAND_RESULT,
      success: true,
      sequence_id: 1,
    };
    manager.onSimMessage({ data: resultMsg } as MessageEvent);

    const result = await promise;
    expect(result.success).toBe(true);
    expect(result.sequence_id).toBe(1);
    expect(manager.state.pendingCommands.size).toBe(0);
  });

  // ---- Test 12: onSimMessage routes PICK_RESPONSE ----
  it("onSimMessage routes PICK_RESPONSE to pick callbacks", () => {
    const received: PickResponseMessage[] = [];
    manager.onPick((result) => received.push(result));

    const pickMsg: PickResponseMessage = {
      type: MessageType.PICK_RESPONSE,
      tile: { x: 10, y: 20 },
    };

    manager.onSimMessage({ data: pickMsg } as MessageEvent);

    expect(received).toHaveLength(1);
    expect(received[0].tile?.x).toBe(10);
    expect(received[0].tile?.y).toBe(20);
  });

  // ---- Test 13: onSimMessage ignores invalid messages ----
  it("onSimMessage ignores null/invalid messages", () => {
    const received: TickOutputMessage[] = [];
    manager.onTick((output) => received.push(output));

    manager.onSimMessage({ data: null } as MessageEvent);
    manager.onSimMessage({ data: {} } as MessageEvent);
    manager.onSimMessage({ data: { type: 123 } } as MessageEvent);

    expect(received).toHaveLength(0);
  });

  // ---- Test 14: onRenderMessage routes PICK_RESPONSE ----
  it("onRenderMessage routes PICK_RESPONSE to pick callbacks", () => {
    const received: PickResponseMessage[] = [];
    manager.onPick((result) => received.push(result));

    const pickMsg: PickResponseMessage = {
      type: MessageType.PICK_RESPONSE,
      entity: { index: 5, generation: 1 },
    };

    manager.onRenderMessage({ data: pickMsg } as MessageEvent);

    expect(received).toHaveLength(1);
    expect(received[0].entity?.index).toBe(5);
  });

  // ---- Test 15: multiple tick callbacks are all invoked ----
  it("invokes all registered tick callbacks", () => {
    let count1 = 0;
    let count2 = 0;
    manager.onTick(() => { count1++; });
    manager.onTick(() => { count2++; });

    const tickMsg: TickOutputMessage = {
      type: MessageType.TICK_OUTPUT,
      tick: 1,
      events_json: "[]",
      population: 0,
      treasury: 0,
    };

    manager.onSimMessage({ data: tickMsg } as MessageEvent);

    expect(count1).toBe(1);
    expect(count2).toBe(1);
  });

  // ---- Test 16: sendCommand without worker throws ----
  it("sendCommand throws when no sim worker is assigned", async () => {
    await expect(manager.sendCommand("{}")).rejects.toThrow("No sim worker available");
  });

  // ---- Test 17: setSpeed without worker does not throw ----
  it("setSpeed without worker updates state but does not throw", () => {
    manager.setSpeed(4);
    expect(manager.state.speed).toBe(4);
  });

  // ---- Test 18: pause without worker does not throw ----
  it("pause without worker updates state but does not throw", () => {
    manager.state.speed = 2;
    manager.pause();
    expect(manager.state.speed).toBe(0);
  });

  // ---- Test 19: shutdown with null workers does not throw ----
  it("shutdown with null workers does not throw", () => {
    expect(() => manager.shutdown()).not.toThrow();
  });

  // ---- Test 20: COMMAND_RESULT for unknown seqId is silently ignored ----
  it("onSimMessage silently ignores COMMAND_RESULT with unknown seqId", () => {
    const resultMsg: CommandResultMessage = {
      type: MessageType.COMMAND_RESULT,
      success: true,
      sequence_id: 999,
    };

    // Should not throw
    expect(() => {
      manager.onSimMessage({ data: resultMsg } as MessageEvent);
    }).not.toThrow();
  });
});

describe("MockWorker", () => {
  // ---- Test 1: captures messages ----
  it("captures posted messages", () => {
    const mock = new MockWorker();

    mock.postMessage({ type: "TEST", value: 1 });
    mock.postMessage({ type: "TEST", value: 2 });

    expect(mock.postedMessages).toHaveLength(2);
    expect(mock.postedMessages[0].value).toBe(1);
    expect(mock.postedMessages[1].value).toBe(2);
  });

  // ---- Test 2: addEventListener and simulateMessage ----
  it("dispatches simulated messages to listeners", () => {
    const mock = new MockWorker();
    const received: any[] = [];

    mock.addEventListener("message", (event: any) => {
      received.push(event.data);
    });

    mock.simulateMessage({ type: "TICK_OUTPUT", tick: 5 });

    expect(received).toHaveLength(1);
    expect(received[0].tick).toBe(5);
  });

  // ---- Test 3: terminate sets flag ----
  it("terminate sets terminated flag", () => {
    const mock = new MockWorker();
    expect(mock.terminated).toBe(false);

    mock.terminate();
    expect(mock.terminated).toBe(true);
  });

  // ---- Test 4: removeEventListener works ----
  it("removeEventListener removes a specific listener", () => {
    const mock = new MockWorker();
    let callCount = 0;

    const listener = () => { callCount++; };
    mock.addEventListener("message", listener);
    mock.simulateMessage({});
    expect(callCount).toBe(1);

    mock.removeEventListener("message", listener);
    mock.simulateMessage({});
    expect(callCount).toBe(1); // not called again
  });

  // ---- Test 5: multiple listeners ----
  it("supports multiple listeners for same event", () => {
    const mock = new MockWorker();
    let count1 = 0;
    let count2 = 0;

    mock.addEventListener("message", () => { count1++; });
    mock.addEventListener("message", () => { count2++; });

    mock.simulateMessage({});

    expect(count1).toBe(1);
    expect(count2).toBe(1);
  });
});
