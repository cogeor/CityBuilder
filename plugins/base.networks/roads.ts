/** Road type definition */
export interface RoadTypeDef {
  id: number;
  name: string;
  speedLimit: number;   // km/h
  capacity: number;     // vehicles per hour
  lanes: number;
  costPerTile: number;  // cents
  upkeepPerTile: number; // cents per tick
  width: number;        // visual width in sub-tiles
  spriteBase: number;
  variants: number;     // auto-tiling variants (16 for NESW)
}

/** Intersection type */
export enum IntersectionType {
  None = 0,
  T = 1,         // 3-way
  Cross = 2,     // 4-way
  Merge = 3,     // road type transition
}

/** Auto-tile rule */
export interface AutoTileRule {
  mask: number;      // NESW bitmask (0-15)
  spriteOffset: number;
  name: string;
}

export const ROAD_TYPES: RoadTypeDef[] = [
  {
    id: 1,
    name: 'Local Street',
    speedLimit: 30,
    capacity: 200,
    lanes: 2,
    costPerTile: 500,
    upkeepPerTile: 0,
    width: 1,
    spriteBase: 100,
    variants: 16,
  },
  {
    id: 2,
    name: 'Collector Road',
    speedLimit: 50,
    capacity: 500,
    lanes: 2,
    costPerTile: 1500,
    upkeepPerTile: 1,
    width: 1,
    spriteBase: 120,
    variants: 16,
  },
  {
    id: 3,
    name: 'Arterial Road',
    speedLimit: 60,
    capacity: 1000,
    lanes: 4,
    costPerTile: 5000,
    upkeepPerTile: 3,
    width: 2,
    spriteBase: 140,
    variants: 16,
  },
  {
    id: 4,
    name: 'Highway',
    speedLimit: 100,
    capacity: 2000,
    lanes: 6,
    costPerTile: 15000,
    upkeepPerTile: 8,
    width: 3,
    spriteBase: 160,
    variants: 16,
  },
];

/** Auto-tile rules for 16 NESW variants */
export const AUTO_TILE_RULES: AutoTileRule[] = [
  { mask: 0b0000, spriteOffset: 0, name: 'isolated' },
  { mask: 0b0001, spriteOffset: 1, name: 'dead_end_n' },
  { mask: 0b0010, spriteOffset: 2, name: 'dead_end_e' },
  { mask: 0b0011, spriteOffset: 3, name: 'curve_ne' },
  { mask: 0b0100, spriteOffset: 4, name: 'dead_end_s' },
  { mask: 0b0101, spriteOffset: 5, name: 'straight_ns' },
  { mask: 0b0110, spriteOffset: 6, name: 'curve_se' },
  { mask: 0b0111, spriteOffset: 7, name: 't_nse' },
  { mask: 0b1000, spriteOffset: 8, name: 'dead_end_w' },
  { mask: 0b1001, spriteOffset: 9, name: 'curve_nw' },
  { mask: 0b1010, spriteOffset: 10, name: 'straight_ew' },
  { mask: 0b1011, spriteOffset: 11, name: 't_new' },
  { mask: 0b1100, spriteOffset: 12, name: 'curve_sw' },
  { mask: 0b1101, spriteOffset: 13, name: 't_nsw' },
  { mask: 0b1110, spriteOffset: 14, name: 't_sew' },
  { mask: 0b1111, spriteOffset: 15, name: 'cross' },
];

/** Get road type by id */
export function getRoadType(id: number): RoadTypeDef | undefined {
  return ROAD_TYPES.find(r => r.id === id);
}

/** Get auto-tile sprite offset for a given connection mask */
export function getAutoTileOffset(mask: number): number {
  const rule = AUTO_TILE_RULES.find(r => r.mask === (mask & 0xF));
  return rule?.spriteOffset ?? 0;
}

/** Compute sprite id for a road at given tile */
export function computeRoadSpriteId(roadTypeId: number, connectionMask: number): number {
  const road = getRoadType(roadTypeId);
  if (!road) return 0;
  return road.spriteBase + getAutoTileOffset(connectionMask);
}

/** Detect intersection type from connection mask */
export function detectIntersectionType(mask: number): IntersectionType {
  const connections = popCount(mask & 0xF);
  if (connections >= 4) return IntersectionType.Cross;
  if (connections === 3) return IntersectionType.T;
  return IntersectionType.None;
}

/** Count set bits */
function popCount(n: number): number {
  let count = 0;
  while (n) { count += n & 1; n >>= 1; }
  return count;
}

/** Compute travel time for a road segment in ticks */
export function computeTravelTime(roadTypeId: number, distanceTiles: number): number {
  const road = getRoadType(roadTypeId);
  if (!road) return Infinity;
  // Convert km/h to tiles/tick: speed_kph / (16m/tile) * (1000m/km) / (3600s/h) / (20 ticks/s)
  const tilesPerTick = (road.speedLimit * 1000) / (16 * 3600 * 20);
  return Math.ceil(distanceTiles / tilesPerTick);
}

/** Validate road configuration */
export function validateRoadConfig(): string[] {
  const errors: string[] = [];
  const ids = new Set<number>();
  for (const r of ROAD_TYPES) {
    if (ids.has(r.id)) errors.push(`Duplicate road id: ${r.id}`);
    ids.add(r.id);
    if (r.speedLimit <= 0) errors.push(`Invalid speed for ${r.name}`);
    if (r.capacity <= 0) errors.push(`Invalid capacity for ${r.name}`);
    if (r.costPerTile < 0) errors.push(`Negative cost for ${r.name}`);
  }
  return errors;
}
