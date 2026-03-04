// @townbuilder/renderer — WebGL2/WebGPU renderer

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
} from "./projection/index.js";

export {
  type UVRect,
  type SpritePivot,
  type SpriteFrame,
  type AtlasMetadata,
  ResolutionTier,
  AtlasManager,
} from "./atlas/index.js";

export {
  type IRenderBackend,
  type RenderStats,
  type MockGLCallLog,
  VERTEX_SHADER_SRC,
  FRAGMENT_SHADER_SRC,
  WebGL2Backend,
  MockGL,
  MockCanvas,
} from "./backends/index.js";
