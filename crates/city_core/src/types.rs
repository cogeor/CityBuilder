//! Core type aliases and constants for the city builder engine.
//!
//! These are engine-level primitives — no game-specific types here.
//! Rendering constants live in `city_render`, game content in `city_game`.

use serde::{Deserialize, Serialize};

// ─── Simulation Primitives ──────────────────────────────────────────────────

/// Simulation tick counter (monotonically increasing).
pub type Tick = u64;

/// Money in cents (1/100 of base currency unit). Signed to allow debt.
pub type MoneyCents = i64;

/// Fixed-point Q16.16 numeric type for deterministic math.
pub type Fixed = i32;

/// Entity generation counter for the generational handle system.
pub type Generation = u32;

/// Unique entity slot index.
pub type EntityId = u32;

/// Archetype identifier (indexes into the archetype registry).
pub type ArchetypeId = u16;

// ─── Coordinates ────────────────────────────────────────────────────────────

/// Tile coordinate in the world grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: i16,
    pub y: i16,
}

impl TileCoord {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    /// Manhattan distance to another tile.
    pub fn manhattan_distance(self, other: TileCoord) -> u32 {
        let dx = (self.x as i32 - other.x as i32).unsigned_abs();
        let dy = (self.y as i32 - other.y as i32).unsigned_abs();
        dx + dy
    }
}

impl std::ops::Add for TileCoord {
    type Output = TileCoord;
    fn add(self, rhs: Self) -> Self::Output {
        TileCoord { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl std::ops::Sub for TileCoord {
    type Output = TileCoord;
    fn sub(self, rhs: Self) -> Self::Output {
        TileCoord { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

/// Map dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSize {
    pub width: u16,
    pub height: u16,
}

impl MapSize {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub const fn tile_count(&self) -> u32 {
        self.width as u32 * self.height as u32
    }

    pub fn contains(&self, coord: TileCoord) -> bool {
        coord.x >= 0
            && coord.y >= 0
            && (coord.x as u16) < self.width
            && (coord.y as u16) < self.height
    }
}

// ─── Entity Handle ──────────────────────────────────────────────────────────

/// Generational handle to an entity. The generation prevents use-after-free.
///
/// `index` is the slot in the SoA storage array.
/// `generation` is incremented each time a slot is recycled; stale handles
/// are detected by comparing generation counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityHandle {
    pub index: u32,
    pub generation: Generation,
}

impl EntityHandle {
    /// Sentinel value representing "no entity".
    pub const INVALID: EntityHandle = EntityHandle {
        index: u32::MAX,
        generation: 0,
    };

    #[inline]
    pub const fn new(index: u32, generation: Generation) -> Self {
        Self { index, generation }
    }

    /// Returns `true` if this handle is not the INVALID sentinel.
    #[inline]
    pub fn is_valid(&self) -> bool {
        *self != Self::INVALID
    }
}

// ─── Status Flags ───────────────────────────────────────────────────────────

/// Bitflags for entity status (used by both sim and render).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StatusFlags(u16);

impl StatusFlags {
    pub const NONE: Self = Self(0);
    pub const POWERED: Self = Self(1 << 0);
    pub const WATER_CONNECTED: Self = Self(1 << 1);
    pub const ROAD_CONNECTED: Self = Self(1 << 2);
    pub const ON_FIRE: Self = Self(1 << 3);
    pub const DAMAGED: Self = Self(1 << 4);
    pub const UNDER_CONSTRUCTION: Self = Self(1 << 5);
    pub const ABANDONED: Self = Self(1 << 6);
    pub const UPGRADING: Self = Self(1 << 7);
    pub const STAFFED: Self = Self(1 << 8);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for StatusFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

impl std::ops::BitAnd for StatusFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self { Self(self.0 & rhs.0) }
}

impl std::ops::BitOrAssign for StatusFlags {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

// ─── Scale Constants ────────────────────────────────────────────────────────

/// Real-world meters per simulation tile side.
pub const TILE_METERS: u32 = 16;

/// Area per tile in m².
pub const TILE_AREA_M2: u32 = TILE_METERS * TILE_METERS;

/// Ticks per in-game day.
pub const TICKS_PER_DAY: u32 = 2880;

// ─── Fixed-Point Helpers ────────────────────────────────────────────────────

/// Convert an integer to Q16.16 fixed-point.
pub const fn to_fixed(n: i32) -> Fixed { n << 16 }

/// Convert Q16.16 fixed-point to integer (truncating).
pub const fn from_fixed(f: Fixed) -> i32 { f >> 16 }

/// Multiply two Q16.16 values.
pub const fn fixed_mul(a: Fixed, b: Fixed) -> Fixed {
    ((a as i64 * b as i64) >> 16) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_coord_manhattan() {
        let a = TileCoord::new(0, 0);
        let b = TileCoord::new(3, 4);
        assert_eq!(a.manhattan_distance(b), 7);
    }

    #[test]
    fn tile_coord_add_sub() {
        let a = TileCoord::new(1, 2);
        let b = TileCoord::new(3, 4);
        assert_eq!(a + b, TileCoord::new(4, 6));
        assert_eq!(b - a, TileCoord::new(2, 2));
    }

    #[test]
    fn map_size_contains() {
        let m = MapSize::new(64, 64);
        assert!(m.contains(TileCoord::new(0, 0)));
        assert!(m.contains(TileCoord::new(63, 63)));
        assert!(!m.contains(TileCoord::new(64, 0)));
        assert!(!m.contains(TileCoord::new(-1, 0)));
    }

    #[test]
    fn entity_handle_invalid() {
        let h = EntityHandle::INVALID;
        assert!(!h.is_valid());
        let h2 = EntityHandle::new(0, 0);
        assert!(h2.is_valid());
    }

    #[test]
    fn status_flags_ops() {
        let f = StatusFlags::POWERED | StatusFlags::ON_FIRE;
        assert!(f.contains(StatusFlags::POWERED));
        assert!(f.contains(StatusFlags::ON_FIRE));
        assert!(!f.contains(StatusFlags::DAMAGED));
    }

    #[test]
    fn fixed_point() {
        let a = to_fixed(3);
        let b = to_fixed(4);
        assert_eq!(from_fixed(fixed_mul(a, b)), 12);
    }
}
