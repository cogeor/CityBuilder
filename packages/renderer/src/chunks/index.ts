// Barrel export for chunks module
export {
  CHUNK_SIZE,
  MAX_CACHED_CHUNKS,
  MAX_REBUILD_PER_FRAME,
  type ChunkKey,
  type ChunkData,
  type ChunkCacheStats,
  ChunkCache,
} from './chunk_cache.js';

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
} from './chunk_builder.js';
