// @townbuilder/base.networks — Barrel export
export {
  type RoadTypeDef,
  IntersectionType,
  type AutoTileRule,
  ROAD_TYPES,
  AUTO_TILE_RULES,
  getRoadType,
  getAutoTileOffset,
  computeRoadSpriteId,
  detectIntersectionType,
  computeTravelTime,
  validateRoadConfig,
} from "./roads.js";

export {
  TransitMode,
  type TransitVehicle,
  TRANSIT_VEHICLES,
  getVehiclesByMode,
  estimateLineCapacity,
  estimateLineCost,
} from "./multimodal.js";
