// @townbuilder/renderer — Render instance types and depth-sort builder

// ─── Constants ──────────────────────────────────────────────────────────────

/** Byte size of one packed RenderInstance in a GPU buffer. */
export const INSTANCE_BYTE_SIZE = 48;

// ─── Enums ──────────────────────────────────────────────────────────────────

/** Render pass order for layered drawing. */
export enum RenderPass {
  Terrain = 0,
  Networks = 1,
  Buildings = 2,
  Props = 3,
  Automata = 4,
  Overlays = 5,
  UI = 6,
}

// ─── Interfaces ─────────────────────────────────────────────────────────────

/**
 * A single renderable sprite instance, matching the Rust RenderInstance struct.
 * Packs into 48 bytes for GPU upload via packInstance / unpackInstance.
 *
 * Binary layout (little-endian, 48 bytes total):
 *   [0..4]   float32  sprite_id
 *   [4..8]   float32  atlas_id
 *   [8..12]  float32  screen_x
 *   [12..16] float32  screen_y
 *   [16..20] float32  z_order
 *   [20..22] uint16   palette_id
 *   [22..24] uint16   mask_flags
 *   [24..26] uint16   anim_frame
 *   [26..28] uint16   render_flags
 *   [28..32] float32  scale_x
 *   [32..36] float32  scale_y
 *   [36..40] float32  rotation
 *   [40..44] 4xUint8  tint_r, tint_g, tint_b, tint_a
 *   [44..48] padding  (zeroed)
 */
export interface RenderInstance {
  /** Sprite identifier in the atlas. */
  sprite_id: number;
  /** Atlas texture slot. */
  atlas_id: number;
  /** Screen-space X position in pixels. */
  screen_x: number;
  /** Screen-space Y position in pixels. */
  screen_y: number;
  /** Depth sort key (higher = drawn later / on top). */
  z_order: number;
  /** Palette swap index (0 = default). */
  palette_id: number;
  /** Bit mask for visual effects (outline, highlight, etc.). */
  mask_flags: number;
  /** Current animation frame index. */
  anim_frame: number;
  /** Render flags (visibility, flip, etc.). Bit 0 = visible. */
  render_flags: number;
  /** Horizontal scale factor. */
  scale_x: number;
  /** Vertical scale factor. */
  scale_y: number;
  /** Rotation in radians. */
  rotation: number;
  /** Tint color red channel (0-255). */
  tint_r: number;
  /** Tint color green channel (0-255). */
  tint_g: number;
  /** Tint color blue channel (0-255). */
  tint_b: number;
  /** Tint color alpha channel (0-255). */
  tint_a: number;
}

// ─── Defaults ───────────────────────────────────────────────────────────────

/** Default RenderInstance: visible, white tint, 1x scale, no rotation. */
export const DEFAULT_INSTANCE: Readonly<RenderInstance> = {
  sprite_id: 0,
  atlas_id: 0,
  screen_x: 0,
  screen_y: 0,
  z_order: 0,
  palette_id: 0,
  mask_flags: 0,
  anim_frame: 0,
  render_flags: 1, // bit 0 = visible
  scale_x: 1.0,
  scale_y: 1.0,
  rotation: 0,
  tint_r: 255,
  tint_g: 255,
  tint_b: 255,
  tint_a: 255,
};

// ─── Pack / Unpack ──────────────────────────────────────────────────────────

/**
 * Pack a RenderInstance into a DataView at the given byte offset (48 bytes).
 *
 * Layout (little-endian):
 *   Offset  0: float32 sprite_id
 *   Offset  4: float32 atlas_id
 *   Offset  8: float32 screen_x
 *   Offset 12: float32 screen_y
 *   Offset 16: float32 z_order
 *   Offset 20: uint16 palette_id + uint16 mask_flags
 *   Offset 24: uint16 anim_frame + uint16 render_flags
 *   Offset 28: float32 scale_x
 *   Offset 32: float32 scale_y
 *   Offset 36: float32 rotation
 *   Offset 40: uint8 tint_r, uint8 tint_g, uint8 tint_b, uint8 tint_a
 *   Offset 44: padding (4 bytes zeroed)
 */
export function packInstance(
  instance: RenderInstance,
  view: DataView,
  offset: number,
): void {
  const LE = true;
  view.setFloat32(offset + 0, instance.sprite_id, LE);
  view.setFloat32(offset + 4, instance.atlas_id, LE);
  view.setFloat32(offset + 8, instance.screen_x, LE);
  view.setFloat32(offset + 12, instance.screen_y, LE);
  view.setFloat32(offset + 16, instance.z_order, LE);

  // Pack palette_id (low u16) + mask_flags (high u16)
  view.setUint16(offset + 20, instance.palette_id & 0xffff, LE);
  view.setUint16(offset + 22, instance.mask_flags & 0xffff, LE);

  // Pack anim_frame (low u16) + render_flags (high u16)
  view.setUint16(offset + 24, instance.anim_frame & 0xffff, LE);
  view.setUint16(offset + 26, instance.render_flags & 0xffff, LE);

  view.setFloat32(offset + 28, instance.scale_x, LE);
  view.setFloat32(offset + 32, instance.scale_y, LE);
  view.setFloat32(offset + 36, instance.rotation, LE);

  // Pack tint as 4 consecutive bytes
  view.setUint8(offset + 40, instance.tint_r & 0xff);
  view.setUint8(offset + 41, instance.tint_g & 0xff);
  view.setUint8(offset + 42, instance.tint_b & 0xff);
  view.setUint8(offset + 43, instance.tint_a & 0xff);

  // Padding
  view.setUint32(offset + 44, 0, LE);
}

/**
 * Unpack a RenderInstance from a DataView at the given byte offset (48 bytes).
 */
export function unpackInstance(
  view: DataView,
  offset: number,
): RenderInstance {
  const LE = true;
  return {
    sprite_id: view.getFloat32(offset + 0, LE),
    atlas_id: view.getFloat32(offset + 4, LE),
    screen_x: view.getFloat32(offset + 8, LE),
    screen_y: view.getFloat32(offset + 12, LE),
    z_order: view.getFloat32(offset + 16, LE),
    palette_id: view.getUint16(offset + 20, LE),
    mask_flags: view.getUint16(offset + 22, LE),
    anim_frame: view.getUint16(offset + 24, LE),
    render_flags: view.getUint16(offset + 26, LE),
    scale_x: view.getFloat32(offset + 28, LE),
    scale_y: view.getFloat32(offset + 32, LE),
    rotation: view.getFloat32(offset + 36, LE),
    tint_r: view.getUint8(offset + 40),
    tint_g: view.getUint8(offset + 41),
    tint_b: view.getUint8(offset + 42),
    tint_a: view.getUint8(offset + 43),
  };
}

// ─── Builder ────────────────────────────────────────────────────────────────

/**
 * Accumulates RenderInstances and sorts them by z_order for depth-correct
 * rendering. Provides GPU-ready typed array conversion.
 */
export class RenderInstanceBuilder {
  /** Internal instance buffer. */
  instances: RenderInstance[] = [];

  /** Add a fully-specified instance. */
  add(instance: RenderInstance): void {
    this.instances.push(instance);
  }

  /**
   * Add an instance with default values, overriding only the essential fields.
   * The instance is visible (render_flags bit 0 set).
   */
  addDefault(
    sprite_id: number,
    screen_x: number,
    screen_y: number,
    z_order: number,
  ): void {
    this.instances.push({
      ...DEFAULT_INSTANCE,
      sprite_id,
      screen_x,
      screen_y,
      z_order,
    });
  }

  /** Sort all instances by z_order (ascending = back-to-front). */
  sort(): void {
    this.instances.sort((a, b) => a.z_order - b.z_order);
  }

  /** Remove all instances. */
  clear(): void {
    this.instances.length = 0;
  }

  /** Return the number of instances. */
  count(): number {
    return this.instances.length;
  }

  /**
   * Pack all instances into a GPU-ready Float32Array.
   * The underlying buffer is sized to `count * INSTANCE_BYTE_SIZE` bytes.
   * Data is written via DataView for precise field layout.
   */
  toTypedArray(): Float32Array {
    const byteLength = this.instances.length * INSTANCE_BYTE_SIZE;
    const buffer = new ArrayBuffer(byteLength);
    const view = new DataView(buffer);

    for (let i = 0; i < this.instances.length; i++) {
      packInstance(this.instances[i], view, i * INSTANCE_BYTE_SIZE);
    }

    return new Float32Array(buffer);
  }

  /**
   * Reconstruct an array of RenderInstance from a packed Float32Array.
   * @param buffer — the packed GPU buffer
   * @param count — how many instances to read
   */
  static fromTypedArray(
    buffer: Float32Array,
    count: number,
  ): RenderInstance[] {
    const view = new DataView(
      buffer.buffer,
      buffer.byteOffset,
      buffer.byteLength,
    );
    const result: RenderInstance[] = [];

    for (let i = 0; i < count; i++) {
      result.push(unpackInstance(view, i * INSTANCE_BYTE_SIZE));
    }

    return result;
  }
}
