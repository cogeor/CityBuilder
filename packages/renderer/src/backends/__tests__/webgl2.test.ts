import { describe, it, expect, beforeEach } from "vitest";
import {
  WebGL2Backend,
  MockCanvas,
  MockGL,
  VERTEX_SHADER_SRC,
  FRAGMENT_SHADER_SRC,
  type RenderStats,
} from "../webgl2.js";

describe("WebGL2Backend", () => {
  let backend: WebGL2Backend;

  beforeEach(() => {
    backend = new WebGL2Backend();
  });

  // ─── Initialization ──────────────────────────────────────────────────

  it("creates with initialized=false", () => {
    expect(backend.isInitialized()).toBe(false);
    expect(backend.initialized).toBe(false);
    expect(backend.gl).toBeNull();
    expect(backend.program).toBeNull();
  });

  it("init with mock canvas succeeds", () => {
    const canvas = new MockCanvas(1024, 768);
    const result = backend.init(canvas);

    expect(result).toBe(true);
    expect(backend.isInitialized()).toBe(true);
    expect(backend.gl).not.toBeNull();
    expect(backend.program).not.toBeNull();
    expect(backend.instanceBuffer).not.toBeNull();
  });

  it("init without webgl2 support returns false", () => {
    const canvas = new MockCanvas(800, 600, false);
    const result = backend.init(canvas);

    expect(result).toBe(false);
    expect(backend.isInitialized()).toBe(false);
    expect(backend.gl).toBeNull();
  });

  // ─── Frame Lifecycle ─────────────────────────────────────────────────

  it("beginFrame resets stats", () => {
    const canvas = new MockCanvas();
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

  // ─── Instance Upload ─────────────────────────────────────────────────

  it("uploadInstances tracks count via GL calls", () => {
    const canvas = new MockCanvas();
    backend.init(canvas);

    const mockGL = backend.gl as MockGL;
    const bufferDataBefore = mockGL.calls.bufferData;

    const buffer = new ArrayBuffer(256);
    backend.uploadInstances(buffer, 10);

    expect(mockGL.calls.bufferData).toBe(bufferDataBefore + 1);
  });

  // ─── Draw Calls ──────────────────────────────────────────────────────

  it("drawInstances increments draw calls and instances count", () => {
    const canvas = new MockCanvas();
    backend.init(canvas);

    backend.beginFrame();
    backend.drawInstances(0, 0, 50);
    backend.drawInstances(1, 0, 30);

    const stats = backend.getStats();
    expect(stats.drawCalls).toBe(2);
    expect(stats.instancesDrawn).toBe(80);
  });

  // ─── Texture Upload ──────────────────────────────────────────────────

  it("uploadTexture stores texture and increments stats", () => {
    const canvas = new MockCanvas();
    backend.init(canvas);

    backend.beginFrame();
    backend.uploadTexture(0, { __mock: "image" });
    backend.uploadTexture(1, { __mock: "image2" });

    expect(backend.textures.size).toBe(2);
    expect(backend.textures.has(0)).toBe(true);
    expect(backend.textures.has(1)).toBe(true);

    const stats = backend.getStats();
    expect(stats.texturesUploaded).toBe(2);
  });

  // ─── Destroy ─────────────────────────────────────────────────────────

  it("destroy cleans up all GPU resources", () => {
    const canvas = new MockCanvas();
    backend.init(canvas);

    const mockGL = backend.gl as MockGL;

    // Upload some textures to clean up
    backend.uploadTexture(0, { __mock: "image" });
    backend.uploadTexture(1, { __mock: "image2" });

    backend.destroy();

    expect(backend.isInitialized()).toBe(false);
    expect(backend.gl).toBeNull();
    expect(backend.program).toBeNull();
    expect(backend.instanceBuffer).toBeNull();
    expect(backend.textures.size).toBe(0);

    // Verify cleanup calls were made
    expect(mockGL.calls.deleteBuffer).toBeGreaterThanOrEqual(1);
    expect(mockGL.calls.deleteTexture).toBeGreaterThanOrEqual(2);
    expect(mockGL.calls.deleteProgram).toBeGreaterThanOrEqual(1);
  });

  // ─── Stats ───────────────────────────────────────────────────────────

  it("getStats returns a copy of current stats", () => {
    const canvas = new MockCanvas();
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

  // ─── Resize ──────────────────────────────────────────────────────────

  it("resize calls viewport on the GL context", () => {
    const canvas = new MockCanvas();
    backend.init(canvas);

    const mockGL = backend.gl as MockGL;
    const viewportBefore = mockGL.calls.viewport;

    backend.resize(1920, 1080);

    expect(mockGL.calls.viewport).toBe(viewportBefore + 1);
  });
});

// ─── Shader Sources ──────────────────────────────────────────────────────

describe("shader sources", () => {
  it("VERTEX_SHADER_SRC is a non-empty string", () => {
    expect(typeof VERTEX_SHADER_SRC).toBe("string");
    expect(VERTEX_SHADER_SRC.length).toBeGreaterThan(0);
    expect(VERTEX_SHADER_SRC).toContain("#version 300 es");
    expect(VERTEX_SHADER_SRC).toContain("gl_Position");
  });

  it("FRAGMENT_SHADER_SRC is a non-empty string", () => {
    expect(typeof FRAGMENT_SHADER_SRC).toBe("string");
    expect(FRAGMENT_SHADER_SRC.length).toBeGreaterThan(0);
    expect(FRAGMENT_SHADER_SRC).toContain("#version 300 es");
    expect(FRAGMENT_SHADER_SRC).toContain("fragColor");
  });
});

// ─── MockCanvas ──────────────────────────────────────────────────────────

describe("MockCanvas", () => {
  it("returns MockGL for webgl2 context", () => {
    const canvas = new MockCanvas(800, 600);
    const gl = canvas.getContext("webgl2");

    expect(gl).toBeInstanceOf(MockGL);
    expect(gl).not.toBeNull();
  });

  it("returns null for unsupported context types", () => {
    const canvas = new MockCanvas(800, 600);
    const gl = canvas.getContext("2d");

    expect(gl).toBeNull();
  });

  it("returns null when webgl2 is disabled", () => {
    const canvas = new MockCanvas(800, 600, false);
    const gl = canvas.getContext("webgl2");

    expect(gl).toBeNull();
  });
});
