use city_core::MapSize;
use serde::{Deserialize, Serialize};

// ─── Sim-specific enums ─────────────────────────────────────────────────────

/// Zoning classification for a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ZoneType {
    None        = 0,
    Residential = 1,
    Commercial  = 2,
    Industrial  = 3,
    Civic       = 4,
    Park        = 5,
    Transport   = 6,
}

/// Zone build density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ZoneDensity {
    Low    = 0,
    Medium = 1,
    High   = 2,
}

impl Default for ZoneDensity {
    fn default() -> Self {
        ZoneDensity::Low
    }
}

/// Base terrain material for a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TerrainType {
    Grass  = 0,
    Water  = 1,
    Sand   = 2,
    Forest = 3,
    Rock   = 4,
}

// ─── Fixed-point type aliases ───────────────────────────────────────────────

/// Q0.16 unsigned ratio in [0, 1). 0x0000 = 0.0, 0xFFFF ~ 0.99998.
pub type RatioQ0_16 = u16;

/// Q0.32 unsigned probability in [0, 1). Full u32 range.
pub type ProbabilityQ0_32 = u32;

/// Distance in millimeters. Max ~4,294 km.
pub type DistanceMm = u32;

/// Rate per tick in Q16.16 fixed point.
pub type RatePerTickQ16_16 = i32;

// ─── Time Constants ─────────────────────────────────────────────────────────

/// Simulation ticks executed per real-world second.
pub const SIM_TICKS_PER_REAL_SECOND: u32 = 20;

/// Game-seconds per tick, stored as a rational: numerator.
pub const GAME_SECONDS_PER_TICK_NUM: u32 = 12;
/// Game-seconds per tick, stored as a rational: denominator.
pub const GAME_SECONDS_PER_TICK_DEN: u32 = 10;

/// Ticks in one game day (24 game-hours).
pub const TICKS_PER_GAME_DAY: u64 = 72_000;

/// Ticks in one game hour.
pub const TICKS_PER_GAME_HOUR: u64 = 3_000;

/// Ticks in one game minute.
pub const TICKS_PER_GAME_MINUTE: u64 = 50;

/// Ticks in one game month (30 game-days).
pub const TICKS_PER_GAME_MONTH: u64 = 2_160_000;

/// Ticks in one game year (12 game-months).
pub const TICKS_PER_GAME_YEAR: u64 = 25_920_000;

// ─── Money ──────────────────────────────────────────────────────────────────

/// Money in cents (1/100 of base currency unit). Signed to allow debt.
pub type MoneyCents = i64;

// ─── Scale Constants ────────────────────────────────────────────────────────

/// Meters per tile edge.
pub const SIM_TILE_M: u32 = 16;

/// Square meters per tile.
pub const SIM_TILE_AREA_M2: u32 = 256;

/// Alias for `SIM_TILE_M` (real-world meters per simulation tile side).
pub const TILE_METERS: u32 = SIM_TILE_M;

/// Alias for `SIM_TILE_AREA_M2` (area per tile in m²).
pub const TILE_AREA_M2: u32 = SIM_TILE_AREA_M2;

/// Ticks per in-game day (legacy constant; prefer `TICKS_PER_GAME_DAY` for new code).
pub const TICKS_PER_DAY: u32 = 2880;

/// Render tile width in pixels.
pub const TILE_W_PX: u32 = 128;

/// Render tile height in pixels (isometric diamond half-height).
pub const TILE_H_PX: u32 = 64;

/// Resolution-independent sub-tile units per tile (for metadata/UV).
pub const TILE_UNITS_PER_TILE: u32 = 1024;

// ─── Map Size Presets ───────────────────────────────────────────────────────

/// Small map preset: 128x128 tiles.
pub const MAP_SIZE_SMALL: MapSize = MapSize::new(128, 128);
/// Medium map preset: 192x192 tiles.
pub const MAP_SIZE_MEDIUM: MapSize = MapSize::new(192, 192);
/// Large map preset: 256x256 tiles.
pub const MAP_SIZE_LARGE: MapSize = MapSize::new(256, 256);

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_type_values() {
        assert_eq!(ZoneType::None        as u8, 0);
        assert_eq!(ZoneType::Residential as u8, 1);
        assert_eq!(ZoneType::Commercial  as u8, 2);
        assert_eq!(ZoneType::Industrial  as u8, 3);
        assert_eq!(ZoneType::Civic       as u8, 4);
        assert_eq!(ZoneType::Park        as u8, 5);
        assert_eq!(ZoneType::Transport   as u8, 6);
    }

    #[test]
    fn zone_density_values() {
        assert_eq!(ZoneDensity::Low    as u8, 0);
        assert_eq!(ZoneDensity::Medium as u8, 1);
        assert_eq!(ZoneDensity::High   as u8, 2);
        assert_eq!(ZoneDensity::default(), ZoneDensity::Low);
    }

    #[test]
    fn zone_density_is_one_byte() {
        assert_eq!(std::mem::size_of::<ZoneDensity>(), 1);
    }

    #[test]
    fn terrain_type_values() {
        assert_eq!(TerrainType::Grass  as u8, 0);
        assert_eq!(TerrainType::Water  as u8, 1);
        assert_eq!(TerrainType::Sand   as u8, 2);
        assert_eq!(TerrainType::Forest as u8, 3);
        assert_eq!(TerrainType::Rock   as u8, 4);
    }

    #[test]
    fn time_constants_one_day() {
        assert_eq!(TICKS_PER_GAME_DAY, 72_000);
    }

    #[test]
    fn time_constants_one_hour() {
        assert_eq!(TICKS_PER_GAME_HOUR, 3_000);
    }

    #[test]
    fn time_constants_one_minute() {
        assert_eq!(TICKS_PER_GAME_MINUTE, 50);
    }

    #[test]
    fn time_constants_consistency() {
        assert_eq!(TICKS_PER_GAME_DAY, TICKS_PER_GAME_HOUR * 24);
        assert_eq!(TICKS_PER_GAME_HOUR, TICKS_PER_GAME_MINUTE * 60);
        assert_eq!(TICKS_PER_GAME_MONTH, TICKS_PER_GAME_DAY * 30);
        assert_eq!(TICKS_PER_GAME_YEAR, TICKS_PER_GAME_MONTH * 12);
    }

    #[test]
    fn scale_constants() {
        assert_eq!(SIM_TILE_M, 16);
        assert_eq!(SIM_TILE_AREA_M2, SIM_TILE_M * SIM_TILE_M);
        assert_eq!(TILE_W_PX, 128);
        assert_eq!(TILE_H_PX, 64);
        assert_eq!(TILE_UNITS_PER_TILE, 1024);
    }

    #[test]
    fn map_size_area_small() {
        assert_eq!(MAP_SIZE_SMALL.tile_count(), 128 * 128);
    }

    #[test]
    fn map_size_area_medium() {
        assert_eq!(MAP_SIZE_MEDIUM.tile_count(), 192 * 192);
    }

    #[test]
    fn map_size_area_large() {
        assert_eq!(MAP_SIZE_LARGE.tile_count(), 256 * 256);
    }
}
