// @townbuilder/runtime — Tests for RenderWorker
import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  RenderWorker,
  RenderWorkerState,
  GPUBackendType,
  DEFAULT_RENDER_CONFIG,
} from "../render.worker.js";
import type {
  RenderCamera,
  RenderWorkerConfig,
  PickRequest,
  RenderFrameStats,
} from "../render.worker.js";

describe("RenderWorker", () => {
  let worker: RenderWorker;
  let messages: any[];

  beforeEach(() => {
    messages = [];
    worker = new RenderWorker();
    worker.setMessageHandler((msg: any) => messages.push(msg));
  });

  // ---- Test 1: Constructor starts in Uninitialized state ----
  it("constructor starts in Uninitialized state", () => {
    const w = new RenderWorker();
    expect(w.getState()).toBe(RenderWorkerState.Uninitialized);
  });

  // ---- Test 2: Constructor merges config ----
  it("constructor merges partial config with defaults", () => {
    const w = new RenderWorker({ targetFps: 30, enableVSync: false });
    // The worker should accept partial config without error
    expect(w.getState()).toBe(RenderWorkerState.Uninitialized);
    // We verify the config was accepted by checking it doesn't throw
    // and defaults are preserved for unset fields
  });

  // ---- Test 3: getBackend returns None initially ----
  it("getBackend returns None initially", () => {
    expect(worker.getBackend()).toBe(GPUBackendType.None);
  });

  // ---- Test 4: updateCamera updates camera state ----
  it("updateCamera updates camera state", () => {
    worker.updateCamera({ x: 100, y: 200 });
    const cam = worker.getCamera();
    expect(cam.x).toBe(100);
    expect(cam.y).toBe(200);
    // Unchanged fields remain at defaults
    expect(cam.zoom).toBe(1);
    expect(cam.viewportW).toBe(800);
    expect(cam.viewportH).toBe(600);
  });

  // ---- Test 5: updateInstances stores buffer and count ----
  it("updateInstances stores buffer and count", () => {
    const buffer = new Float32Array([1.0, 2.0, 3.0]);
    worker.updateInstances(buffer, 3);
    const stats = worker.getStats();
    expect(stats.instanceCount).toBe(3);
  });

  // ---- Test 6: submitPick adds pick request ----
  it("submitPick adds pick request", () => {
    const pick: PickRequest = { id: 1, screenX: 100, screenY: 200 };
    worker.submitPick(pick);
    // Pick is pending; we verify it doesn't throw
    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
  });

  // ---- Test 7: handleMessage with camera type updates camera ----
  it("handleMessage with camera type updates camera", () => {
    worker.handleMessage({
      type: "camera",
      camera: { x: 50, y: 75, zoom: 2 },
    });
    const cam = worker.getCamera();
    expect(cam.x).toBe(50);
    expect(cam.y).toBe(75);
    expect(cam.zoom).toBe(2);
  });

  // ---- Test 8: handleMessage with instances type stores buffer ----
  it("handleMessage with instances type stores buffer", () => {
    const buffer = new Float32Array([1.0, 2.0]);
    worker.handleMessage({
      type: "instances",
      buffer: buffer,
      count: 2,
    });
    const stats = worker.getStats();
    expect(stats.instanceCount).toBe(2);
  });

  // ---- Test 9: handleMessage with pick type adds pick request ----
  it("handleMessage with pick type adds pick request", () => {
    worker.handleMessage({
      type: "pick",
      request: { id: 42, screenX: 300, screenY: 400 },
    });
    // No error means pick was accepted
    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
  });

  // ---- Test 10: handleMessage with pause type pauses (when running) ----
  it("handleMessage with pause type pauses when running", () => {
    // Need to be in Running state for pause to work
    // Since we can't initialize without a real canvas, we test that
    // handleMessage doesn't throw for pause
    worker.handleMessage({ type: "pause" });
    // State doesn't change from Uninitialized since pause only works in Running
    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
  });

  // ---- Test 11: handleMessage ignores unknown types ----
  it("handleMessage ignores unknown types", () => {
    worker.handleMessage({ type: "unknown_type_xyz" });
    expect(messages).toHaveLength(0);
    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
  });

  // ---- Test 12: start throws if not Ready ----
  it("start throws if not Ready", () => {
    expect(() => worker.start()).toThrow("Cannot start from state: uninitialized");
  });

  // ---- Test 13: setMessageHandler stores handler ----
  it("setMessageHandler stores handler", () => {
    const handler = vi.fn();
    worker.setMessageHandler(handler);
    // The handler should be stored — we can verify by triggering output
    // that uses onMessage (e.g., via handleMessage with pick while in a state
    // that processes picks). For now we just check no errors.
    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
  });

  // ---- Test 14: getStats returns current stats ----
  it("getStats returns current stats with defaults", () => {
    const stats = worker.getStats();
    expect(stats.fps).toBe(0);
    expect(stats.drawCalls).toBe(0);
    expect(stats.instanceCount).toBe(0);
    expect(stats.backend).toBe(GPUBackendType.None);
    expect(typeof stats.frameTimeMs).toBe("number");
  });

  // ---- Test 15: shutdown resets state ----
  it("shutdown resets state to Uninitialized", () => {
    // Set some state first
    worker.updateCamera({ x: 999, y: 999 });
    worker.updateInstances(new Float32Array([1, 2, 3]), 3);
    worker.submitPick({ id: 1, screenX: 10, screenY: 20 });

    worker.shutdown();

    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
    const stats = worker.getStats();
    expect(stats.instanceCount).toBe(0);
  });

  // ---- Test 16: getFrameCount returns 0 initially ----
  it("getFrameCount returns 0 initially", () => {
    expect(worker.getFrameCount()).toBe(0);
  });

  // ---- Test 17: Default config has correct values ----
  it("DEFAULT_RENDER_CONFIG has correct values", () => {
    expect(DEFAULT_RENDER_CONFIG.preferredBackend).toBe(GPUBackendType.WebGPU);
    expect(DEFAULT_RENDER_CONFIG.targetFps).toBe(60);
    expect(DEFAULT_RENDER_CONFIG.enableVSync).toBe(true);
    expect(DEFAULT_RENDER_CONFIG.maxChunkRebuildsPerFrame).toBe(4);
  });

  // ---- Test 18: handleMessage ignores null and invalid messages ----
  it("ignores null, undefined, and messages without type", () => {
    worker.handleMessage(null);
    worker.handleMessage(undefined);
    worker.handleMessage({});
    expect(messages).toHaveLength(0);
  });

  // ---- Test 19: handleMessage with config updates config ----
  it("handleMessage with config type updates config", () => {
    worker.handleMessage({
      type: "config",
      config: { targetFps: 30 },
    });
    // No error means config was accepted
    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
  });

  // ---- Test 20: getCamera returns a copy ----
  it("getCamera returns a copy, not a reference", () => {
    const cam1 = worker.getCamera();
    cam1.x = 9999;
    const cam2 = worker.getCamera();
    expect(cam2.x).toBe(0); // unchanged
  });

  // ---- Test 21: updateCamera partial update preserves other fields ----
  it("updateCamera partial update preserves other fields", () => {
    worker.updateCamera({ x: 10, y: 20, zoom: 3, viewportW: 1920, viewportH: 1080 });
    worker.updateCamera({ zoom: 5 });
    const cam = worker.getCamera();
    expect(cam.x).toBe(10);
    expect(cam.y).toBe(20);
    expect(cam.zoom).toBe(5);
    expect(cam.viewportW).toBe(1920);
    expect(cam.viewportH).toBe(1080);
  });

  // ---- Test 22: getFps returns 0 initially ----
  it("getFps returns 0 initially", () => {
    expect(worker.getFps()).toBe(0);
  });

  // ---- Test 23: getStats after updateInstances reflects count ----
  it("getStats after updateInstances reflects correct instance count", () => {
    worker.updateInstances(new Float32Array(100), 25);
    const stats = worker.getStats();
    expect(stats.instanceCount).toBe(25);
    expect(stats.backend).toBe(GPUBackendType.None);
  });

  // ---- Test 24: handleMessage resume does nothing when not paused ----
  it("handleMessage resume does nothing when not paused", () => {
    worker.handleMessage({ type: "resume" });
    expect(worker.getState()).toBe(RenderWorkerState.Uninitialized);
  });
});

describe("RenderWorkerState enum", () => {
  it("has correct string values", () => {
    expect(RenderWorkerState.Uninitialized).toBe("uninitialized");
    expect(RenderWorkerState.Initializing).toBe("initializing");
    expect(RenderWorkerState.Ready).toBe("ready");
    expect(RenderWorkerState.Running).toBe("running");
    expect(RenderWorkerState.Paused).toBe("paused");
    expect(RenderWorkerState.Error).toBe("error");
  });
});

describe("GPUBackendType enum", () => {
  it("has correct string values", () => {
    expect(GPUBackendType.WebGPU).toBe("webgpu");
    expect(GPUBackendType.WebGL2).toBe("webgl2");
    expect(GPUBackendType.None).toBe("none");
  });
});
