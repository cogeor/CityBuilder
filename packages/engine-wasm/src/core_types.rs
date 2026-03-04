//! Shared type aliases, structs, enums, and constants for the simulation engine.
//!
//! All simulation-critical numeric types use integer or fixed-point arithmetic.
//! No floating-point types are used in simulation logic.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

// ─── Fixed-Point Numeric Type Aliases ───────────────────────────────────────

/// Simulation tick counter. Monotonically increasing, never wraps in practice.
pub type Tick = u64;

/// Currency in 1/100 units (cents). Signed to allow debt.
pub type MoneyCents = i64;

/// Q16.16 fixed-point: 16 integer bits + 16 fractional bits.
/// Range: roughly -32768.0 .. +32767.99998
pub type QuantityQ16_16 = i32;

/// Q0.16 unsigned ratio in [0, 1). 0x0000 = 0.0, 0xFFFF ~ 0.99998.
pub type RatioQ0_16 = u16;

/// Q0.32 unsigned probability in [0, 1). Full u32 range.
pub type ProbabilityQ0_32 = u32;

/// Distance in millimeters. Max ~4,294 km.
pub type DistanceMm = u32;

/// Rate per tick in Q16.16 fixed point.
pub type RatePerTickQ16_16 = i32;

/// Entity index for wire transfer (no generation).
pub type EntityId = u32;

/// Archetype identifier.
pub type ArchetypeId = u16;

// ─── EntityHandle ───────────────────────────────────────────────────────────

/// Generational entity handle for safe entity references.
///
/// `index` is the slot in the SoA storage array.
/// `generation` is incremented each time a slot is recycled; stale handles
/// are detected by comparing generation counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityHandle {
    pub index: u32,
    pub generation: u32,
}

impl EntityHandle {
    /// Sentinel value representing "no entity".
    pub const INVALID: EntityHandle = EntityHandle {
        index: u32::MAX,
        generation: 0,
    };

    /// Create a new handle with the given index and generation.
    #[inline]
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns `true` if this handle is not the INVALID sentinel.
    #[inline]
    pub fn is_valid(&self) -> bool {
        *self != Self::INVALID
    }
}

// ─── TileCoord ──────────────────────────────────────────────────────────────

/// Signed tile coordinate. Supports negative values for editor or offset math.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: i16,
    pub y: i16,
}

impl TileCoord {
    /// Create a new tile coordinate.
    #[inline]
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    /// Manhattan distance to another tile.
    #[inline]
    pub fn manhattan_distance(&self, other: &TileCoord) -> u32 {
        let dx = (self.x as i32 - other.x as i32).unsigned_abs();
        let dy = (self.y as i32 - other.y as i32).unsigned_abs();
        dx + dy
    }
}

impl Add for TileCoord {
    type Output = TileCoord;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        TileCoord {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for TileCoord {
    type Output = TileCoord;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        TileCoord {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

// ─── MapSize ────────────────────────────────────────────────────────────────

/// Map dimensions in tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapSize {
    pub width: u16,
    pub height: u16,
}

impl MapSize {
    /// Create a new map size.
    #[inline]
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    /// Total number of tiles in the map.
    #[inline]
    pub const fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }
}

/// Small map preset: 128x128 tiles.
pub const MAP_SIZE_SMALL: MapSize = MapSize::new(128, 128);
/// Medium map preset: 192x192 tiles.
pub const MAP_SIZE_MEDIUM: MapSize = MapSize::new(192, 192);
/// Large map preset: 256x256 tiles.
pub const MAP_SIZE_LARGE: MapSize = MapSize::new(256, 256);

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

// ─── Scale Constants ────────────────────────────────────────────────────────

/// Meters per tile edge.
pub const SIM_TILE_M: u32 = 16;

/// Square meters per tile.
pub const SIM_TILE_AREA_M2: u32 = 256;

/// Render tile width in pixels.
pub const TILE_W_PX: u32 = 128;

/// Render tile height in pixels (isometric diamond half-height).
pub const TILE_H_PX: u32 = 64;

/// Resolution-independent sub-tile units per tile (for metadata/UV).
pub const TILE_UNITS_PER_TILE: u32 = 1024;

// ─── ZoneType ───────────────────────────────────────────────────────────────

/// Zoning classification for a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ZoneType {
    None = 0,
    Residential = 1,
    Commercial = 2,
    Industrial = 3,
    Civic = 4,
}

// ─── TerrainType ────────────────────────────────────────────────────────────

/// Base terrain material for a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TerrainType {
    Grass = 0,
    Water = 1,
    Sand = 2,
    Forest = 3,
    Rock = 4,
}

// ─── StatusFlags ────────────────────────────────────────────────────────────

/// Bitflag newtype for per-entity status indicators.
///
/// Uses a `u16` backing store. Each flag is a single bit.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StatusFlags(pub u16);

impl StatusFlags {
    pub const NONE: StatusFlags = StatusFlags(0);
    pub const POWERED: StatusFlags = StatusFlags(1 << 0);
    pub const HAS_WATER: StatusFlags = StatusFlags(1 << 1);
    pub const STAFFED: StatusFlags = StatusFlags(1 << 2);
    pub const UNDER_CONSTRUCTION: StatusFlags = StatusFlags(1 << 3);
    pub const ON_FIRE: StatusFlags = StatusFlags(1 << 4);
    pub const DAMAGED: StatusFlags = StatusFlags(1 << 5);

    /// Returns `true` if all bits in `other` are set in `self`.
    #[inline]
    pub const fn contains(self, other: StatusFlags) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Set the bits of `other` in `self`.
    #[inline]
    pub const fn insert(self, other: StatusFlags) -> StatusFlags {
        StatusFlags(self.0 | other.0)
    }

    /// Clear the bits of `other` from `self`.
    #[inline]
    pub const fn remove(self, other: StatusFlags) -> StatusFlags {
        StatusFlags(self.0 & !other.0)
    }

    /// Toggle the bits of `other` in `self`.
    #[inline]
    pub const fn toggle(self, other: StatusFlags) -> StatusFlags {
        StatusFlags(self.0 ^ other.0)
    }

    /// Returns `true` if no flags are set.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Debug for StatusFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        let mut remaining = self.0;

        let flags = [
            (Self::POWERED.0, "POWERED"),
            (Self::HAS_WATER.0, "HAS_WATER"),
            (Self::STAFFED.0, "STAFFED"),
            (Self::UNDER_CONSTRUCTION.0, "UNDER_CONSTRUCTION"),
            (Self::ON_FIRE.0, "ON_FIRE"),
            (Self::DAMAGED.0, "DAMAGED"),
        ];

        write!(f, "StatusFlags(")?;
        for (bit, name) in flags {
            if remaining & bit != 0 {
                if !first {
                    write!(f, " | ")?;
                }
                write!(f, "{}", name)?;
                remaining &= !bit;
                first = false;
            }
        }
        if remaining != 0 {
            if !first {
                write!(f, " | ")?;
            }
            write!(f, "0x{:04x}", remaining)?;
        }
        if first {
            write!(f, "NONE")?;
        }
        write!(f, ")")
    }
}

impl std::ops::BitOr for StatusFlags {
    type Output = StatusFlags;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        StatusFlags(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for StatusFlags {
    type Output = StatusFlags;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        StatusFlags(self.0 & rhs.0)
    }
}

impl std::ops::BitXor for StatusFlags {
    type Output = StatusFlags;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        StatusFlags(self.0 ^ rhs.0)
    }
}

impl std::ops::Not for StatusFlags {
    type Output = StatusFlags;

    #[inline]
    fn not(self) -> Self::Output {
        StatusFlags(!self.0)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn entity_handle_size() {
        assert_eq!(mem::size_of::<EntityHandle>(), 8);
    }

    #[test]
    fn tile_coord_size() {
        assert_eq!(mem::size_of::<TileCoord>(), 4);
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
        // 1 day = 24 hours
        assert_eq!(TICKS_PER_GAME_DAY, TICKS_PER_GAME_HOUR * 24);
        // 1 hour = 60 minutes
        assert_eq!(TICKS_PER_GAME_HOUR, TICKS_PER_GAME_MINUTE * 60);
        // 1 month = 30 days
        assert_eq!(TICKS_PER_GAME_MONTH, TICKS_PER_GAME_DAY * 30);
        // 1 year = 12 months
        assert_eq!(TICKS_PER_GAME_YEAR, TICKS_PER_GAME_MONTH * 12);
    }

    #[test]
    fn entity_handle_invalid() {
        let h = EntityHandle::INVALID;
        assert_eq!(h.index, u32::MAX);
        assert_eq!(h.generation, 0);
        assert!(!h.is_valid());
    }

    #[test]
    fn entity_handle_valid() {
        let h = EntityHandle::new(0, 1);
        assert!(h.is_valid());
    }

    #[test]
    fn tile_coord_add() {
        let a = TileCoord::new(3, 5);
        let b = TileCoord::new(-1, 2);
        let c = a + b;
        assert_eq!(c, TileCoord::new(2, 7));
    }

    #[test]
    fn tile_coord_sub() {
        let a = TileCoord::new(10, 20);
        let b = TileCoord::new(3, 8);
        let c = a - b;
        assert_eq!(c, TileCoord::new(7, 12));
    }

    #[test]
    fn tile_coord_manhattan_distance() {
        let a = TileCoord::new(0, 0);
        let b = TileCoord::new(3, 4);
        assert_eq!(a.manhattan_distance(&b), 7);
    }

    #[test]
    fn tile_coord_manhattan_distance_negative() {
        let a = TileCoord::new(-5, 3);
        let b = TileCoord::new(5, -3);
        assert_eq!(a.manhattan_distance(&b), 16);
    }

    #[test]
    fn status_flags_empty() {
        let f = StatusFlags::NONE;
        assert!(f.is_empty());
        assert!(!f.contains(StatusFlags::POWERED));
    }

    #[test]
    fn status_flags_insert_and_contains() {
        let f = StatusFlags::NONE
            .insert(StatusFlags::POWERED)
            .insert(StatusFlags::HAS_WATER);
        assert!(f.contains(StatusFlags::POWERED));
        assert!(f.contains(StatusFlags::HAS_WATER));
        assert!(!f.contains(StatusFlags::ON_FIRE));
    }

    #[test]
    fn status_flags_remove() {
        let f = StatusFlags::POWERED
            .insert(StatusFlags::STAFFED)
            .remove(StatusFlags::POWERED);
        assert!(!f.contains(StatusFlags::POWERED));
        assert!(f.contains(StatusFlags::STAFFED));
    }

    #[test]
    fn status_flags_toggle() {
        let f = StatusFlags::POWERED.toggle(StatusFlags::POWERED);
        assert!(f.is_empty());
        let f = f.toggle(StatusFlags::ON_FIRE);
        assert!(f.contains(StatusFlags::ON_FIRE));
    }

    #[test]
    fn status_flags_bitor() {
        let f = StatusFlags::POWERED | StatusFlags::ON_FIRE;
        assert!(f.contains(StatusFlags::POWERED));
        assert!(f.contains(StatusFlags::ON_FIRE));
    }

    #[test]
    fn status_flags_bitand() {
        let a = StatusFlags::POWERED | StatusFlags::ON_FIRE | StatusFlags::STAFFED;
        let b = StatusFlags::POWERED | StatusFlags::DAMAGED;
        let c = a & b;
        assert!(c.contains(StatusFlags::POWERED));
        assert!(!c.contains(StatusFlags::ON_FIRE));
        assert!(!c.contains(StatusFlags::DAMAGED));
    }

    #[test]
    fn map_size_area_small() {
        assert_eq!(MAP_SIZE_SMALL.area(), 128 * 128);
    }

    #[test]
    fn map_size_area_medium() {
        assert_eq!(MAP_SIZE_MEDIUM.area(), 192 * 192);
    }

    #[test]
    fn map_size_area_large() {
        assert_eq!(MAP_SIZE_LARGE.area(), 256 * 256);
    }

    #[test]
    fn map_size_custom_area() {
        let m = MapSize::new(100, 50);
        assert_eq!(m.area(), 5000);
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
    fn zone_type_values() {
        assert_eq!(ZoneType::None as u8, 0);
        assert_eq!(ZoneType::Residential as u8, 1);
        assert_eq!(ZoneType::Commercial as u8, 2);
        assert_eq!(ZoneType::Industrial as u8, 3);
        assert_eq!(ZoneType::Civic as u8, 4);
    }

    #[test]
    fn terrain_type_values() {
        assert_eq!(TerrainType::Grass as u8, 0);
        assert_eq!(TerrainType::Water as u8, 1);
        assert_eq!(TerrainType::Sand as u8, 2);
        assert_eq!(TerrainType::Forest as u8, 3);
        assert_eq!(TerrainType::Rock as u8, 4);
    }
}
