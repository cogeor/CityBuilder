// @townbuilder/core-types — shared type definitions
// TypeScript mirror of engine-wasm/src/core_types.rs

// Re-export all constants
export {
  SIM_TICKS_PER_REAL_SECOND,
  GAME_SECONDS_PER_TICK_NUM,
  GAME_SECONDS_PER_TICK_DEN,
  TICKS_PER_GAME_DAY,
  TICKS_PER_GAME_HOUR,
  TICKS_PER_GAME_MINUTE,
  TICKS_PER_GAME_MONTH,
  TICKS_PER_GAME_YEAR,
  SIM_TILE_M,
  SIM_TILE_AREA_M2,
  TILE_W_PX,
  TILE_H_PX,
  TILE_UNITS_PER_TILE,
  MAP_SIZE_SMALL_W,
  MAP_SIZE_SMALL_H,
  MAP_SIZE_MEDIUM_W,
  MAP_SIZE_MEDIUM_H,
  MAP_SIZE_LARGE_W,
  MAP_SIZE_LARGE_H,
} from "./constants.js";

// ─── Fixed-Point Numeric Type Aliases ───────────────────────────────────────

/**
 * Simulation tick counter. Uses bigint because u64 exceeds safe integer range.
 * Monotonically increasing, never wraps in practice.
 */
export type Tick = bigint;

/**
 * Currency in 1/100 units (cents). Signed to allow debt.
 * Uses bigint because i64 exceeds safe integer range.
 */
export type MoneyCents = bigint;

/**
 * Q16.16 fixed-point: 16 integer bits + 16 fractional bits.
 * Fits in JS safe integer range (i32).
 */
export type QuantityQ16_16 = number;

/**
 * Q0.16 unsigned ratio in [0, 1). 0x0000 = 0.0, 0xFFFF ~ 0.99998.
 * Fits in JS safe integer range (u16).
 */
export type RatioQ0_16 = number;

/**
 * Q0.32 unsigned probability in [0, 1). Full u32 range.
 * Fits in JS safe integer range (u32).
 */
export type ProbabilityQ0_32 = number;

/**
 * Distance in millimeters. Max ~4,294 km.
 * Fits in JS safe integer range (u32).
 */
export type DistanceMm = number;

/**
 * Rate per tick in Q16.16 fixed point.
 * Fits in JS safe integer range (i32).
 */
export type RatePerTickQ16_16 = number;

/**
 * Entity index for wire transfer (no generation). u32.
 */
export type EntityId = number;

/**
 * Archetype identifier. u16.
 */
export type ArchetypeId = number;

// ─── EntityHandle ───────────────────────────────────────────────────────────

/**
 * Generational entity handle for safe entity references.
 * Mirrors Rust's EntityHandle { index: u32, generation: u32 }.
 */
export interface EntityHandle {
  readonly index: number;
  readonly generation: number;
}

/** Sentinel value representing "no entity". */
export const ENTITY_HANDLE_INVALID: EntityHandle = {
  index: 0xFFFFFFFF,
  generation: 0,
} as const;

/** Create a new EntityHandle. */
export function entityHandle(index: number, generation: number): EntityHandle {
  return { index, generation };
}

/** Returns true if the handle is not the INVALID sentinel. */
export function isEntityHandleValid(h: EntityHandle): boolean {
  return h.index !== ENTITY_HANDLE_INVALID.index || h.generation !== ENTITY_HANDLE_INVALID.generation;
}

// ─── TileCoord ──────────────────────────────────────────────────────────────

/**
 * Signed tile coordinate. Mirrors Rust's TileCoord { x: i16, y: i16 }.
 */
export interface TileCoord {
  readonly x: number;
  readonly y: number;
}

/** Create a new TileCoord. */
export function tileCoord(x: number, y: number): TileCoord {
  return { x, y };
}

/** Add two tile coordinates. */
export function tileCoordAdd(a: TileCoord, b: TileCoord): TileCoord {
  return { x: a.x + b.x, y: a.y + b.y };
}

/** Subtract two tile coordinates. */
export function tileCoordSub(a: TileCoord, b: TileCoord): TileCoord {
  return { x: a.x - b.x, y: a.y - b.y };
}

/** Manhattan distance between two tile coordinates. */
export function tileCoordManhattan(a: TileCoord, b: TileCoord): number {
  return Math.abs(a.x - b.x) + Math.abs(a.y - b.y);
}

// ─── MapSize ────────────────────────────────────────────────────────────────

/**
 * Map dimensions in tiles. Mirrors Rust's MapSize { width: u16, height: u16 }.
 */
export interface MapSize {
  readonly width: number;
  readonly height: number;
}

/** Create a new MapSize. */
export function mapSize(width: number, height: number): MapSize {
  return { width, height };
}

/** Total number of tiles in the map. */
export function mapSizeArea(m: MapSize): number {
  return m.width * m.height;
}

/** Small map preset. */
export const MAP_SMALL: MapSize = { width: 128, height: 128 } as const;

/** Medium map preset. */
export const MAP_MEDIUM: MapSize = { width: 192, height: 192 } as const;

/** Large map preset. */
export const MAP_LARGE: MapSize = { width: 256, height: 256 } as const;

// ─── ZoneType ───────────────────────────────────────────────────────────────

/** Zoning classification for a tile. Mirrors Rust's ZoneType enum. */
export const enum ZoneType {
  None = 0,
  Residential = 1,
  Commercial = 2,
  Industrial = 3,
  Civic = 4,
}

// ─── TerrainType ────────────────────────────────────────────────────────────

/** Base terrain material for a tile. Mirrors Rust's TerrainType enum. */
export const enum TerrainType {
  Grass = 0,
  Water = 1,
  Sand = 2,
  Forest = 3,
  Rock = 4,
}

// ─── StatusFlags ────────────────────────────────────────────────────────────

/**
 * Bitflag constants for per-entity status indicators.
 * Use bitwise operations to combine:
 *   const flags = StatusFlags.POWERED | StatusFlags.HAS_WATER;
 *   const hasPower = (flags & StatusFlags.POWERED) !== 0;
 */
export const StatusFlags = {
  NONE: 0,
  POWERED: 1 << 0,
  HAS_WATER: 1 << 1,
  STAFFED: 1 << 2,
  UNDER_CONSTRUCTION: 1 << 3,
  ON_FIRE: 1 << 4,
  DAMAGED: 1 << 5,
} as const;

export type StatusFlagsValue = number;
