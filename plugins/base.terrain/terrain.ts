/** Terrain type definition */
export interface TerrainTypeDef {
  id: number;
  name: string;
  buildable: boolean;
  movementCost: number;  // 1.0 = normal, higher = slower, 0 = impassable
  color: { r: number; g: number; b: number };
  spriteBase: number;
  variants: number;  // number of visual variants
}

/** Elevation rule */
export interface ElevationRule {
  minElevation: number;
  maxElevation: number;
  defaultTerrain: number;  // terrain type id
}

/** Water edge detection result */
export interface WaterEdge {
  tileX: number;
  tileY: number;
  edgeMask: number;  // NESW bitmask of water edges
}

export const TERRAIN_TYPES: TerrainTypeDef[] = [
  { id: 0, name: 'Grass', buildable: true, movementCost: 1.0, color: { r: 100, g: 180, b: 60 }, spriteBase: 1, variants: 4 },
  { id: 1, name: 'Water', buildable: false, movementCost: 0, color: { r: 40, g: 100, b: 200 }, spriteBase: 10, variants: 1 },
  { id: 2, name: 'Sand', buildable: true, movementCost: 1.2, color: { r: 220, g: 200, b: 140 }, spriteBase: 20, variants: 3 },
  { id: 3, name: 'Forest', buildable: false, movementCost: 2.0, color: { r: 40, g: 120, b: 30 }, spriteBase: 30, variants: 6 },
  { id: 4, name: 'Rock', buildable: false, movementCost: 0, color: { r: 140, g: 140, b: 140 }, spriteBase: 40, variants: 3 },
];

export const ELEVATION_RULES: ElevationRule[] = [
  { minElevation: 0, maxElevation: 0, defaultTerrain: 1 },  // water level
  { minElevation: 1, maxElevation: 2, defaultTerrain: 2 },  // sand/beach
  { minElevation: 3, maxElevation: 10, defaultTerrain: 0 }, // grass
  { minElevation: 11, maxElevation: 13, defaultTerrain: 3 }, // forest
  { minElevation: 14, maxElevation: 15, defaultTerrain: 4 }, // rock/mountain
];

/** Get terrain type definition by id */
export function getTerrainType(id: number): TerrainTypeDef | undefined {
  return TERRAIN_TYPES.find(t => t.id === id);
}

/** Get default terrain for an elevation */
export function terrainForElevation(elevation: number): number {
  for (const rule of ELEVATION_RULES) {
    if (elevation >= rule.minElevation && elevation <= rule.maxElevation) {
      return rule.defaultTerrain;
    }
  }
  return 0; // default grass
}

/** Check if a terrain type is buildable */
export function isBuildable(terrainId: number): boolean {
  const terrain = getTerrainType(terrainId);
  return terrain?.buildable ?? false;
}

/** Compute water edge mask for a tile */
export function computeWaterEdgeMask(
  isWater: (x: number, y: number) => boolean,
  tileX: number,
  tileY: number,
): number {
  if (!isWater(tileX, tileY)) return 0;
  let mask = 0;
  if (!isWater(tileX, tileY - 1)) mask |= 1; // North edge
  if (!isWater(tileX + 1, tileY)) mask |= 2; // East edge
  if (!isWater(tileX, tileY + 1)) mask |= 4; // South edge
  if (!isWater(tileX - 1, tileY)) mask |= 8; // West edge
  return mask;
}

/** Select terrain sprite variant based on position (deterministic) */
export function selectVariant(terrainId: number, tileX: number, tileY: number): number {
  const terrain = getTerrainType(terrainId);
  if (!terrain || terrain.variants <= 1) return 0;
  // Simple hash for deterministic variant selection
  return Math.abs((tileX * 7 + tileY * 13) % terrain.variants);
}

/** Validate terrain configuration */
export function validateTerrainConfig(): string[] {
  const errors: string[] = [];
  const ids = new Set<number>();
  for (const t of TERRAIN_TYPES) {
    if (ids.has(t.id)) errors.push(`Duplicate terrain id: ${t.id}`);
    ids.add(t.id);
    if (t.movementCost < 0) errors.push(`Negative movement cost for ${t.name}`);
    if (t.variants < 1) errors.push(`Invalid variant count for ${t.name}`);
  }
  return errors;
}
