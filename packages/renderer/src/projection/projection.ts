// @townbuilder/renderer — Isometric projection and coordinate transforms

// ─── Constants ──────────────────────────────────────────────────────────────

/** Render tile width in pixels. */
export const TILE_W = 128;

/** Render tile height in pixels (isometric diamond half-height). */
export const TILE_H = 64;

/** Pixels per elevation unit. */
export const ELEVATION_HEIGHT = 16;

// ─── Interfaces ─────────────────────────────────────────────────────────────

/** Camera state for viewport positioning and zoom. */
export interface CameraState {
  /** World position in tiles (can be fractional). */
  readonly x: number;
  /** World position in tiles (can be fractional). */
  readonly y: number;
  /** Zoom level. 1.0 = default, 0.5 = zoomed out, 2.0 = zoomed in. */
  readonly zoom: number;
  /** Viewport width in pixels. */
  readonly viewportWidth: number;
  /** Viewport height in pixels. */
  readonly viewportHeight: number;
}

/** A screen-space coordinate in pixels. */
export interface ScreenCoord {
  readonly x: number;
  readonly y: number;
}

// ─── Projection Functions ───────────────────────────────────────────────────

/**
 * Convert world tile coordinates to screen pixel coordinates.
 *
 * Applies the standard isometric transform:
 *   screenX = (tileX - tileY) * (TILE_W / 2) * zoom
 *   screenY = (tileX + tileY) * (TILE_H / 2) * zoom - elevation * ELEVATION_HEIGHT * zoom
 *
 * Then offsets by camera position and centers in viewport.
 */
export function worldToScreen(
  tileX: number,
  tileY: number,
  elevation: number,
  camera: CameraState,
): ScreenCoord {
  const halfW = TILE_W / 2;
  const halfH = TILE_H / 2;
  const z = camera.zoom;

  // Isometric projection relative to camera
  const relX = tileX - camera.x;
  const relY = tileY - camera.y;

  const sx = relX - relY;
  const sy = relX + relY;

  const screenX = sx * halfW * z + camera.viewportWidth / 2;
  const screenY = sy * halfH * z - elevation * ELEVATION_HEIGHT * z + camera.viewportHeight / 2;

  return { x: screenX, y: screenY };
}

/**
 * Convert screen pixel coordinates back to world tile coordinates.
 * Inverse of worldToScreen without elevation (assumes elevation = 0).
 * Returns fractional tile coordinates.
 */
export function screenToWorld(
  screenX: number,
  screenY: number,
  camera: CameraState,
): { tileX: number; tileY: number } {
  const halfW = TILE_W / 2;
  const halfH = TILE_H / 2;
  const z = camera.zoom;

  // Remove viewport centering
  const cx = screenX - camera.viewportWidth / 2;
  const cy = screenY - camera.viewportHeight / 2;

  // Undo zoom and isometric scaling
  const sx = cx / (halfW * z);
  const sy = cy / (halfH * z);

  // Invert the isometric transform:
  //   sx = relX - relY
  //   sy = relX + relY
  // => relX = (sx + sy) / 2
  //    relY = (sy - sx) / 2
  const relX = (sx + sy) / 2;
  const relY = (sy - sx) / 2;

  return {
    tileX: relX + camera.x,
    tileY: relY + camera.y,
  };
}

/**
 * Compute a depth sort key for rendering order.
 *
 * Encodes layer, iso depth (tileX + tileY), z (elevation), and localY
 * into a single number safe within JavaScript's 2^53 integer range.
 *
 * Order: layer * 2^32 + (tileX + tileY) * 2^16 + z * 2^8 + localY
 */
export function depthKey(
  tileX: number,
  tileY: number,
  z: number,
  layer: number,
  localY: number,
): number {
  // Use multiplication instead of bit-shift for values > 2^31
  return (
    layer * 4294967296 + // 2^32
    (tileX + tileY) * 65536 + // 2^16
    z * 256 + // 2^8
    localY
  );
}

/**
 * Convenience: project tile center to screen at elevation 0.
 */
export function tileToScreenCenter(
  tileX: number,
  tileY: number,
  camera: CameraState,
): ScreenCoord {
  return worldToScreen(tileX, tileY, 0, camera);
}

/**
 * Returns true if the given screen coordinate is within the viewport
 * expanded by the given margin (in pixels).
 */
export function isInViewport(
  screenX: number,
  screenY: number,
  camera: CameraState,
  margin: number,
): boolean {
  return (
    screenX >= -margin &&
    screenX <= camera.viewportWidth + margin &&
    screenY >= -margin &&
    screenY <= camera.viewportHeight + margin
  );
}

/**
 * Returns the range of tile coordinates visible in the current viewport.
 * Used for culling tiles outside the camera view.
 *
 * Computes by projecting the four viewport corners back to world space
 * and finding the bounding box in tile coordinates.
 */
export function visibleTileRange(camera: CameraState): {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
} {
  // Project all four corners of the viewport to world space
  const topLeft = screenToWorld(0, 0, camera);
  const topRight = screenToWorld(camera.viewportWidth, 0, camera);
  const bottomLeft = screenToWorld(0, camera.viewportHeight, camera);
  const bottomRight = screenToWorld(camera.viewportWidth, camera.viewportHeight, camera);

  const allX = [topLeft.tileX, topRight.tileX, bottomLeft.tileX, bottomRight.tileX];
  const allY = [topLeft.tileY, topRight.tileY, bottomLeft.tileY, bottomRight.tileY];

  return {
    minX: Math.floor(Math.min(...allX)),
    maxX: Math.ceil(Math.max(...allX)),
    minY: Math.floor(Math.min(...allY)),
    maxY: Math.ceil(Math.max(...allY)),
  };
}
