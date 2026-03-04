import { describe, it, expect, beforeEach } from "vitest";
import {
  WebGPUBackend,
  MockGPUDevice,
  MockGPUCanvas,
  MockGPUAdapter,
  WGSL_VERTEX_SHADER,
  WGSL_FRAGMENT_SHADER,
  isWebGPUAvailable,
  type RenderStats,
} from "../index.js";

describe("WebGPUBackend", () => {
  let backend: WebGPUBackend;
  let mockDevice: MockGPUDevice;

  beforeEach(() => {
    backend = new WebGPUBackend();
    mockDevice = new MockGPUDevice();
  });

  // ─── Initialization ──────────────────────────────────────────────────

  it("creates with initialized=false", () => {
    expect(backend.isInitialized()).toBe(false);
    expect(backend.initialized).toBe(false);
    expect(backend.device).toBeNull();
    expect(backend.context).toBeNull();
    expect(backend.pipeline).toBeNull();
  });

  it("isWebGPUAvailable returns boolean", () => {
    const result = WebGPUBackend.isWebGPUAvailable();
    expect(typeof result).toBe("boolean");
    // In test environment (no navigator.gpu), should be false
    expect(result).toBe(false);
  });

  it("standalone isWebGPUAvailable returns boolean", () => {
    const result = isWebGPUAvailable();
    expect(typeof result).toBe("boolean");
  });

  it("init with mock device and canvas succeeds", () => {
    const calls = mockDevice.calls;
    const canvas = new MockGPUCanvas(1024, 768, true, calls);

    // Pre-set the device (synchronous init path)
    backend.device = mockDevice;
    const result = backend.init(canvas);

    expect(result).toBe(true);
    expect(backend.isInitialized()).toBe(true);
    expect(backend.context).not.toBeNull();
    expect(backend.pipeline).not.toBeNull();
    expect(backend.instanceBuffer).not.toBeNull();
  });

  it("init without webgpu context returns false", () => {
    const canvas = new MockGPUCanvas(800, 600, false);
    backend.device = mockDevice;
    const result = backend.init(canvas);

    expect(result).toBe(false);
    expect(backend.isInitialized()).toBe(false);
  });

  it("init without device returns false", () => {
    const canvas = new MockGPUCanvas(800, 600, true);
    // device is null by default
    const result = backend.init(canvas);

    expect(result).toBe(false);
    expect(backend.isInitialized()).toBe(false);
  });

  // ─── Async Init ─────────────────────────────────────────────────────

  it("initAsync with mock GPU succeeds", async () => {
    const adapter = new MockGPUAdapter();
    const mockGpu = {
      requestAdapter: async () => {
        adapter.calls.requestAdapter++;
        return adapter;
      },
    };

    const canvas = new MockGPUCanvas(1024, 768, true, adapter.calls);
    const result = await backend.initAsync(canvas, mockGpu);

    expect(result).toBe(true);
    expect(backend.isInitialized()).toBe(true);
    expect(backend.device).not.toBeNull();
    expect(backend.pipeline).not.toBeNull();
    expect(adapter.calls.requestAdapter).toBe(1);
    expect(adapter.calls.requestDevice).toBe(1);
    expect(adapter.calls.configure).toBe(1);
  });

  it("initAsync without gpu api returns false", async () => {
    const canvas = new MockGPUCanvas();
    const result = await backend.initAsync(canvas, undefined);

    expect(result).toBe(false);
    expect(backend.isInitialized()).toBe(false);
  });

  // ─── Frame Lifecycle ──────────────────────────────────────────────────

  it("beginFrame resets stats", () => {
    backend.device = mockDevice;
    const canvas = new MockGPUCanvas(800, 600, true, mockDevice.calls);
    backend.init(canvas);

    // Manually set some stats
    backend.stats.drawCalls = 5;
    backend.stats.instancesDrawn = 100;
    backend.stats.texturesUploaded = 2;
    backend.stats.frameTime = 16.5;

    backend.beginFrame();

    const stats = backend.getStats();
    expect(stats.drawCalls).toBe(0);
    expect(stats.instancesDrawn).toBe(0);
    expect(stats.texturesUploaded).toBe(0);
    expect(stats.frameTime).toBe(0);
  });

  // ─── Draw Calls ───────────────────────────────────────────────────────

  it("drawInstances tracks draw calls and instance count", () => {
    backend.device = mockDevice;
    const canvas = new MockGPUCanvas(800, 600, true, mockDevice.calls);
    backend.init(canvas);

    backend.beginFrame();
    backend.drawInstances(0, 0, 50);
    backend.drawInstances(1, 0, 30);

    const stats = backend.getStats();
    expect(stats.drawCalls).toBe(2);
    expect(stats.instancesDrawn).toBe(80);
  });

  // ─── Texture Upload ───────────────────────────────────────────────────

  it("uploadTexture stores textures and increments stats", () => {
    backend.device = mockDevice;
    const canvas = new MockGPUCanvas(800, 600, true, mockDevice.calls);
    backend.init(canvas);

    backend.beginFrame();
    backend.uploadTexture(0, { width: 256, height: 256 });
    backend.uploadTexture(1, { width: 512, height: 512 });

    expect(backend.textures.size).toBe(2);
    expect(backend.textures.has(0)).toBe(true);
    expect(backend.textures.has(1)).toBe(true);

    const stats = backend.getStats();
    expect(stats.texturesUploaded).toBe(2);
  });

  // ─── Destroy ──────────────────────────────────────────────────────────

  it("destroy cleans up all GPU resources", () => {
    backend.device = mockDevice;
    const canvas = new MockGPUCanvas(800, 600, true, mockDevice.calls);
    backend.init(canvas);

    // Upload some textures to clean up
    backend.uploadTexture(0, { width: 256, height: 256 });
    backend.uploadTexture(1, { width: 512, height: 512 });

    backend.destroy();

    expect(backend.isInitialized()).toBe(false);
    expect(backend.device).toBeNull();
    expect(backend.context).toBeNull();
    expect(backend.pipeline).toBeNull();
    expect(backend.instanceBuffer).toBeNull();
    expect(backend.textures.size).toBe(0);

    // Verify cleanup calls were made
    expect(mockDevice.calls.destroyBuffer).toBeGreaterThanOrEqual(1);
    expect(mockDevice.calls.destroyTexture).toBeGreaterThanOrEqual(2);
  });

  // ─── Stats ────────────────────────────────────────────────────────────

  it("getStats returns a copy of current stats", () => {
    backend.device = mockDevice;
    const canvas = new MockGPUCanvas(800, 600, true, mockDevice.calls);
    backend.init(canvas);

    backend.beginFrame();
    backend.drawInstances(0, 0, 25);

    const stats1 = backend.getStats();
    const stats2 = backend.getStats();

    // Should be equal values
    expect(stats1.drawCalls).toBe(stats2.drawCalls);
    expect(stats1.instancesDrawn).toBe(stats2.instancesDrawn);

    // But should be different object references (copy)
    expect(stats1).not.toBe(stats2);
    expect(stats1).toEqual(stats2);
  });

  // ─── Resize ───────────────────────────────────────────────────────────

  it("resize updates canvas dimensions", () => {
    backend.device = mockDevice;
    const canvas = new MockGPUCanvas(800, 600, true, mockDevice.calls);
    backend.init(canvas);

    backend.resize(1920, 1080);

    expect(canvas.width).toBe(1920);
    expect(canvas.height).toBe(1080);
  });
});

// ─── WGSL Shader Sources ──────────────────────────────────────────────────

describe("WGSL shader sources", () => {
  it("WGSL_VERTEX_SHADER is a non-empty string", () => {
    expect(typeof WGSL_VERTEX_SHADER).toBe("string");
    expect(WGSL_VERTEX_SHADER.length).toBeGreaterThan(0);
    expect(WGSL_VERTEX_SHADER).toContain("vs_main");
    expect(WGSL_VERTEX_SHADER).toContain("position");
  });

  it("WGSL_FRAGMENT_SHADER is a non-empty string", () => {
    expect(typeof WGSL_FRAGMENT_SHADER).toBe("string");
    expect(WGSL_FRAGMENT_SHADER.length).toBeGreaterThan(0);
    expect(WGSL_FRAGMENT_SHADER).toContain("fs_main");
    expect(WGSL_FRAGMENT_SHADER).toContain("textureSample");
  });
});

// ─── Mock Classes ─────────────────────────────────────────────────────────

describe("MockGPUAdapter", () => {
  it("requestDevice returns a MockGPUDevice", async () => {
    const adapter = new MockGPUAdapter();
    const device = await adapter.requestDevice();

    expect(device).toBeInstanceOf(MockGPUDevice);
    expect(adapter.calls.requestDevice).toBe(1);
  });
});

describe("MockGPUDevice", () => {
  it("createShaderModule tracks calls", () => {
    const device = new MockGPUDevice();
    device.createShaderModule({ code: "test" });
    device.createShaderModule({ code: "test2" });

    expect(device.calls.createShaderModule).toBe(2);
  });

  it("createBuffer returns object with destroy", () => {
    const device = new MockGPUDevice();
    const buffer = device.createBuffer({ size: 256 }) as any;

    expect(buffer.destroy).toBeDefined();
    buffer.destroy();
    expect(device.calls.destroyBuffer).toBe(1);
  });
});
