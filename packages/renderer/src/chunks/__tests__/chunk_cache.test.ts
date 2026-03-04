import { describe, it, expect } from 'vitest';
import {
  CHUNK_SIZE,
  MAX_CACHED_CHUNKS,
  MAX_REBUILD_PER_FRAME,
  ChunkCache,
} from '../chunk_cache.js';
import type { CameraState } from '../../projection/index.js';

/** Camera centered at (50,50) with a standard viewport. */
const defaultCamera: CameraState = {
  x: 50,
  y: 50,
  zoom: 1.0,
  viewportWidth: 1920,
  viewportHeight: 1080,
};

describe('ChunkCache', () => {
  // ─── getChunkKey ──────────────────────────────────────────────────

  it('getChunkKey returns correct chunk for tile coordinates', () => {
    const cache = new ChunkCache(256, 256);
    const key = cache.getChunkKey(10, 20);
    expect(key.cx).toBe(0);
    expect(key.cy).toBe(0);

    const key2 = cache.getChunkKey(64, 96);
    expect(key2.cx).toBe(2);
    expect(key2.cy).toBe(3);
  });

  it('getChunkKey handles edge case at chunk boundary (tile 31 vs 32)', () => {
    const cache = new ChunkCache(256, 256);

    // Tile 31 is the last tile in chunk 0
    const key31 = cache.getChunkKey(31, 31);
    expect(key31.cx).toBe(0);
    expect(key31.cy).toBe(0);

    // Tile 32 is the first tile in chunk 1
    const key32 = cache.getChunkKey(32, 32);
    expect(key32.cx).toBe(1);
    expect(key32.cy).toBe(1);
  });

  // ─── markDirty ────────────────────────────────────────────────────

  it('markDirty marks the correct chunk', () => {
    const cache = new ChunkCache(256, 256);
    // Store a chunk first
    cache.setChunk(1, 1, new Float32Array(48), 1);
    expect(cache.getChunk(1, 1)!.dirty).toBe(false);

    // Mark a tile in chunk (1,1) as dirty
    cache.markDirty(40, 40);
    expect(cache.getChunk(1, 1)!.dirty).toBe(true);
  });

  it('markAllDirty marks all cached chunks', () => {
    const cache = new ChunkCache(256, 256);
    cache.setChunk(0, 0, new Float32Array(48), 1);
    cache.setChunk(1, 0, new Float32Array(48), 1);
    cache.setChunk(0, 1, new Float32Array(48), 1);

    // All should be clean after setChunk
    expect(cache.getChunk(0, 0)!.dirty).toBe(false);
    expect(cache.getChunk(1, 0)!.dirty).toBe(false);
    expect(cache.getChunk(0, 1)!.dirty).toBe(false);

    cache.markAllDirty();

    expect(cache.getChunk(0, 0)!.dirty).toBe(true);
    expect(cache.getChunk(1, 0)!.dirty).toBe(true);
    expect(cache.getChunk(0, 1)!.dirty).toBe(true);
  });

  // ─── setChunk / getChunk ──────────────────────────────────────────

  it('setChunk stores and retrieves chunk data', () => {
    const cache = new ChunkCache(256, 256);
    const instances = new Float32Array(96); // 2 instances x 48 bytes / 4 bytes
    cache.setChunk(2, 3, instances, 2);

    const chunk = cache.getChunk(2, 3);
    expect(chunk).toBeDefined();
    expect(chunk!.key).toEqual({ cx: 2, cy: 3 });
    expect(chunk!.instanceCount).toBe(2);
    expect(chunk!.instances).toBe(instances);
    expect(chunk!.dirty).toBe(false);
  });

  it('getChunk returns undefined for non-cached chunk', () => {
    const cache = new ChunkCache(256, 256);
    expect(cache.getChunk(5, 5)).toBeUndefined();
  });

  // ─── getVisibleChunks ─────────────────────────────────────────────

  it('getVisibleChunks returns chunks in camera viewport', () => {
    const cache = new ChunkCache(256, 256);
    const visible = cache.getVisibleChunks(defaultCamera);

    // Should return at least one chunk
    expect(visible.length).toBeGreaterThan(0);

    // All returned chunks should be in valid range
    for (const key of visible) {
      expect(key.cx).toBeGreaterThanOrEqual(0);
      expect(key.cx).toBeLessThan(cache.chunksX);
      expect(key.cy).toBeGreaterThanOrEqual(0);
      expect(key.cy).toBeLessThan(cache.chunksY);
    }
  });

  it('getVisibleChunks clamps to valid range', () => {
    // Small map with a camera looking way outside
    const cache = new ChunkCache(64, 64); // 2x2 chunks
    const farCamera: CameraState = {
      x: -1000,
      y: -1000,
      zoom: 0.1,
      viewportWidth: 1920,
      viewportHeight: 1080,
    };

    const visible = cache.getVisibleChunks(farCamera);
    for (const key of visible) {
      expect(key.cx).toBeGreaterThanOrEqual(0);
      expect(key.cx).toBeLessThan(cache.chunksX);
      expect(key.cy).toBeGreaterThanOrEqual(0);
      expect(key.cy).toBeLessThan(cache.chunksY);
    }
  });

  // ─── getDirtyVisible ──────────────────────────────────────────────

  it('getDirtyVisible limits to MAX_REBUILD_PER_FRAME', () => {
    const cache = new ChunkCache(512, 512);

    // Add many chunks and mark them dirty
    for (let cx = 0; cx < 8; cx++) {
      for (let cy = 0; cy < 8; cy++) {
        cache.setChunk(cx, cy, new Float32Array(48), 1);
      }
    }
    cache.markAllDirty();

    // Camera that sees many chunks
    const wideCam: CameraState = {
      x: 128,
      y: 128,
      zoom: 0.1,
      viewportWidth: 3840,
      viewportHeight: 2160,
    };

    const dirty = cache.getDirtyVisible(wideCam);
    expect(dirty.length).toBeLessThanOrEqual(MAX_REBUILD_PER_FRAME);
  });

  it('getDirtyVisible only returns dirty chunks', () => {
    const cache = new ChunkCache(256, 256);

    // Set up some chunks, only mark some dirty
    cache.setChunk(1, 1, new Float32Array(48), 1);
    cache.setChunk(1, 2, new Float32Array(48), 1);
    cache.markChunkDirty(1, 1);
    // chunk (1,2) stays clean

    // Camera centered on these chunks
    const cam: CameraState = {
      x: 48,
      y: 48,
      zoom: 1.0,
      viewportWidth: 1920,
      viewportHeight: 1080,
    };

    const dirty = cache.getDirtyVisible(cam);

    // The dirty result should include chunk (1,1) which is dirty
    // It may also include chunks that are not cached (which count as needing rebuild)
    // But it should NOT include chunk (1,2) since it is cached and clean
    const hasCleanCached = dirty.some(k => k.cx === 1 && k.cy === 2);
    expect(hasCleanCached).toBe(false);
  });

  // ─── evictLRU ─────────────────────────────────────────────────────

  it('evictLRU removes old chunks beyond MAX_CACHED_CHUNKS', () => {
    const cache = new ChunkCache(10000, 10000); // large map to avoid chunk limit issues

    // Fill cache beyond MAX_CACHED_CHUNKS
    const totalToAdd = MAX_CACHED_CHUNKS + 10;
    for (let i = 0; i < totalToAdd; i++) {
      cache.setChunk(i, 0, new Float32Array(48), 1);
    }

    expect(cache.getStats().cachedChunks).toBe(totalToAdd);

    // Evict with a high frame number so all are stale
    const evicted = cache.evictLRU(1000);
    expect(evicted).toBe(10);
    expect(cache.getStats().cachedChunks).toBe(MAX_CACHED_CHUNKS);
  });

  // ─── clear ────────────────────────────────────────────────────────

  it('clear removes all chunks', () => {
    const cache = new ChunkCache(256, 256);
    cache.setChunk(0, 0, new Float32Array(48), 1);
    cache.setChunk(1, 1, new Float32Array(48), 1);
    cache.setChunk(2, 2, new Float32Array(48), 1);

    expect(cache.getStats().cachedChunks).toBe(3);

    cache.clear();
    expect(cache.getStats().cachedChunks).toBe(0);
    expect(cache.getChunk(0, 0)).toBeUndefined();
  });

  // ─── Stats ────────────────────────────────────────────────────────

  it('stats reflect current state accurately', () => {
    const cache = new ChunkCache(128, 128); // 4x4 chunks

    const emptyStats = cache.getStats();
    expect(emptyStats.totalChunks).toBe(16); // 4*4
    expect(emptyStats.cachedChunks).toBe(0);
    expect(emptyStats.dirtyChunks).toBe(0);

    cache.setChunk(0, 0, new Float32Array(48), 1);
    cache.setChunk(1, 0, new Float32Array(48), 1);
    cache.setChunk(2, 0, new Float32Array(48), 1);

    const afterAdd = cache.getStats();
    expect(afterAdd.cachedChunks).toBe(3);
    expect(afterAdd.dirtyChunks).toBe(0);

    cache.markChunkDirty(0, 0);
    cache.markChunkDirty(2, 0);

    const afterDirty = cache.getStats();
    expect(afterDirty.dirtyChunks).toBe(2);
  });

  // ─── Version tracking ─────────────────────────────────────────────

  it('setChunk updates version counter', () => {
    const cache = new ChunkCache(256, 256);

    cache.setChunk(0, 0, new Float32Array(48), 1);
    expect(cache.getChunk(0, 0)!.version).toBe(1);

    cache.setChunk(0, 0, new Float32Array(96), 2);
    expect(cache.getChunk(0, 0)!.version).toBe(2);

    cache.setChunk(0, 0, new Float32Array(144), 3);
    expect(cache.getChunk(0, 0)!.version).toBe(3);
  });

  // ─── Constructor ──────────────────────────────────────────────────

  it('constructor computes chunksX and chunksY correctly', () => {
    // Exact multiple
    const cache1 = new ChunkCache(128, 64);
    expect(cache1.chunksX).toBe(4); // 128 / 32
    expect(cache1.chunksY).toBe(2); // 64 / 32

    // Non-exact: 100/32 = 3.125 -> ceil = 4
    const cache2 = new ChunkCache(100, 50);
    expect(cache2.chunksX).toBe(4);
    expect(cache2.chunksY).toBe(2); // 50/32 = 1.5625 -> ceil = 2
  });
});
