import { describe, it, expect, vi } from 'vitest';
import {
  RenderPipeline,
  DEFAULT_PIPELINE_CONFIG,
  type PipelineConfig,
  type RenderDataSources,
  type FrameStats,
  type RenderPlan,
} from '../render_pipeline.js';
import { OverlayType } from '../overlays/index.js';
import type { CameraState } from '../projection/index.js';
import type { ChunkKey, ChunkData, ChunkCache } from '../chunks/index.js';
import type { ChunkBuilder } from '../chunks/index.js';
import type { DynamicRenderer } from '../dynamic/index.js';
import type { OverlayRenderer } from '../overlays/index.js';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function makeCamera(overrides: Partial<CameraState> = {}): CameraState {
  return {
    x: 0,
    y: 0,
    zoom: 1,
    viewportWidth: 800,
    viewportHeight: 600,
    ...overrides,
  };
}

function makeChunkCache(options: {
  visibleChunks?: ChunkKey[];
  dirtyChunks?: ChunkKey[];
  chunkData?: Map<string, ChunkData>;
} = {}): ChunkCache {
  const {
    visibleChunks = [{ cx: 0, cy: 0 }],
    dirtyChunks = [],
    chunkData = new Map<string, ChunkData>(),
  } = options;

  return {
    getVisibleChunks: vi.fn().mockReturnValue(visibleChunks),
    getDirtyVisible: vi.fn().mockReturnValue(dirtyChunks),
    getChunk: vi.fn((cx: number, cy: number) => {
      return chunkData.get(`${cx},${cy}`);
    }),
    setChunk: vi.fn(),
    evictLRU: vi.fn().mockReturnValue(0),
  } as unknown as ChunkCache;
}

function makeChunkBuilder(buildResult?: { instances: Float32Array; count: number }): ChunkBuilder {
  const defaultResult = { instances: new Float32Array(0), count: 5 };
  return {
    buildChunk: vi.fn().mockReturnValue(buildResult ?? defaultResult),
  } as unknown as ChunkBuilder;
}

function makeDynamicRenderer(buildResult?: { instances: Float32Array; count: number; stats: any }): DynamicRenderer {
  const defaultResult = {
    instances: new Float32Array(0),
    count: 3,
    stats: { entityCount: 3, instanceCount: 3, interpolatedCount: 0 },
  };
  return {
    buildInstances: vi.fn().mockReturnValue(buildResult ?? defaultResult),
    updateCamera: vi.fn(),
    updateTiming: vi.fn(),
  } as unknown as DynamicRenderer;
}

function makeOverlayRenderer(options: {
  overlay?: OverlayType;
  heatmapResult?: { instances: Float32Array; count: number; stats: any };
  zoningResult?: { instances: Float32Array; count: number; stats: any };
} = {}): OverlayRenderer {
  const { overlay = OverlayType.None } = options;
  const defaultHeatmap = {
    instances: new Float32Array(0),
    count: 10,
    stats: { tileCount: 10, instanceCount: 10, activeOverlay: overlay },
  };
  const defaultZoning = {
    instances: new Float32Array(0),
    count: 7,
    stats: { tileCount: 7, instanceCount: 7, activeOverlay: OverlayType.Zoning },
  };
  return {
    getOverlay: vi.fn().mockReturnValue(overlay),
    setOverlay: vi.fn(),
    buildHeatmapInstances: vi.fn().mockReturnValue(options.heatmapResult ?? defaultHeatmap),
    buildZoningInstances: vi.fn().mockReturnValue(options.zoningResult ?? defaultZoning),
    updateCamera: vi.fn(),
  } as unknown as OverlayRenderer;
}

function makeDataSources(overrides: Partial<RenderDataSources> = {}): RenderDataSources {
  return {
    getTile: vi.fn().mockReturnValue(null),
    getDynamicEntities: vi.fn().mockReturnValue([]),
    getSelections: vi.fn().mockReturnValue([]),
    getHeatmapValue: vi.fn().mockReturnValue(0),
    getZoneType: vi.fn().mockReturnValue(0),
    ...overrides,
  };
}

// ─── Constructor Tests ───────────────────────────────────────────────────────

describe('RenderPipeline constructor', () => {
  it('creates pipeline with default config', () => {
    const pipeline = new RenderPipeline(256, 256);
    const config = pipeline.getConfig();
    expect(config.maxChunkRebuildsPerFrame).toBe(4);
    expect(config.enableOverlays).toBe(true);
    expect(config.enableDynamic).toBe(true);
    expect(config.enableShadows).toBe(true);
    expect(config.targetFps).toBe(60);
  });

  it('merges partial config with defaults', () => {
    const pipeline = new RenderPipeline(256, 256, {
      enableOverlays: false,
      targetFps: 30,
    });
    const config = pipeline.getConfig();
    expect(config.enableOverlays).toBe(false);
    expect(config.targetFps).toBe(30);
    // Defaults preserved
    expect(config.maxChunkRebuildsPerFrame).toBe(4);
    expect(config.enableDynamic).toBe(true);
    expect(config.enableShadows).toBe(true);
  });

  it('starts at frame zero', () => {
    const pipeline = new RenderPipeline(256, 256);
    expect(pipeline.getFrameNumber()).toBe(0);
    expect(pipeline.getFps()).toBe(0);
  });
});

// ─── setConfig Tests ─────────────────────────────────────────────────────────

describe('RenderPipeline.setConfig', () => {
  it('updates configuration partially', () => {
    const pipeline = new RenderPipeline(256, 256);
    pipeline.setConfig({ enableDynamic: false });
    const config = pipeline.getConfig();
    expect(config.enableDynamic).toBe(false);
    // Others unchanged
    expect(config.enableOverlays).toBe(true);
    expect(config.targetFps).toBe(60);
  });
});

// ─── computeRenderPlan Tests ─────────────────────────────────────────────────

describe('RenderPipeline.computeRenderPlan', () => {
  it('returns visible chunks from cache', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();
    const visibleChunks = [{ cx: 0, cy: 0 }, { cx: 1, cy: 0 }];
    const chunkCache = makeChunkCache({ visibleChunks });

    const plan = pipeline.computeRenderPlan(camera, chunkCache);
    expect(plan.visibleChunks).toEqual(visibleChunks);
    expect(chunkCache.getVisibleChunks).toHaveBeenCalledWith(camera);
  });

  it('identifies dirty chunks', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();
    const dirtyChunks = [{ cx: 0, cy: 0 }];
    const chunkCache = makeChunkCache({
      visibleChunks: [{ cx: 0, cy: 0 }, { cx: 1, cy: 0 }],
      dirtyChunks,
    });

    const plan = pipeline.computeRenderPlan(camera, chunkCache);
    expect(plan.dirtyChunks).toEqual(dirtyChunks);
  });

  it('identifies cached (non-dirty) chunks', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();

    const chunkData = new Map<string, ChunkData>();
    chunkData.set('1,0', {
      key: { cx: 1, cy: 0 },
      instances: new Float32Array(48),
      instanceCount: 4,
      dirty: false,
      lastUsedFrame: 0,
      version: 1,
    });
    // Chunk at 0,0 is not in cache => not a "cached" chunk
    const chunkCache = makeChunkCache({
      visibleChunks: [{ cx: 0, cy: 0 }, { cx: 1, cy: 0 }],
      dirtyChunks: [{ cx: 0, cy: 0 }],
      chunkData,
    });

    const plan = pipeline.computeRenderPlan(camera, chunkCache);
    expect(plan.cachedChunks).toEqual([{ cx: 1, cy: 0 }]);
  });

  it('sets needsDynamicRebuild based on config', () => {
    const pipeline = new RenderPipeline(256, 256, { enableDynamic: false });
    const camera = makeCamera();
    const chunkCache = makeChunkCache();

    const plan = pipeline.computeRenderPlan(camera, chunkCache);
    expect(plan.needsDynamicRebuild).toBe(false);
  });

  it('sets needsOverlayRebuild based on config', () => {
    const pipeline = new RenderPipeline(256, 256, { enableOverlays: false });
    const camera = makeCamera();
    const chunkCache = makeChunkCache();

    const plan = pipeline.computeRenderPlan(camera, chunkCache);
    expect(plan.needsOverlayRebuild).toBe(false);
  });
});

// ─── executeFrame Tests ──────────────────────────────────────────────────────

describe('RenderPipeline.executeFrame', () => {
  it('increments frame number', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    const stats1 = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);
    expect(stats1.frameNumber).toBe(1);

    const stats2 = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);
    expect(stats2.frameNumber).toBe(2);

    expect(pipeline.getFrameNumber()).toBe(2);
  });

  it('rebuilds dirty chunks via chunkBuilder', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();
    const dirtyChunks = [{ cx: 0, cy: 0 }, { cx: 1, cy: 0 }];
    const chunkCache = makeChunkCache({
      visibleChunks: dirtyChunks,
      dirtyChunks,
    });
    const buildResult = { instances: new Float32Array(0), count: 8 };
    const chunkBuilder = makeChunkBuilder(buildResult);
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);

    expect(chunkBuilder.buildChunk).toHaveBeenCalledTimes(2);
    expect(chunkCache.setChunk).toHaveBeenCalledTimes(2);
    expect(stats.chunksRebuilt).toBe(2);
    // 2 dirty chunks * 8 instances each = 16 total from rebuild
    expect(stats.totalInstances).toBe(16);
  });

  it('counts cached chunk instances', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();

    const chunkData = new Map<string, ChunkData>();
    chunkData.set('0,0', {
      key: { cx: 0, cy: 0 },
      instances: new Float32Array(0),
      instanceCount: 12,
      dirty: false,
      lastUsedFrame: 0,
      version: 1,
    });
    chunkData.set('1,0', {
      key: { cx: 1, cy: 0 },
      instances: new Float32Array(0),
      instanceCount: 7,
      dirty: false,
      lastUsedFrame: 0,
      version: 1,
    });

    const chunkCache = makeChunkCache({
      visibleChunks: [{ cx: 0, cy: 0 }, { cx: 1, cy: 0 }],
      dirtyChunks: [],
      chunkData,
    });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);

    // 12 + 7 = 19 cached instances
    expect(stats.totalInstances).toBe(19);
    expect(stats.drawCalls).toBe(2);
    expect(stats.chunksRebuilt).toBe(0);
  });

  it('includes dynamic instances when enabled', () => {
    const pipeline = new RenderPipeline(256, 256, { enableDynamic: true });
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dynamicResult = {
      instances: new Float32Array(0),
      count: 5,
      stats: { entityCount: 5, instanceCount: 5, interpolatedCount: 0 },
    };
    const dynamicRenderer = makeDynamicRenderer(dynamicResult);
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, dynamicRenderer, null);

    expect(dynamicRenderer.buildInstances).toHaveBeenCalled();
    expect(stats.dynamicInstances).toBe(5);
    expect(stats.totalInstances).toBe(5);
    expect(stats.drawCalls).toBe(1);
  });

  it('skips dynamic when disabled', () => {
    const pipeline = new RenderPipeline(256, 256, { enableDynamic: false });
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dynamicRenderer = makeDynamicRenderer();
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, dynamicRenderer, null);

    expect(dynamicRenderer.buildInstances).not.toHaveBeenCalled();
    expect(stats.dynamicInstances).toBe(0);
  });

  it('includes overlay instances when heatmap active', () => {
    const pipeline = new RenderPipeline(64, 64, { enableOverlays: true });
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const heatmapResult = {
      instances: new Float32Array(0),
      count: 15,
      stats: { tileCount: 15, instanceCount: 15, activeOverlay: OverlayType.Traffic },
    };
    const overlayRenderer = makeOverlayRenderer({
      overlay: OverlayType.Traffic,
      heatmapResult,
    });
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, overlayRenderer);

    expect(overlayRenderer.buildHeatmapInstances).toHaveBeenCalledWith(
      dataSources.getHeatmapValue,
      64,
      64,
    );
    expect(stats.overlayInstances).toBe(15);
    expect(stats.totalInstances).toBe(15);
    expect(stats.drawCalls).toBe(1);
  });

  it('skips overlays when None active', () => {
    const pipeline = new RenderPipeline(256, 256, { enableOverlays: true });
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const overlayRenderer = makeOverlayRenderer({ overlay: OverlayType.None });
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, overlayRenderer);

    expect(overlayRenderer.buildHeatmapInstances).not.toHaveBeenCalled();
    expect(overlayRenderer.buildZoningInstances).not.toHaveBeenCalled();
    expect(stats.overlayInstances).toBe(0);
  });

  it('uses zoning path when Zoning overlay active', () => {
    const pipeline = new RenderPipeline(64, 64, { enableOverlays: true });
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const zoningResult = {
      instances: new Float32Array(0),
      count: 9,
      stats: { tileCount: 9, instanceCount: 9, activeOverlay: OverlayType.Zoning },
    };
    const overlayRenderer = makeOverlayRenderer({
      overlay: OverlayType.Zoning,
      zoningResult,
    });
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, overlayRenderer);

    expect(overlayRenderer.buildZoningInstances).toHaveBeenCalledWith(
      dataSources.getZoneType,
      64,
      64,
    );
    expect(overlayRenderer.buildHeatmapInstances).not.toHaveBeenCalled();
    expect(stats.overlayInstances).toBe(9);
  });

  it('runs LRU eviction each frame', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);

    expect(chunkCache.evictLRU).toHaveBeenCalledWith(1);
  });

  it('returns valid FrameStats', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [{ cx: 0, cy: 0 }], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);

    expect(stats).toHaveProperty('frameNumber');
    expect(stats).toHaveProperty('totalInstances');
    expect(stats).toHaveProperty('drawCalls');
    expect(stats).toHaveProperty('chunksDrawn');
    expect(stats).toHaveProperty('chunksRebuilt');
    expect(stats).toHaveProperty('dynamicInstances');
    expect(stats).toHaveProperty('overlayInstances');
    expect(stats).toHaveProperty('frameTimeMs');
    expect(stats).toHaveProperty('fps');
    expect(stats.frameNumber).toBe(1);
    expect(stats.chunksDrawn).toBe(1);
    expect(typeof stats.frameTimeMs).toBe('number');
    expect(stats.frameTimeMs).toBeGreaterThanOrEqual(0);
  });

  it('skips dynamic when dynamicRenderer is null', () => {
    const pipeline = new RenderPipeline(256, 256, { enableDynamic: true });
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);

    expect(stats.dynamicInstances).toBe(0);
  });

  it('skips overlay when overlayRenderer is null', () => {
    const pipeline = new RenderPipeline(256, 256, { enableOverlays: true });
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    const stats = pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);

    expect(stats.overlayInstances).toBe(0);
  });
});

// ─── reset Tests ─────────────────────────────────────────────────────────────

describe('RenderPipeline.reset', () => {
  it('clears frame counter', () => {
    const pipeline = new RenderPipeline(256, 256);
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    // Execute a few frames
    pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);
    pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);
    expect(pipeline.getFrameNumber()).toBe(2);

    pipeline.reset();
    expect(pipeline.getFrameNumber()).toBe(0);
    expect(pipeline.getFps()).toBe(0);
  });
});

// ─── getFps Tests ────────────────────────────────────────────────────────────

describe('RenderPipeline.getFps', () => {
  it('returns 0 initially', () => {
    const pipeline = new RenderPipeline(256, 256);
    expect(pipeline.getFps()).toBe(0);
  });

  it('returns current FPS value', () => {
    const pipeline = new RenderPipeline(256, 256);
    // FPS only updates when fpsAccumulator >= 1000ms, so initially stays 0
    const camera = makeCamera();
    const chunkCache = makeChunkCache({ visibleChunks: [], dirtyChunks: [] });
    const chunkBuilder = makeChunkBuilder();
    const dataSources = makeDataSources();

    pipeline.executeFrame(camera, chunkCache, chunkBuilder, dataSources, null, null);
    // FPS will still be 0 since accumulated time < 1000ms
    expect(typeof pipeline.getFps()).toBe('number');
    expect(pipeline.getFps()).toBeGreaterThanOrEqual(0);
  });
});

// ─── DEFAULT_PIPELINE_CONFIG Tests ───────────────────────────────────────────

describe('DEFAULT_PIPELINE_CONFIG', () => {
  it('has expected default values', () => {
    expect(DEFAULT_PIPELINE_CONFIG.maxChunkRebuildsPerFrame).toBe(4);
    expect(DEFAULT_PIPELINE_CONFIG.enableOverlays).toBe(true);
    expect(DEFAULT_PIPELINE_CONFIG.enableDynamic).toBe(true);
    expect(DEFAULT_PIPELINE_CONFIG.enableShadows).toBe(true);
    expect(DEFAULT_PIPELINE_CONFIG.targetFps).toBe(60);
  });
});
