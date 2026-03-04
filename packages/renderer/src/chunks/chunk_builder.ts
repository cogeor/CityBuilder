// @townbuilder/renderer — Static chunk builder for terrain, networks, buildings

import { CHUNK_SIZE } from './chunk_cache.js';
import {
  worldToScreen,
  TILE_W,
  TILE_H,
  depthKey,
  type CameraState,
} from '../projection/index.js';
import {
  RenderPass,
  INSTANCE_BYTE_SIZE,
  packInstance,
  type RenderInstance,
  DEFAULT_INSTANCE,
} from '../types/index.js';

// ─── Enums ──────────────────────────────────────────────────────────────────

/** Terrain type enum matching Rust TerrainType. */
export enum TerrainType {
  Grass = 0,
  Water = 1,
  Sand = 2,
  Forest = 3,
  Rock = 4,
}

/** Zone type matching Rust ZoneType. */
export enum ZoneType {
  None = 0,
  Residential = 1,
  Commercial = 2,
  Industrial = 3,
}

/** Road type for auto-tiling. */
export enum RoadType {
  None = 0,
  Local = 1,
  Collector = 2,
  Arterial = 3,
  Highway = 4,
}

// ─── Interfaces ─────────────────────────────────────────────────────────────

/** Simplified tile data for rendering. */
export interface TileRenderData {
  terrain: TerrainType;
  elevation: number; // 0-15
  zone: ZoneType;
  road: RoadType;
  entityId: number; // 0 if no building, else entity index
  archetypeId: number; // building archetype if entity present
  flags: number; // StatusFlags from engine
}

/** Sprite ID mapping result. */
export interface SpriteMapping {
  spriteId: number;
  atlasId: number;
  variant: number; // tile variant index (for auto-tiling)
}

/** Callback to resolve sprite for a terrain/road/building. */
export type SpriteResolver = (
  type: 'terrain' | 'road' | 'building',
  id: number,
  variant: number,
) => SpriteMapping;

// ─── Default Sprite Tables ─────────────────────────────────────────────────

const DEFAULT_TERRAIN_SPRITES: Record<TerrainType, number> = {
  [TerrainType.Grass]: 1,
  [TerrainType.Water]: 2,
  [TerrainType.Sand]: 3,
  [TerrainType.Forest]: 4,
  [TerrainType.Rock]: 5,
};

const DEFAULT_ROAD_SPRITES: Record<RoadType, number> = {
  [RoadType.None]: 0,
  [RoadType.Local]: 10,
  [RoadType.Collector]: 11,
  [RoadType.Arterial]: 12,
  [RoadType.Highway]: 13,
};

// ─── Auto-tiling Masks ─────────────────────────────────────────────────────

/**
 * Compute road connection bitmask from neighbors.
 * Bits: 0=North, 1=East, 2=South, 3=West
 * This gives 16 variants (0x0 to 0xF).
 */
export function computeRoadMask(
  hasNorth: boolean,
  hasEast: boolean,
  hasSouth: boolean,
  hasWest: boolean,
): number {
  return (
    (hasNorth ? 1 : 0) |
    (hasEast ? 2 : 0) |
    (hasSouth ? 4 : 0) |
    (hasWest ? 8 : 0)
  );
}

/**
 * Compute terrain edge variant based on neighbors.
 * Used for water edges, forest edges, etc.
 * Same bitmask approach as roads: N/E/S/W neighbor same type.
 */
export function computeTerrainEdgeMask(
  sameNorth: boolean,
  sameEast: boolean,
  sameSouth: boolean,
  sameWest: boolean,
): number {
  return (
    (sameNorth ? 1 : 0) |
    (sameEast ? 2 : 0) |
    (sameSouth ? 4 : 0) |
    (sameWest ? 8 : 0)
  );
}

// ─── Identity Camera ────────────────────────────────────────────────────────

/**
 * Identity camera at origin with no viewport offset.
 * Used for chunk building where screen positions are relative to world origin.
 */
const IDENTITY_CAMERA: CameraState = {
  x: 0,
  y: 0,
  zoom: 1,
  viewportWidth: 0,
  viewportHeight: 0,
};

// ─── ChunkBuilder ───────────────────────────────────────────────────────────

/**
 * Builds pre-sorted render instances for a 32x32 chunk of static tiles.
 *
 * Three passes per tile:
 *   Pass 0 (Terrain) — ground tiles with edge variant auto-tiling
 *   Pass 1 (Networks) — roads with connection-based auto-tiling
 *   Pass 2 (Buildings) — entities with shadows
 *
 * Screen positions are computed relative to world origin (identity camera).
 * The final camera transform is applied at render time.
 */
export class ChunkBuilder {
  private mapWidth: number;
  private mapHeight: number;
  private resolver: SpriteResolver | null;

  constructor(mapWidth: number, mapHeight: number, resolver?: SpriteResolver) {
    this.mapWidth = mapWidth;
    this.mapHeight = mapHeight;
    this.resolver = resolver ?? null;
  }

  /**
   * Build all render instances for a chunk.
   * Returns packed Float32Array and instance count.
   */
  buildChunk(
    chunkX: number,
    chunkY: number,
    getTile: (x: number, y: number) => TileRenderData | null,
  ): { instances: Float32Array; count: number } {
    const startX = chunkX * CHUNK_SIZE;
    const startY = chunkY * CHUNK_SIZE;
    const endX = Math.min(startX + CHUNK_SIZE, this.mapWidth);
    const endY = Math.min(startY + CHUNK_SIZE, this.mapHeight);

    const collected: RenderInstance[] = [];

    for (let y = startY; y < endY; y++) {
      for (let x = startX; x < endX; x++) {
        const tile = getTile(x, y);
        if (!tile) continue;

        // Pass 1: Terrain
        const edgeMask = this.computeEdgeMask(x, y, tile, getTile);
        collected.push(this.buildTerrainInstance(x, y, tile, edgeMask));

        // Pass 2: Networks (roads)
        if (tile.road !== RoadType.None) {
          const roadMask = this.computeNeighborRoadMask(x, y, getTile);
          collected.push(this.buildRoadInstance(x, y, tile, roadMask));
        }

        // Pass 3: Buildings (entities)
        if (tile.entityId > 0) {
          collected.push(this.buildBuildingInstance(x, y, tile));
          collected.push(this.buildShadowInstance(x, y, tile));
        }
      }
    }

    // Sort by depth (ascending = back-to-front)
    collected.sort((a, b) => a.z_order - b.z_order);

    // Pack into Float32Array
    const count = collected.length;
    const byteLength = count * INSTANCE_BYTE_SIZE;
    const buffer = new ArrayBuffer(byteLength);
    const view = new DataView(buffer);

    for (let i = 0; i < count; i++) {
      packInstance(collected[i], view, i * INSTANCE_BYTE_SIZE);
    }

    return { instances: new Float32Array(buffer), count };
  }

  // ─── Instance Builders ──────────────────────────────────────────────

  private buildTerrainInstance(
    x: number,
    y: number,
    tile: TileRenderData,
    edgeMask: number,
  ): RenderInstance {
    const screen = worldToScreen(x, y, tile.elevation, IDENTITY_CAMERA);
    const sprite = this.resolver
      ? this.resolver('terrain', tile.terrain, edgeMask)
      : {
          spriteId: DEFAULT_TERRAIN_SPRITES[tile.terrain] ?? 1,
          atlasId: 0,
          variant: edgeMask,
        };

    return {
      ...DEFAULT_INSTANCE,
      screen_x: screen.x,
      screen_y: screen.y,
      sprite_id: sprite.spriteId,
      atlas_id: sprite.atlasId,
      z_order: depthKey(x, y, 0, RenderPass.Terrain, 0),
      anim_frame: sprite.variant,
    };
  }

  private buildRoadInstance(
    x: number,
    y: number,
    tile: TileRenderData,
    roadMask: number,
  ): RenderInstance {
    const screen = worldToScreen(x, y, tile.elevation, IDENTITY_CAMERA);
    const sprite = this.resolver
      ? this.resolver('road', tile.road, roadMask)
      : {
          spriteId: DEFAULT_ROAD_SPRITES[tile.road] ?? 10,
          atlasId: 0,
          variant: roadMask,
        };

    return {
      ...DEFAULT_INSTANCE,
      screen_x: screen.x,
      screen_y: screen.y,
      sprite_id: sprite.spriteId,
      atlas_id: sprite.atlasId,
      z_order: depthKey(x, y, 0, RenderPass.Networks, 0),
      anim_frame: sprite.variant,
    };
  }

  private buildBuildingInstance(
    x: number,
    y: number,
    tile: TileRenderData,
  ): RenderInstance {
    const screen = worldToScreen(x, y, tile.elevation, IDENTITY_CAMERA);
    const sprite = this.resolver
      ? this.resolver('building', tile.archetypeId, 0)
      : {
          spriteId: tile.archetypeId * 10,
          atlasId: 0,
          variant: 0,
        };

    return {
      ...DEFAULT_INSTANCE,
      screen_x: screen.x,
      screen_y: screen.y,
      sprite_id: sprite.spriteId,
      atlas_id: sprite.atlasId,
      z_order: depthKey(x, y, tile.elevation, RenderPass.Buildings, 0),
      anim_frame: sprite.variant,
    };
  }

  private buildShadowInstance(
    x: number,
    y: number,
    tile: TileRenderData,
  ): RenderInstance {
    // Shadow sits at ground level, slightly offset in Y, drawn before building
    const screen = worldToScreen(x, y, 0, IDENTITY_CAMERA);
    const sprite = this.resolver
      ? this.resolver('building', tile.archetypeId, 0)
      : {
          spriteId: tile.archetypeId * 10,
          atlasId: 0,
          variant: 0,
        };

    return {
      ...DEFAULT_INSTANCE,
      screen_x: screen.x,
      screen_y: screen.y + TILE_H * 0.25, // offset shadow downward
      sprite_id: sprite.spriteId,
      atlas_id: sprite.atlasId,
      z_order: depthKey(x, y, 0, RenderPass.Terrain, 1), // behind building, on terrain layer
      tint_r: 0,
      tint_g: 0,
      tint_b: 0,
      tint_a: 128, // semi-transparent black
    };
  }

  // ─── Neighbor Queries ───────────────────────────────────────────────

  private computeEdgeMask(
    x: number,
    y: number,
    tile: TileRenderData,
    getTile: (x: number, y: number) => TileRenderData | null,
  ): number {
    const sameNorth = this.isSameTerrainAt(x, y - 1, tile.terrain, getTile);
    const sameEast = this.isSameTerrainAt(x + 1, y, tile.terrain, getTile);
    const sameSouth = this.isSameTerrainAt(x, y + 1, tile.terrain, getTile);
    const sameWest = this.isSameTerrainAt(x - 1, y, tile.terrain, getTile);
    return computeTerrainEdgeMask(sameNorth, sameEast, sameSouth, sameWest);
  }

  private isSameTerrainAt(
    x: number,
    y: number,
    terrain: TerrainType,
    getTile: (x: number, y: number) => TileRenderData | null,
  ): boolean {
    if (x < 0 || x >= this.mapWidth || y < 0 || y >= this.mapHeight) {
      return false;
    }
    const neighbor = getTile(x, y);
    return neighbor !== null && neighbor.terrain === terrain;
  }

  private computeNeighborRoadMask(
    x: number,
    y: number,
    getTile: (x: number, y: number) => TileRenderData | null,
  ): number {
    const hasNorth = this.hasRoadAt(x, y - 1, getTile);
    const hasEast = this.hasRoadAt(x + 1, y, getTile);
    const hasSouth = this.hasRoadAt(x, y + 1, getTile);
    const hasWest = this.hasRoadAt(x - 1, y, getTile);
    return computeRoadMask(hasNorth, hasEast, hasSouth, hasWest);
  }

  private hasRoadAt(
    x: number,
    y: number,
    getTile: (x: number, y: number) => TileRenderData | null,
  ): boolean {
    if (x < 0 || x >= this.mapWidth || y < 0 || y >= this.mapHeight) {
      return false;
    }
    const neighbor = getTile(x, y);
    return neighbor !== null && neighbor.road !== RoadType.None;
  }
}
