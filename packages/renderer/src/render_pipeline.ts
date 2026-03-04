// @townbuilder/renderer — 7-pass render pipeline integration

import type { CameraState } from './projection/index.js';
import type { ChunkKey, ChunkCache } from './chunks/index.js';
import type { ChunkBuilder } from './chunks/index.js';
import type { DynamicEntity, SelectionHighlight, DynamicRenderer } from './dynamic/index.js';
import { OverlayType, type ZoneDisplayType, type OverlayRenderer } from './overlays/index.js';

// ─── Interfaces ─────────────────────────────────────────────────────────────

/** Frame render statistics */
export interface FrameStats {
  frameNumber: number;
  totalInstances: number;
  drawCalls: number;
  chunksDrawn: number;
  chunksRebuilt: number;
  dynamicInstances: number;
  overlayInstances: number;
  frameTimeMs: number;
  fps: number;
}

/** Render pipeline configuration */
export interface PipelineConfig {
  maxChunkRebuildsPerFrame: number;  // default 4
  enableOverlays: boolean;           // default true
  enableDynamic: boolean;            // default true
  enableShadows: boolean;            // default true
  targetFps: number;                 // default 60
}

/** Data sources for the render pipeline */
export interface RenderDataSources {
  /** Get tile data for chunk building */
  getTile: (x: number, y: number) => any;
  /** Get dynamic entities to render */
  getDynamicEntities: () => DynamicEntity[];
  /** Get selection highlights */
  getSelections: () => SelectionHighlight[];
  /** Get heatmap value for overlay */
  getHeatmapValue?: (x: number, y: number) => number;
  /** Get zone type for overlay */
  getZoneType?: (x: number, y: number) => ZoneDisplayType;
  /** Get status icons */
  getStatusIcons?: () => any[];
}

/** Render plan -- what to do this frame */
export interface RenderPlan {
  visibleChunks: ChunkKey[];
  dirtyChunks: ChunkKey[];
  cachedChunks: ChunkKey[];
  needsDynamicRebuild: boolean;
  needsOverlayRebuild: boolean;
}

// ─── Defaults ───────────────────────────────────────────────────────────────

export const DEFAULT_PIPELINE_CONFIG: PipelineConfig = {
  maxChunkRebuildsPerFrame: 4,
  enableOverlays: true,
  enableDynamic: true,
  enableShadows: true,
  targetFps: 60,
};

// ─── RenderPipeline ─────────────────────────────────────────────────────────

/**
 * Orchestrates the full 7-pass rendering frame.
 *
 * The 7 passes:
 *   0. Terrain (from chunks)
 *   1. Networks/Roads (from chunks)
 *   2. Buildings (from chunks)
 *   3. Props (from chunks)
 *   4. Automata/Dynamic (rebuilt each frame)
 *   5. Overlays (heatmaps, zoning)
 *   6. UI overlay (selection highlights, cursors)
 */
export class RenderPipeline {
  private config: PipelineConfig;
  private frameNumber: number;
  private lastFrameTime: number;
  private fpsAccumulator: number;
  private fpsFrameCount: number;
  private currentFps: number;
  private mapWidth: number;
  private mapHeight: number;

  constructor(mapWidth: number, mapHeight: number, config?: Partial<PipelineConfig>) {
    this.config = { ...DEFAULT_PIPELINE_CONFIG, ...config };
    this.frameNumber = 0;
    this.lastFrameTime = 0;
    this.fpsAccumulator = 0;
    this.fpsFrameCount = 0;
    this.currentFps = 0;
    this.mapWidth = mapWidth;
    this.mapHeight = mapHeight;
  }

  /** Get current config */
  getConfig(): PipelineConfig {
    return { ...this.config };
  }

  /** Update config */
  setConfig(config: Partial<PipelineConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Compute the render plan for this frame.
   * Determines which chunks need rebuild, which come from cache,
   * and whether dynamic/overlay passes are needed.
   */
  computeRenderPlan(
    camera: CameraState,
    chunkCache: ChunkCache,
  ): RenderPlan {
    const visibleChunks = chunkCache.getVisibleChunks(camera);
    const dirtyChunks = chunkCache.getDirtyVisible(camera);
    const cachedChunks = visibleChunks.filter(ck => {
      const chunk = chunkCache.getChunk(ck.cx, ck.cy);
      return chunk !== undefined && !chunk.dirty;
    });

    return {
      visibleChunks,
      dirtyChunks,
      cachedChunks,
      needsDynamicRebuild: this.config.enableDynamic,
      needsOverlayRebuild: this.config.enableOverlays,
    };
  }

  /**
   * Execute a full frame render.
   * Returns frame stats.
   *
   * The 7 passes:
   *   0. Terrain (from chunks)
   *   1. Networks/Roads (from chunks)
   *   2. Buildings (from chunks)
   *   3. Props (from chunks)
   *   4. Automata/Dynamic (rebuilt each frame)
   *   5. Overlays (heatmaps, zoning)
   *   6. UI overlay (selection highlights, cursors)
   */
  executeFrame(
    camera: CameraState,
    chunkCache: ChunkCache,
    chunkBuilder: ChunkBuilder,
    dataSources: RenderDataSources,
    dynamicRenderer: DynamicRenderer | null,
    overlayRenderer: OverlayRenderer | null,
  ): FrameStats {
    const startTime = performance.now();
    this.frameNumber++;

    let totalInstances = 0;
    let drawCalls = 0;
    let chunksRebuilt = 0;
    let dynamicInstances = 0;
    let overlayInstances = 0;

    // Step 1: Compute render plan
    const plan = this.computeRenderPlan(camera, chunkCache);

    // Step 2: Rebuild dirty chunks (time-budgeted)
    for (const chunk of plan.dirtyChunks) {
      const result = chunkBuilder.buildChunk(chunk.cx, chunk.cy, dataSources.getTile);
      chunkCache.setChunk(chunk.cx, chunk.cy, result.instances, result.count);
      chunksRebuilt++;
      totalInstances += result.count;
    }

    // Step 3: Count cached chunk instances
    for (const chunk of plan.cachedChunks) {
      const data = chunkCache.getChunk(chunk.cx, chunk.cy);
      if (data) {
        totalInstances += data.instanceCount;
        drawCalls++;
      }
    }

    // Step 4: Dynamic pass
    if (dynamicRenderer && plan.needsDynamicRebuild) {
      const entities = dataSources.getDynamicEntities();
      const selections = dataSources.getSelections();
      const result = dynamicRenderer.buildInstances(entities, selections);
      dynamicInstances = result.count;
      totalInstances += result.count;
      if (result.count > 0) drawCalls++;
    }

    // Step 5: Overlay pass
    if (overlayRenderer && plan.needsOverlayRebuild) {
      const activeOverlay = overlayRenderer.getOverlay();
      if (activeOverlay !== OverlayType.None && activeOverlay !== OverlayType.Zoning) {
        // Heatmap overlay
        if (dataSources.getHeatmapValue) {
          const result = overlayRenderer.buildHeatmapInstances(
            dataSources.getHeatmapValue,
            this.mapWidth,
            this.mapHeight,
          );
          overlayInstances += result.count;
          totalInstances += result.count;
          if (result.count > 0) drawCalls++;
        }
      } else if (activeOverlay === OverlayType.Zoning) {
        // Zoning overlay
        if (dataSources.getZoneType) {
          const result = overlayRenderer.buildZoningInstances(
            dataSources.getZoneType,
            this.mapWidth,
            this.mapHeight,
          );
          overlayInstances += result.count;
          totalInstances += result.count;
          if (result.count > 0) drawCalls++;
        }
      }
    }

    // Step 6: LRU eviction
    chunkCache.evictLRU(this.frameNumber);

    // Compute FPS
    const endTime = performance.now();
    const frameTimeMs = endTime - startTime;
    this.fpsAccumulator += frameTimeMs;
    this.fpsFrameCount++;
    if (this.fpsAccumulator >= 1000) {
      this.currentFps = Math.round(this.fpsFrameCount / (this.fpsAccumulator / 1000));
      this.fpsAccumulator = 0;
      this.fpsFrameCount = 0;
    }

    return {
      frameNumber: this.frameNumber,
      totalInstances,
      drawCalls,
      chunksDrawn: plan.visibleChunks.length,
      chunksRebuilt,
      dynamicInstances,
      overlayInstances,
      frameTimeMs,
      fps: this.currentFps,
    };
  }

  /** Get current frame number */
  getFrameNumber(): number {
    return this.frameNumber;
  }

  /** Get current FPS */
  getFps(): number {
    return this.currentFps;
  }

  /** Reset frame counter */
  reset(): void {
    this.frameNumber = 0;
    this.lastFrameTime = 0;
    this.fpsAccumulator = 0;
    this.fpsFrameCount = 0;
    this.currentFps = 0;
  }
}
