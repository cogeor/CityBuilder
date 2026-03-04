// Barrel export for projection module
export {
  TILE_W,
  TILE_H,
  ELEVATION_HEIGHT,
  type CameraState,
  type ScreenCoord,
  worldToScreen,
  screenToWorld,
  depthKey,
  tileToScreenCenter,
  isInViewport,
  visibleTileRange,
} from "./projection.js";
