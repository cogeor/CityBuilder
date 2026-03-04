// @townbuilder/renderer — Chunk cache for isometric tile rendering

import { visibleTileRange, type CameraState } from '../projection/index.js';

// ─── Constants ──────────────────────────────────────────────────────────────

/** Number of tiles per chunk side. */
export const CHUNK_SIZE = 32;

/** Maximum number of chunks to keep in cache. */
export const MAX_CACHED_CHUNKS = 256;

/** Maximum number of chunks to rebuild per frame. */
export const MAX_REBUILD_PER_FRAME = 4;

// ─── Interfaces ─────────────────────────────────────────────────────────────

/** Identifies a chunk by its grid indices. */
export interface ChunkKey {
  cx: number;
  cy: number;
}

/** Cached data for a single chunk. */
export interface ChunkData {
  key: ChunkKey;
  /** Pre-sorted render instances (48 bytes each). */
  instances: Float32Array;
  instanceCount: number;
  dirty: boolean;
  lastUsedFrame: number;
  version: number;
}

/** Runtime statistics for the chunk cache. */
export interface ChunkCacheStats {
  totalChunks: number;
  dirtyChunks: number;
  cachedChunks: number;
  rebuiltThisFrame: number;
  evictedThisFrame: number;
}

// ─── ChunkCache ─────────────────────────────────────────────────────────────

/**
 * Cache for 32x32 tile chunks with dirty tracking and LRU eviction.
 *
 * Chunks are identified by (cx, cy) grid indices. Each chunk covers
 * CHUNK_SIZE x CHUNK_SIZE tiles. The cache stores pre-built render
 * instance data and tracks which chunks need rebuilding.
 */
export class ChunkCache {
  /** Number of chunks along the X axis. */
  readonly chunksX: number;
  /** Number of chunks along the Y axis. */
  readonly chunksY: number;

  private readonly cache = new Map<string, ChunkData>();
  private rebuiltThisFrame = 0;
  private evictedThisFrame = 0;

  constructor(
    readonly mapWidth: number,
    readonly mapHeight: number,
  ) {
    this.chunksX = Math.ceil(mapWidth / CHUNK_SIZE);
    this.chunksY = Math.ceil(mapHeight / CHUNK_SIZE);
  }

  // ─── Key Helpers ────────────────────────────────────────────────────

  /** Compute which chunk a tile belongs to. */
  getChunkKey(tileX: number, tileY: number): ChunkKey {
    return {
      cx: Math.floor(tileX / CHUNK_SIZE),
      cy: Math.floor(tileY / CHUNK_SIZE),
    };
  }

  private static keyStr(cx: number, cy: number): string {
    return `${cx},${cy}`;
  }

  // ─── Dirty Tracking ────────────────────────────────────────────────

  /** Mark the chunk containing the given tile as dirty. */
  markDirty(tileX: number, tileY: number): void {
    const { cx, cy } = this.getChunkKey(tileX, tileY);
    this.markChunkDirty(cx, cy);
  }

  /** Mark a specific chunk dirty by chunk indices. */
  markChunkDirty(cx: number, cy: number): void {
    const key = ChunkCache.keyStr(cx, cy);
    const chunk = this.cache.get(key);
    if (chunk) {
      chunk.dirty = true;
    }
  }

  /** Mark all cached chunks as dirty (full rebuild). */
  markAllDirty(): void {
    for (const chunk of this.cache.values()) {
      chunk.dirty = true;
    }
  }

  // ─── Cache Access ──────────────────────────────────────────────────

  /** Get cached chunk data. Updates lastUsedFrame on access. */
  getChunk(cx: number, cy: number): ChunkData | undefined {
    const key = ChunkCache.keyStr(cx, cy);
    return this.cache.get(key);
  }

  /** Store rebuilt chunk data. */
  setChunk(cx: number, cy: number, instances: Float32Array, instanceCount: number): void {
    const key = ChunkCache.keyStr(cx, cy);
    const existing = this.cache.get(key);
    const version = existing ? existing.version + 1 : 1;

    this.cache.set(key, {
      key: { cx, cy },
      instances,
      instanceCount,
      dirty: false,
      lastUsedFrame: 0,
      version,
    });
  }

  // ─── Visibility ────────────────────────────────────────────────────

  /**
   * Return chunk keys overlapping the camera viewport.
   *
   * Uses the projection module's visibleTileRange to find visible tiles,
   * then computes which chunks those tiles belong to. Results are clamped
   * to the valid chunk range.
   */
  getVisibleChunks(cameraState: CameraState): ChunkKey[] {
    const range = visibleTileRange(cameraState);

    // Convert tile range to chunk range
    const minCX = Math.max(0, Math.floor(range.minX / CHUNK_SIZE));
    const minCY = Math.max(0, Math.floor(range.minY / CHUNK_SIZE));
    const maxCX = Math.min(this.chunksX - 1, Math.floor(range.maxX / CHUNK_SIZE));
    const maxCY = Math.min(this.chunksY - 1, Math.floor(range.maxY / CHUNK_SIZE));

    const keys: ChunkKey[] = [];
    for (let cx = minCX; cx <= maxCX; cx++) {
      for (let cy = minCY; cy <= maxCY; cy++) {
        keys.push({ cx, cy });

        // Update lastUsedFrame for cached chunks
        const cached = this.cache.get(ChunkCache.keyStr(cx, cy));
        if (cached) {
          cached.lastUsedFrame = this.currentFrame;
        }
      }
    }
    return keys;
  }

  /** Current frame counter for LRU tracking. */
  private currentFrame = 0;

  /** Set the current frame number (call once per frame). */
  setFrame(frame: number): void {
    this.currentFrame = frame;
    this.rebuiltThisFrame = 0;
    this.evictedThisFrame = 0;
  }

  /**
   * Return visible chunks that are dirty, limited to MAX_REBUILD_PER_FRAME.
   */
  getDirtyVisible(cameraState: CameraState): ChunkKey[] {
    const visible = this.getVisibleChunks(cameraState);
    const dirty: ChunkKey[] = [];

    for (const key of visible) {
      if (dirty.length >= MAX_REBUILD_PER_FRAME) break;
      const cached = this.cache.get(ChunkCache.keyStr(key.cx, key.cy));
      if (!cached || cached.dirty) {
        dirty.push(key);
      }
    }

    return dirty;
  }

  // ─── Eviction ──────────────────────────────────────────────────────

  /**
   * Evict chunks not used in the last 60 frames, up to bringing count
   * back to MAX_CACHED_CHUNKS. Returns number of evicted chunks.
   */
  evictLRU(currentFrame: number): number {
    let evicted = 0;

    if (this.cache.size <= MAX_CACHED_CHUNKS) {
      return 0;
    }

    // Collect entries eligible for eviction (not used in last 60 frames)
    const staleThreshold = currentFrame - 60;
    const candidates: string[] = [];

    for (const [key, chunk] of this.cache) {
      if (chunk.lastUsedFrame < staleThreshold) {
        candidates.push(key);
      }
    }

    // Sort by lastUsedFrame ascending (oldest first)
    candidates.sort((a, b) => {
      const chunkA = this.cache.get(a)!;
      const chunkB = this.cache.get(b)!;
      return chunkA.lastUsedFrame - chunkB.lastUsedFrame;
    });

    // Evict until we are at or below MAX_CACHED_CHUNKS
    for (const key of candidates) {
      if (this.cache.size <= MAX_CACHED_CHUNKS) break;
      this.cache.delete(key);
      evicted++;
    }

    this.evictedThisFrame = evicted;
    return evicted;
  }

  // ─── Stats ─────────────────────────────────────────────────────────

  /** Return current cache statistics. */
  getStats(): ChunkCacheStats {
    let dirtyCount = 0;
    for (const chunk of this.cache.values()) {
      if (chunk.dirty) dirtyCount++;
    }

    return {
      totalChunks: this.chunksX * this.chunksY,
      dirtyChunks: dirtyCount,
      cachedChunks: this.cache.size,
      rebuiltThisFrame: this.rebuiltThisFrame,
      evictedThisFrame: this.evictedThisFrame,
    };
  }

  /** Remove all cached chunks. */
  clear(): void {
    this.cache.clear();
    this.rebuiltThisFrame = 0;
    this.evictedThisFrame = 0;
  }
}
