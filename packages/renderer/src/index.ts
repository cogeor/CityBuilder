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

export {
  INSTANCE_BYTE_SIZE,
  RenderPass,
  type RenderInstance,
  DEFAULT_INSTANCE,
  packInstance,
  unpackInstance,
  RenderInstanceBuilder,
} from "./types/index.js";

export {
  CHUNK_SIZE,
  MAX_CACHED_CHUNKS,
  MAX_REBUILD_PER_FRAME,
  type ChunkKey,
  type ChunkData,
  type ChunkCacheStats,
  ChunkCache,
} from "./chunks/index.js";

export {
  TerrainType,
  ZoneType,
  RoadType,
  type TileRenderData,
  type SpriteMapping,
  type SpriteResolver,
  computeRoadMask,
  computeTerrainEdgeMask,
  ChunkBuilder,
} from "./chunks/index.js";

export {
  AutomataType,
  type DynamicEntity,
  type SelectionHighlight,
  type DynamicRenderStats,
  interpolatePosition,
  computeAnimFrame,
  computeSpriteOffset,
  DynamicRenderer,
} from "./dynamic/index.js";

export {
  OverlayType,
  ZoneDisplayType,
  StatusIconType,
  type OverlayColor,
  type GradientStop,
  type OverlayConfig,
  type OverlayRenderStats,
  OVERLAY_SPRITE_ID,
  STATUS_ICON_SPRITES,
  GRADIENT_GREEN_RED,
  GRADIENT_BLUE_BROWN,
  GRADIENT_COOL_HOT,
  ZONE_COLORS,
  DEFAULT_OVERLAY_CONFIGS,
  sampleGradient,
  normalizeHeatmapValue,
  OverlayRenderer,
} from "./overlays/index.js";
