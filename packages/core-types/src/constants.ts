// @townbuilder/core-types — numeric constants
// All simulation-critical constants. No floating-point in sim logic.

// ─── Time Constants ─────────────────────────────────────────────────────────

/** Simulation ticks executed per real-world second. */
export const SIM_TICKS_PER_REAL_SECOND = 20;

/** Game-seconds per tick as rational: numerator. (1.2 = 12/10) */
export const GAME_SECONDS_PER_TICK_NUM = 12;

/** Game-seconds per tick as rational: denominator. */
export const GAME_SECONDS_PER_TICK_DEN = 10;

/** Ticks in one game day (24 game-hours). */
export const TICKS_PER_GAME_DAY = 72_000;

/** Ticks in one game hour. */
export const TICKS_PER_GAME_HOUR = 3_000;

/** Ticks in one game minute. */
export const TICKS_PER_GAME_MINUTE = 50;

/** Ticks in one game month (30 game-days). */
export const TICKS_PER_GAME_MONTH = 2_160_000;

/** Ticks in one game year (12 game-months). */
export const TICKS_PER_GAME_YEAR = 25_920_000;

// ─── Scale Constants ────────────────────────────────────────────────────────

/** Meters per tile edge. */
export const SIM_TILE_M = 16;

/** Square meters per tile. */
export const SIM_TILE_AREA_M2 = 256;

/** Render tile width in pixels. */
export const TILE_W_PX = 128;

/** Render tile height in pixels (isometric diamond half-height). */
export const TILE_H_PX = 64;

/** Resolution-independent sub-tile units per tile (for metadata/UV). */
export const TILE_UNITS_PER_TILE = 1024;

// ─── Map Size Presets ───────────────────────────────────────────────────────

/** Small map: 128x128 tiles. */
export const MAP_SIZE_SMALL_W = 128;
export const MAP_SIZE_SMALL_H = 128;

/** Medium map: 192x192 tiles. */
export const MAP_SIZE_MEDIUM_W = 192;
export const MAP_SIZE_MEDIUM_H = 192;

/** Large map: 256x256 tiles. */
export const MAP_SIZE_LARGE_W = 256;
export const MAP_SIZE_LARGE_H = 256;
