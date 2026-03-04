import { describe, it, expect } from "vitest";
import {
  TILE_W,
  TILE_H,
  ELEVATION_HEIGHT,
  worldToScreen,
  screenToWorld,
  depthKey,
  tileToScreenCenter,
  isInViewport,
  visibleTileRange,
  type CameraState,
} from "../projection.js";

/** Default camera centered at origin with 1x zoom. */
const defaultCamera: CameraState = {
  x: 0,
  y: 0,
  zoom: 1.0,
  viewportWidth: 1920,
  viewportHeight: 1080,
};

describe("projection", () => {
  // ─── Constants ────────────────────────────────────────────────────────

  it("constants have expected values", () => {
    expect(TILE_W).toBe(128);
    expect(TILE_H).toBe(64);
    expect(ELEVATION_HEIGHT).toBe(16);
  });

  // ─── worldToScreen ────────────────────────────────────────────────────

  it("worldToScreen maps origin tile to viewport center", () => {
    const result = worldToScreen(0, 0, 0, defaultCamera);
    expect(result.x).toBeCloseTo(1920 / 2);
    expect(result.y).toBeCloseTo(1080 / 2);
  });

  it("worldToScreen positive X tile moves right and down", () => {
    const origin = worldToScreen(0, 0, 0, defaultCamera);
    const right = worldToScreen(1, 0, 0, defaultCamera);

    // In isometric, moving +X goes right and down
    expect(right.x).toBeGreaterThan(origin.x);
    expect(right.y).toBeGreaterThan(origin.y);
  });

  it("worldToScreen positive Y tile moves left and down", () => {
    const origin = worldToScreen(0, 0, 0, defaultCamera);
    const down = worldToScreen(0, 1, 0, defaultCamera);

    // In isometric, moving +Y goes left and down
    expect(down.x).toBeLessThan(origin.x);
    expect(down.y).toBeGreaterThan(origin.y);
  });

  it("worldToScreen elevation offsets Y upward", () => {
    const flat = worldToScreen(5, 5, 0, defaultCamera);
    const elevated = worldToScreen(5, 5, 2, defaultCamera);

    expect(elevated.x).toBeCloseTo(flat.x);
    expect(elevated.y).toBeLessThan(flat.y);
    expect(flat.y - elevated.y).toBeCloseTo(2 * ELEVATION_HEIGHT * defaultCamera.zoom);
  });

  it("worldToScreen applies camera offset", () => {
    const cam: CameraState = { ...defaultCamera, x: 10, y: 10 };
    const result = worldToScreen(10, 10, 0, cam);

    // Tile at camera position should map to viewport center
    expect(result.x).toBeCloseTo(cam.viewportWidth / 2);
    expect(result.y).toBeCloseTo(cam.viewportHeight / 2);
  });

  it("worldToScreen zoom affects screen coordinates", () => {
    const zoom1 = worldToScreen(3, 1, 0, defaultCamera);
    const cam2x: CameraState = { ...defaultCamera, zoom: 2.0 };
    const zoom2 = worldToScreen(3, 1, 0, cam2x);

    // At 2x zoom, the offset from center should be doubled
    const cx = defaultCamera.viewportWidth / 2;
    const cy = defaultCamera.viewportHeight / 2;

    const offset1x = zoom1.x - cx;
    const offset1y = zoom1.y - cy;
    const offset2x = zoom2.x - cx;
    const offset2y = zoom2.y - cy;

    expect(offset2x).toBeCloseTo(offset1x * 2);
    expect(offset2y).toBeCloseTo(offset1y * 2);
  });

  // ─── screenToWorld ────────────────────────────────────────────────────

  it("screenToWorld inverts worldToScreen at elevation 0", () => {
    const tileX = 7.5;
    const tileY = 3.2;

    const screen = worldToScreen(tileX, tileY, 0, defaultCamera);
    const world = screenToWorld(screen.x, screen.y, defaultCamera);

    expect(world.tileX).toBeCloseTo(tileX);
    expect(world.tileY).toBeCloseTo(tileY);
  });

  it("screenToWorld round-trip consistency with integer tiles", () => {
    for (let tx = -5; tx <= 5; tx++) {
      for (let ty = -5; ty <= 5; ty++) {
        const screen = worldToScreen(tx, ty, 0, defaultCamera);
        const world = screenToWorld(screen.x, screen.y, defaultCamera);

        expect(world.tileX).toBeCloseTo(tx, 5);
        expect(world.tileY).toBeCloseTo(ty, 5);
      }
    }
  });

  it("screenToWorld round-trip with zoomed camera", () => {
    const cam: CameraState = { ...defaultCamera, zoom: 0.5, x: 20, y: -10 };
    const tileX = 25;
    const tileY = -5;

    const screen = worldToScreen(tileX, tileY, 0, cam);
    const world = screenToWorld(screen.x, screen.y, cam);

    expect(world.tileX).toBeCloseTo(tileX);
    expect(world.tileY).toBeCloseTo(tileY);
  });

  it("screenToWorld viewport center maps to camera position", () => {
    const cam: CameraState = { ...defaultCamera, x: 15, y: 25 };
    const world = screenToWorld(cam.viewportWidth / 2, cam.viewportHeight / 2, cam);

    expect(world.tileX).toBeCloseTo(cam.x);
    expect(world.tileY).toBeCloseTo(cam.y);
  });

  // ─── depthKey ─────────────────────────────────────────────────────────

  it("depthKey ordering: closer tiles have lower key", () => {
    // In isometric, tile (0,0) is "further back" than tile (5,5)
    const keyFar = depthKey(0, 0, 0, 0, 0);
    const keyClose = depthKey(5, 5, 0, 0, 0);

    expect(keyClose).toBeGreaterThan(keyFar);
  });

  it("depthKey higher layer sorts after lower layer", () => {
    const layer0 = depthKey(10, 10, 0, 0, 0);
    const layer1 = depthKey(0, 0, 0, 1, 0);

    expect(layer1).toBeGreaterThan(layer0);
  });

  it("depthKey higher z sorts after lower z at same iso depth", () => {
    const z0 = depthKey(3, 3, 0, 0, 0);
    const z1 = depthKey(3, 3, 1, 0, 0);

    expect(z1).toBeGreaterThan(z0);
  });

  it("depthKey localY tiebreaker works within same tile", () => {
    const y0 = depthKey(3, 3, 0, 0, 0);
    const y5 = depthKey(3, 3, 0, 0, 5);

    expect(y5).toBeGreaterThan(y0);
  });

  // ─── tileToScreenCenter ───────────────────────────────────────────────

  it("tileToScreenCenter matches worldToScreen with elevation 0", () => {
    const direct = worldToScreen(4, 7, 0, defaultCamera);
    const convenience = tileToScreenCenter(4, 7, defaultCamera);

    expect(convenience.x).toBeCloseTo(direct.x);
    expect(convenience.y).toBeCloseTo(direct.y);
  });

  // ─── isInViewport ─────────────────────────────────────────────────────

  it("isInViewport returns true for point inside viewport", () => {
    expect(isInViewport(500, 400, defaultCamera, 0)).toBe(true);
  });

  it("isInViewport returns false for point outside viewport", () => {
    expect(isInViewport(-100, 500, defaultCamera, 0)).toBe(false);
    expect(isInViewport(500, -100, defaultCamera, 0)).toBe(false);
    expect(isInViewport(2000, 500, defaultCamera, 0)).toBe(false);
    expect(isInViewport(500, 1200, defaultCamera, 0)).toBe(false);
  });

  it("isInViewport margin expands bounds", () => {
    // Point at -50 is outside with 0 margin, inside with 100 margin
    expect(isInViewport(-50, 500, defaultCamera, 0)).toBe(false);
    expect(isInViewport(-50, 500, defaultCamera, 100)).toBe(true);
  });

  it("isInViewport edge cases at viewport boundary", () => {
    // Exactly at boundary with 0 margin
    expect(isInViewport(0, 0, defaultCamera, 0)).toBe(true);
    expect(isInViewport(1920, 1080, defaultCamera, 0)).toBe(true);
    expect(isInViewport(1921, 0, defaultCamera, 0)).toBe(false);
  });

  // ─── visibleTileRange ─────────────────────────────────────────────────

  it("visibleTileRange covers viewport area", () => {
    const range = visibleTileRange(defaultCamera);

    expect(range.maxX).toBeGreaterThan(range.minX);
    expect(range.maxY).toBeGreaterThan(range.minY);
  });

  it("visibleTileRange includes camera center tile", () => {
    const cam: CameraState = { ...defaultCamera, x: 50, y: 50 };
    const range = visibleTileRange(cam);

    expect(range.minX).toBeLessThanOrEqual(50);
    expect(range.maxX).toBeGreaterThanOrEqual(50);
    expect(range.minY).toBeLessThanOrEqual(50);
    expect(range.maxY).toBeGreaterThanOrEqual(50);
  });

  it("visibleTileRange shrinks with higher zoom", () => {
    const cam1x: CameraState = { ...defaultCamera, zoom: 1.0 };
    const cam2x: CameraState = { ...defaultCamera, zoom: 2.0 };

    const range1 = visibleTileRange(cam1x);
    const range2 = visibleTileRange(cam2x);

    const area1 = (range1.maxX - range1.minX) * (range1.maxY - range1.minY);
    const area2 = (range2.maxX - range2.minX) * (range2.maxY - range2.minY);

    expect(area2).toBeLessThan(area1);
  });

  it("visibleTileRange expands with lower zoom", () => {
    const cam1x: CameraState = { ...defaultCamera, zoom: 1.0 };
    const camHalf: CameraState = { ...defaultCamera, zoom: 0.5 };

    const range1 = visibleTileRange(cam1x);
    const rangeHalf = visibleTileRange(camHalf);

    const area1 = (range1.maxX - range1.minX) * (range1.maxY - range1.minY);
    const areaHalf = (rangeHalf.maxX - rangeHalf.minX) * (rangeHalf.maxY - rangeHalf.minY);

    expect(areaHalf).toBeGreaterThan(area1);
  });
});
