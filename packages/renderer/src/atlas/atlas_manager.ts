// atlas_manager.ts — Sprite atlas system: loading, UV lookup, frame table

// ─── Interfaces ───────────────────────────────────────────────────────────────

/** UV rectangle in normalized or texel coordinates. */
export interface UVRect {
  u: number;
  v: number;
  w: number;
  h: number;
}

/** Sprite pivot point (offset from top-left in pixels). */
export interface SpritePivot {
  x: number;
  y: number;
}

/** A single sprite frame within an atlas. */
export interface SpriteFrame {
  sprite_id: number;
  atlas_id: number;
  uv: UVRect;
  pivot: SpritePivot;
  width: number;
  height: number;
}

/** Metadata for an entire sprite atlas texture. */
export interface AtlasMetadata {
  atlas_id: number;
  width: number;
  height: number;
  frames: SpriteFrame[];
}

// ─── Enum ─────────────────────────────────────────────────────────────────────

/** Resolution tier for atlas selection. */
export enum ResolutionTier {
  High = "high",
  Medium = "medium",
  Low = "low",
}

// ─── Binary format constants ──────────────────────────────────────────────────

/** Header: atlas_id(u16) + width(u16) + height(u16) + frame_count(u16) = 8 bytes */
const HEADER_BYTES = 8;

/** Per-frame: sprite_id(u16) + u(u16) + v(u16) + w(u16) + h(u16) + pivot_x(i16) + pivot_y(i16) + orig_w(u16) + orig_h(u16) = 18 bytes */
const FRAME_BYTES = 18;

// ─── AtlasManager ─────────────────────────────────────────────────────────────

/**
 * Manages sprite atlas metadata: registration, frame lookup by sprite_id,
 * resolution tier switching, and binary metadata parsing.
 */
export class AtlasManager {
  /** All registered atlases keyed by atlas_id. */
  readonly atlases: Map<number, AtlasMetadata> = new Map();

  /** Fast lookup from sprite_id to its SpriteFrame. */
  readonly frameIndex: Map<number, SpriteFrame> = new Map();

  /** Currently active resolution tier. */
  currentTier: ResolutionTier;

  constructor(tier: ResolutionTier = ResolutionTier.High) {
    this.currentTier = tier;
  }

  /**
   * Register an atlas and index all its frames by sprite_id.
   * Overwrites any previously registered frame with the same sprite_id.
   */
  registerAtlas(metadata: AtlasMetadata): void {
    this.atlases.set(metadata.atlas_id, metadata);
    for (const frame of metadata.frames) {
      this.frameIndex.set(frame.sprite_id, frame);
    }
  }

  /** Look up a sprite frame by its sprite_id. */
  getFrame(sprite_id: number): SpriteFrame | undefined {
    return this.frameIndex.get(sprite_id);
  }

  /** Look up atlas metadata by atlas_id. */
  getAtlas(atlas_id: number): AtlasMetadata | undefined {
    return this.atlases.get(atlas_id);
  }

  /** Return a sorted list of all registered atlas IDs. */
  listAtlasIds(): number[] {
    return Array.from(this.atlases.keys()).sort((a, b) => a - b);
  }

  /** Switch the active resolution tier. */
  setResolutionTier(tier: ResolutionTier): void {
    this.currentTier = tier;
  }

  /** Total number of indexed sprite frames. */
  frameCount(): number {
    return this.frameIndex.size;
  }

  /** Total number of registered atlases. */
  atlasCount(): number {
    return this.atlases.size;
  }

  /**
   * Parse binary atlas metadata (.atlasbin format).
   *
   * Binary layout (little-endian):
   *   Header (8 bytes):
   *     atlas_id   : u16
   *     width      : u16
   *     height     : u16
   *     frame_count: u16
   *
   *   Per frame (18 bytes each):
   *     sprite_id : u16
   *     u         : u16  (texels)
   *     v         : u16  (texels)
   *     w         : u16  (texels)
   *     h         : u16  (texels)
   *     pivot_x   : i16
   *     pivot_y   : i16
   *     orig_w    : u16
   *     orig_h    : u16
   */
  parseMetadataFromBinary(buffer: ArrayBuffer): AtlasMetadata {
    const view = new DataView(buffer);

    if (buffer.byteLength < HEADER_BYTES) {
      throw new Error(
        `Atlas binary too small for header: ${buffer.byteLength} < ${HEADER_BYTES}`,
      );
    }

    // Read header (little-endian)
    const atlas_id = view.getUint16(0, true);
    const width = view.getUint16(2, true);
    const height = view.getUint16(4, true);
    const frame_count = view.getUint16(6, true);

    const expectedSize = HEADER_BYTES + frame_count * FRAME_BYTES;
    if (buffer.byteLength < expectedSize) {
      throw new Error(
        `Atlas binary too small: ${buffer.byteLength} < ${expectedSize} (${frame_count} frames)`,
      );
    }

    const frames: SpriteFrame[] = [];

    for (let i = 0; i < frame_count; i++) {
      const offset = HEADER_BYTES + i * FRAME_BYTES;

      const sprite_id = view.getUint16(offset, true);
      const u = view.getUint16(offset + 2, true);
      const v = view.getUint16(offset + 4, true);
      const w = view.getUint16(offset + 6, true);
      const h = view.getUint16(offset + 8, true);
      const pivot_x = view.getInt16(offset + 10, true);
      const pivot_y = view.getInt16(offset + 12, true);
      const orig_w = view.getUint16(offset + 14, true);
      const orig_h = view.getUint16(offset + 16, true);

      frames.push({
        sprite_id,
        atlas_id,
        uv: { u, v, w, h },
        pivot: { x: pivot_x, y: pivot_y },
        width: orig_w,
        height: orig_h,
      });
    }

    return { atlas_id, width, height, frames };
  }

  /** Remove all registered atlases and frame index entries. */
  clear(): void {
    this.atlases.clear();
    this.frameIndex.clear();
  }
}
