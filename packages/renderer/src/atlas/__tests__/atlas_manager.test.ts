import { describe, it, expect, beforeEach } from "vitest";
import {
  AtlasManager,
  ResolutionTier,
  type AtlasMetadata,
  type SpriteFrame,
} from "../atlas_manager.js";

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Build a minimal SpriteFrame for testing. */
function makeFrame(
  sprite_id: number,
  atlas_id: number,
  u = 0,
  v = 0,
  w = 32,
  h = 32,
): SpriteFrame {
  return {
    sprite_id,
    atlas_id,
    uv: { u, v, w, h },
    pivot: { x: w / 2, y: h / 2 },
    width: w,
    height: h,
  };
}

/** Build a minimal AtlasMetadata for testing. */
function makeAtlas(
  atlas_id: number,
  frames: SpriteFrame[],
  width = 512,
  height = 512,
): AtlasMetadata {
  return { atlas_id, width, height, frames };
}

/**
 * Build an .atlasbin ArrayBuffer from an AtlasMetadata.
 * Uses little-endian encoding matching the spec.
 */
function buildBinary(meta: AtlasMetadata): ArrayBuffer {
  const HEADER = 8;
  const PER_FRAME = 18;
  const buf = new ArrayBuffer(HEADER + meta.frames.length * PER_FRAME);
  const view = new DataView(buf);

  // Header
  view.setUint16(0, meta.atlas_id, true);
  view.setUint16(2, meta.width, true);
  view.setUint16(4, meta.height, true);
  view.setUint16(6, meta.frames.length, true);

  // Frames
  for (let i = 0; i < meta.frames.length; i++) {
    const f = meta.frames[i];
    const off = HEADER + i * PER_FRAME;
    view.setUint16(off, f.sprite_id, true);
    view.setUint16(off + 2, f.uv.u, true);
    view.setUint16(off + 4, f.uv.v, true);
    view.setUint16(off + 6, f.uv.w, true);
    view.setUint16(off + 8, f.uv.h, true);
    view.setInt16(off + 10, f.pivot.x, true);
    view.setInt16(off + 12, f.pivot.y, true);
    view.setUint16(off + 14, f.width, true);
    view.setUint16(off + 16, f.height, true);
  }

  return buf;
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("AtlasManager", () => {
  let mgr: AtlasManager;

  beforeEach(() => {
    mgr = new AtlasManager();
  });

  // 1
  it("registers an atlas and retrieves a frame by sprite_id", () => {
    const frame = makeFrame(100, 1, 0, 0, 64, 64);
    const atlas = makeAtlas(1, [frame]);

    mgr.registerAtlas(atlas);

    const result = mgr.getFrame(100);
    expect(result).toBeDefined();
    expect(result!.sprite_id).toBe(100);
    expect(result!.atlas_id).toBe(1);
    expect(result!.uv).toEqual({ u: 0, v: 0, w: 64, h: 64 });
    expect(result!.width).toBe(64);
    expect(result!.height).toBe(64);
  });

  // 2
  it("returns undefined for an unknown sprite_id", () => {
    const result = mgr.getFrame(9999);
    expect(result).toBeUndefined();
  });

  // 3
  it("tracks frame count correctly", () => {
    expect(mgr.frameCount()).toBe(0);

    const frames = [makeFrame(1, 1), makeFrame(2, 1), makeFrame(3, 1)];
    mgr.registerAtlas(makeAtlas(1, frames));

    expect(mgr.frameCount()).toBe(3);
  });

  // 4
  it("tracks atlas count correctly", () => {
    expect(mgr.atlasCount()).toBe(0);

    mgr.registerAtlas(makeAtlas(1, [makeFrame(1, 1)]));
    expect(mgr.atlasCount()).toBe(1);

    mgr.registerAtlas(makeAtlas(2, [makeFrame(2, 2)]));
    expect(mgr.atlasCount()).toBe(2);
  });

  // 5
  it("handles multiple atlases with frames from each", () => {
    mgr.registerAtlas(makeAtlas(1, [makeFrame(10, 1), makeFrame(11, 1)]));
    mgr.registerAtlas(makeAtlas(2, [makeFrame(20, 2), makeFrame(21, 2)]));

    expect(mgr.atlasCount()).toBe(2);
    expect(mgr.frameCount()).toBe(4);

    expect(mgr.getFrame(10)?.atlas_id).toBe(1);
    expect(mgr.getFrame(21)?.atlas_id).toBe(2);
  });

  // 6
  it("defaults to High resolution tier", () => {
    expect(mgr.currentTier).toBe(ResolutionTier.High);
  });

  // 7
  it("sets resolution tier", () => {
    mgr.setResolutionTier(ResolutionTier.Low);
    expect(mgr.currentTier).toBe(ResolutionTier.Low);

    mgr.setResolutionTier(ResolutionTier.Medium);
    expect(mgr.currentTier).toBe(ResolutionTier.Medium);
  });

  // 8
  it("accepts custom initial resolution tier", () => {
    const custom = new AtlasManager(ResolutionTier.Medium);
    expect(custom.currentTier).toBe(ResolutionTier.Medium);
  });

  // 9
  it("parses binary metadata correctly", () => {
    const original = makeAtlas(5, [
      {
        sprite_id: 42,
        atlas_id: 5,
        uv: { u: 10, v: 20, w: 64, h: 64 },
        pivot: { x: 32, y: 32 },
        width: 64,
        height: 64,
      },
      {
        sprite_id: 43,
        atlas_id: 5,
        uv: { u: 74, v: 20, w: 48, h: 48 },
        pivot: { x: -8, y: -4 },
        width: 48,
        height: 48,
      },
    ], 1024, 1024);

    const buf = buildBinary(original);
    const parsed = mgr.parseMetadataFromBinary(buf);

    expect(parsed.atlas_id).toBe(5);
    expect(parsed.width).toBe(1024);
    expect(parsed.height).toBe(1024);
    expect(parsed.frames).toHaveLength(2);

    const f0 = parsed.frames[0];
    expect(f0.sprite_id).toBe(42);
    expect(f0.uv).toEqual({ u: 10, v: 20, w: 64, h: 64 });
    expect(f0.pivot).toEqual({ x: 32, y: 32 });
    expect(f0.width).toBe(64);
    expect(f0.height).toBe(64);

    const f1 = parsed.frames[1];
    expect(f1.sprite_id).toBe(43);
    expect(f1.pivot).toEqual({ x: -8, y: -4 });
    expect(f1.width).toBe(48);
  });

  // 10
  it("lists atlas IDs in sorted order", () => {
    mgr.registerAtlas(makeAtlas(5, [makeFrame(50, 5)]));
    mgr.registerAtlas(makeAtlas(1, [makeFrame(10, 1)]));
    mgr.registerAtlas(makeAtlas(3, [makeFrame(30, 3)]));

    expect(mgr.listAtlasIds()).toEqual([1, 3, 5]);
  });

  // 11
  it("clear removes all atlases and frames", () => {
    mgr.registerAtlas(makeAtlas(1, [makeFrame(1, 1), makeFrame(2, 1)]));
    mgr.registerAtlas(makeAtlas(2, [makeFrame(3, 2)]));

    expect(mgr.atlasCount()).toBe(2);
    expect(mgr.frameCount()).toBe(3);

    mgr.clear();

    expect(mgr.atlasCount()).toBe(0);
    expect(mgr.frameCount()).toBe(0);
    expect(mgr.getFrame(1)).toBeUndefined();
    expect(mgr.getAtlas(1)).toBeUndefined();
    expect(mgr.listAtlasIds()).toEqual([]);
  });

  // 12
  it("getAtlas returns registered atlas metadata", () => {
    const atlas = makeAtlas(7, [makeFrame(70, 7)], 2048, 2048);
    mgr.registerAtlas(atlas);

    const result = mgr.getAtlas(7);
    expect(result).toBeDefined();
    expect(result!.atlas_id).toBe(7);
    expect(result!.width).toBe(2048);
    expect(result!.height).toBe(2048);
    expect(result!.frames).toHaveLength(1);
  });

  // 13
  it("getAtlas returns undefined for unknown atlas_id", () => {
    expect(mgr.getAtlas(999)).toBeUndefined();
  });

  // 14
  it("parseMetadataFromBinary throws on truncated header", () => {
    const buf = new ArrayBuffer(4); // too small for 8-byte header
    expect(() => mgr.parseMetadataFromBinary(buf)).toThrow(/too small for header/);
  });

  // 15
  it("parseMetadataFromBinary throws on truncated frame data", () => {
    // Valid header claiming 1 frame, but no frame data
    const buf = new ArrayBuffer(8);
    const view = new DataView(buf);
    view.setUint16(0, 1, true);   // atlas_id
    view.setUint16(2, 256, true); // width
    view.setUint16(4, 256, true); // height
    view.setUint16(6, 1, true);   // frame_count = 1

    expect(() => mgr.parseMetadataFromBinary(buf)).toThrow(/too small/);
  });

  // 16
  it("registerAtlas overwrites frames with same sprite_id from different atlas", () => {
    // Register sprite_id=1 in atlas 1
    mgr.registerAtlas(makeAtlas(1, [makeFrame(1, 1, 0, 0, 32, 32)]));
    expect(mgr.getFrame(1)?.atlas_id).toBe(1);

    // Register sprite_id=1 in atlas 2 (overwrite)
    mgr.registerAtlas(makeAtlas(2, [makeFrame(1, 2, 64, 64, 48, 48)]));
    expect(mgr.getFrame(1)?.atlas_id).toBe(2);
    expect(mgr.getFrame(1)?.uv.u).toBe(64);
  });

  // 17
  it("parsed binary can be registered and looked up", () => {
    const original = makeAtlas(10, [
      makeFrame(200, 10, 0, 0, 128, 128),
      makeFrame(201, 10, 128, 0, 64, 64),
    ], 512, 512);

    const buf = buildBinary(original);
    const parsed = mgr.parseMetadataFromBinary(buf);
    mgr.registerAtlas(parsed);

    expect(mgr.atlasCount()).toBe(1);
    expect(mgr.frameCount()).toBe(2);
    expect(mgr.getFrame(200)?.uv.w).toBe(128);
    expect(mgr.getFrame(201)?.uv.u).toBe(128);
  });

  // 18
  it("binary parsing handles negative pivot values", () => {
    const atlas = makeAtlas(1, [{
      sprite_id: 1,
      atlas_id: 1,
      uv: { u: 0, v: 0, w: 32, h: 32 },
      pivot: { x: -16, y: -24 },
      width: 32,
      height: 32,
    }]);

    const buf = buildBinary(atlas);
    const parsed = mgr.parseMetadataFromBinary(buf);

    expect(parsed.frames[0].pivot.x).toBe(-16);
    expect(parsed.frames[0].pivot.y).toBe(-24);
  });
});
