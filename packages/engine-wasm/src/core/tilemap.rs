//! Per-tile bitmap data model.
//!
//! Packs all tile state into 4 bytes per tile (`TileValue`), mirroring the
//! SimCity C bitmap approach in safe, typed Rust.
//! A 256x256 map holds 262 144 tiles = 1 MiB of tile data.

use serde::{Deserialize, Serialize};

use crate::core_types::{TerrainType, ZoneType};

// ─── TileKind ────────────────────────────────────────────────────────────────

/// Overlay classification for a tile.
///
/// `Empty` means bare terrain with no overlay placed on top.
/// `Zone` means the tile is zoned but no building has been placed yet.
/// `Utility` is a catch-all for infrastructure that is neither a road, power
/// line, nor water pipe (e.g. a pump station base tile).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TileKind {
    Empty     = 0,
    Road      = 1,
    Zone      = 2,
    Building  = 3,
    Utility   = 4,
    PowerLine = 5,
    WaterPipe = 6,
}

impl Default for TileKind {
    fn default() -> Self {
        TileKind::Empty
    }
}

// ─── TileFlags ───────────────────────────────────────────────────────────────

/// Per-tile service and state flags packed into one byte.
///
/// Follows the same hand-rolled bitflag pattern as `StatusFlags` in
/// `core_types.rs`. Bits 6-7 are reserved for future use.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileFlags(pub u8);

impl TileFlags {
    pub const NONE:               TileFlags = TileFlags(0);
    pub const POWERED:            TileFlags = TileFlags(1 << 0); // 0x01
    pub const WATERED:            TileFlags = TileFlags(1 << 1); // 0x02
    pub const ROAD_ACCESS:        TileFlags = TileFlags(1 << 2); // 0x04
    pub const UNDER_CONSTRUCTION: TileFlags = TileFlags(1 << 3); // 0x08
    pub const BULLDOZABLE:        TileFlags = TileFlags(1 << 4); // 0x10
    pub const ON_FIRE:            TileFlags = TileFlags(1 << 5); // 0x20
    // bits 6-7 reserved

    /// Returns `true` if all bits of `other` are set in `self`.
    #[inline]
    pub const fn contains(self, other: TileFlags) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Set the bits of `other` in `self`.
    #[inline]
    pub fn insert(&mut self, other: TileFlags) {
        self.0 |= other.0;
    }

    /// Clear the bits of `other` from `self`.
    #[inline]
    pub fn remove(&mut self, other: TileFlags) {
        self.0 &= !other.0;
    }

    /// Returns `true` if no flags are set.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl Default for TileFlags {
    fn default() -> Self {
        TileFlags::NONE
    }
}

impl std::fmt::Debug for TileFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let flags = [
            (Self::POWERED.0,            "POWERED"),
            (Self::WATERED.0,            "WATERED"),
            (Self::ROAD_ACCESS.0,        "ROAD_ACCESS"),
            (Self::UNDER_CONSTRUCTION.0, "UNDER_CONSTRUCTION"),
            (Self::BULLDOZABLE.0,        "BULLDOZABLE"),
            (Self::ON_FIRE.0,            "ON_FIRE"),
        ];
        let mut first = true;
        let mut remaining = self.0;
        write!(f, "TileFlags(")?;
        for (bit, name) in flags {
            if remaining & bit != 0 {
                if !first { write!(f, " | ")?; }
                write!(f, "{}", name)?;
                remaining &= !bit;
                first = false;
            }
        }
        if remaining != 0 {
            if !first { write!(f, " | ")?; }
            write!(f, "0x{:02x}", remaining)?;
        }
        if first { write!(f, "NONE")?; }
        write!(f, ")")
    }
}

impl std::ops::BitOr for TileFlags {
    type Output = TileFlags;
    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output { TileFlags(self.0 | rhs.0) }
}

impl std::ops::BitAnd for TileFlags {
    type Output = TileFlags;
    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output { TileFlags(self.0 & rhs.0) }
}

impl std::ops::BitXor for TileFlags {
    type Output = TileFlags;
    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output { TileFlags(self.0 ^ rhs.0) }
}

impl std::ops::Not for TileFlags {
    type Output = TileFlags;
    #[inline]
    fn not(self) -> Self::Output { TileFlags(!self.0) }
}

// ─── TileValue ───────────────────────────────────────────────────────────────

/// Packed per-tile data: exactly 4 bytes.
///
/// Field layout (all fields are `u8`-sized enums or newtypes):
///   byte 0 — terrain: TerrainType
///   byte 1 — kind:    TileKind
///   byte 2 — zone:    ZoneType  (None when kind != Zone / Building)
///   byte 3 — flags:   TileFlags
///
/// Density is encoded in `flags` bits 6-7 (reserved, to be used by a later
/// loop) or in the `data` supplementary byte provided by the caller. For now,
/// density is left to the higher-level building/zone system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub struct TileValue {
    pub terrain: TerrainType,
    pub kind:    TileKind,
    pub zone:    ZoneType,
    pub flags:   TileFlags,
}

impl TileValue {
    /// Default tile: grass terrain, no overlay, no zone, no flags.
    pub const DEFAULT: TileValue = TileValue {
        terrain: TerrainType::Grass,
        kind:    TileKind::Empty,
        zone:    ZoneType::None,
        flags:   TileFlags::NONE,
    };
}

impl Default for TileValue {
    fn default() -> Self {
        TileValue::DEFAULT
    }
}

// ─── Coordinate helpers ──────────────────────────────────────────────────────

/// Row-major flat index: `idx = y * width + x`.
///
/// Panics in debug builds if `x >= width` or `y >= height` (caller is
/// responsible for bounds checking). Use `TileMap::tile_index` for a
/// checked version.
#[inline]
pub fn tile_index(x: u32, y: u32, width: u32) -> usize {
    y as usize * width as usize + x as usize
}

/// Convert a flat index back to `(x, y)`.
///
/// `width` must not be zero.
#[inline]
pub fn index_to_coord(idx: usize, width: u32) -> (u32, u32) {
    let w = width as usize;
    ((idx % w) as u32, (idx / w) as u32)
}

/// Cardinal (N, E, S, W) neighbours of tile `(x, y)` on a map of `width` x
/// `height`. Returns `None` in any direction that would be out of bounds.
///
/// Order: `[N, E, S, W]`
pub fn tile_neighbors_4(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> [Option<(u32, u32)>; 4] {
    [
        if y > 0          { Some((x, y - 1)) } else { None }, // N
        if x + 1 < width  { Some((x + 1, y)) } else { None }, // E
        if y + 1 < height { Some((x, y + 1)) } else { None }, // S
        if x > 0          { Some((x - 1, y)) } else { None }, // W
    ]
}

// ─── TileMap ─────────────────────────────────────────────────────────────────

/// Flat, row-major grid of `TileValue` cells.
///
/// Indexing: `idx = y * width + x`.
pub struct TileMap {
    tiles:  Vec<TileValue>,
    width:  u32,
    height: u32,
}

impl TileMap {
    /// Create a new map filled with `TileValue::DEFAULT`.
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize).saturating_mul(height as usize);
        Self {
            tiles: vec![TileValue::DEFAULT; len],
            width,
            height,
        }
    }

    /// Create a new map where every tile is set to `fill`.
    pub fn new_with(width: u32, height: u32, fill: TileValue) -> Self {
        let len = (width as usize).saturating_mul(height as usize);
        Self {
            tiles: vec![fill; len],
            width,
            height,
        }
    }

    /// Map width in tiles.
    #[inline]
    pub fn width(&self) -> u32 { self.width }

    /// Map height in tiles.
    #[inline]
    pub fn height(&self) -> u32 { self.height }

    /// Returns `true` if `(x, y)` is within the map bounds.
    #[inline]
    pub fn in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    /// Row-major index for `(x, y)`. Returns `None` if out of bounds.
    #[inline]
    pub fn tile_index(&self, x: u32, y: u32) -> Option<usize> {
        if self.in_bounds(x, y) {
            Some(tile_index(x, y, self.width))
        } else {
            None
        }
    }

    /// Convert a flat index to `(x, y)`. Returns `None` if `idx` is out of range.
    #[inline]
    pub fn index_to_coord(&self, idx: usize) -> Option<(u32, u32)> {
        if idx < self.tiles.len() && self.width > 0 {
            Some(index_to_coord(idx, self.width))
        } else {
            None
        }
    }

    /// Get a copy of the tile at `(x, y)`. Returns `None` if out of bounds.
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> Option<TileValue> {
        self.tile_index(x, y).map(|i| self.tiles[i])
    }

    /// Get a mutable reference to the tile at `(x, y)`. Returns `None` if
    /// out of bounds.
    #[inline]
    pub fn get_mut(&mut self, x: u32, y: u32) -> Option<&mut TileValue> {
        self.tile_index(x, y).map(|i| &mut self.tiles[i])
    }

    /// Overwrite the tile at `(x, y)`. Returns `false` if out of bounds.
    #[inline]
    pub fn set(&mut self, x: u32, y: u32, val: TileValue) -> bool {
        if let Some(i) = self.tile_index(x, y) {
            self.tiles[i] = val;
            true
        } else {
            false
        }
    }

    /// Set additional flags on the tile at `(x, y)`. Returns `false` if out
    /// of bounds.
    #[inline]
    pub fn set_flags(&mut self, x: u32, y: u32, flags: TileFlags) -> bool {
        if let Some(t) = self.get_mut(x, y) {
            t.flags.insert(flags);
            true
        } else {
            false
        }
    }

    /// Clear flags on the tile at `(x, y)`. Returns `false` if out of bounds.
    #[inline]
    pub fn clear_flags(&mut self, x: u32, y: u32, flags: TileFlags) -> bool {
        if let Some(t) = self.get_mut(x, y) {
            t.flags.remove(flags);
            true
        } else {
            false
        }
    }

    /// Cardinal neighbours of `(x, y)`. Order: `[N, E, S, W]`.
    #[inline]
    pub fn tile_neighbors(&self, x: u32, y: u32) -> [Option<(u32, u32)>; 4] {
        tile_neighbors_4(x, y, self.width, self.height)
    }

    /// Raw slice of all tiles in row-major order.
    #[inline]
    pub fn raw(&self) -> &[TileValue] {
        &self.tiles
    }

    /// Total number of tiles in this map.
    #[inline]
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Iterator yielding `(x, y, tile)` for every tile in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, TileValue)> + '_ {
        self.tiles.iter().enumerate().map(move |(i, &t)| {
            let (x, y) = index_to_coord(i, self.width);
            (x, y, t)
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    // ── Size assertions ──────────────────────────────────────────────────────

    #[test]
    fn tile_value_is_four_bytes() {
        assert_eq!(mem::size_of::<TileValue>(), 4);
    }

    #[test]
    fn tile_flags_is_one_byte() {
        assert_eq!(mem::size_of::<TileFlags>(), 1);
    }

    #[test]
    fn tile_kind_is_one_byte() {
        assert_eq!(mem::size_of::<TileKind>(), 1);
    }

    // ── tile_index (free-standing) ───────────────────────────────────────────

    #[test]
    fn tile_index_origin() {
        assert_eq!(tile_index(0, 0, 10), 0);
    }

    #[test]
    fn tile_index_row_major() {
        // (2, 3) in a 10-wide map => 3*10 + 2 = 32
        assert_eq!(tile_index(2, 3, 10), 32);
    }

    #[test]
    fn index_to_coord_round_trip() {
        let width = 15_u32;
        for y in 0..8_u32 {
            for x in 0..width {
                let idx = tile_index(x, y, width);
                assert_eq!(index_to_coord(idx, width), (x, y));
            }
        }
    }

    // ── tile_neighbors_4 ────────────────────────────────────────────────────

    #[test]
    fn neighbors_top_left_corner() {
        // (0, 0) on a 5x5 map: N and W are out of bounds
        let n = tile_neighbors_4(0, 0, 5, 5);
        assert_eq!(n[0], None);           // N
        assert_eq!(n[1], Some((1, 0)));   // E
        assert_eq!(n[2], Some((0, 1)));   // S
        assert_eq!(n[3], None);           // W
    }

    #[test]
    fn neighbors_bottom_right_corner() {
        // (4, 4) on a 5x5 map: E and S are out of bounds
        let n = tile_neighbors_4(4, 4, 5, 5);
        assert_eq!(n[0], Some((4, 3)));   // N
        assert_eq!(n[1], None);           // E
        assert_eq!(n[2], None);           // S
        assert_eq!(n[3], Some((3, 4)));   // W
    }

    #[test]
    fn neighbors_center() {
        let n = tile_neighbors_4(2, 2, 5, 5);
        assert_eq!(n[0], Some((2, 1)));   // N
        assert_eq!(n[1], Some((3, 2)));   // E
        assert_eq!(n[2], Some((2, 3)));   // S
        assert_eq!(n[3], Some((1, 2)));   // W
    }

    // ── TileFlags operations ─────────────────────────────────────────────────

    #[test]
    fn tile_flags_default_is_none() {
        assert!(TileFlags::default().is_empty());
    }

    #[test]
    fn tile_flags_insert_and_contains() {
        let mut f = TileFlags::NONE;
        f.insert(TileFlags::POWERED);
        f.insert(TileFlags::WATERED);
        assert!(f.contains(TileFlags::POWERED));
        assert!(f.contains(TileFlags::WATERED));
        assert!(!f.contains(TileFlags::ROAD_ACCESS));
    }

    #[test]
    fn tile_flags_remove() {
        let mut f = TileFlags::POWERED | TileFlags::WATERED;
        f.remove(TileFlags::POWERED);
        assert!(!f.contains(TileFlags::POWERED));
        assert!(f.contains(TileFlags::WATERED));
    }

    #[test]
    fn tile_flags_bitor() {
        let f = TileFlags::POWERED | TileFlags::BULLDOZABLE;
        assert!(f.contains(TileFlags::POWERED));
        assert!(f.contains(TileFlags::BULLDOZABLE));
        assert!(!f.contains(TileFlags::ON_FIRE));
    }

    #[test]
    fn tile_flags_bitand() {
        let a = TileFlags::POWERED | TileFlags::WATERED | TileFlags::ON_FIRE;
        let b = TileFlags::POWERED | TileFlags::BULLDOZABLE;
        let c = a & b;
        assert!(c.contains(TileFlags::POWERED));
        assert!(!c.contains(TileFlags::WATERED));
        assert!(!c.contains(TileFlags::BULLDOZABLE));
    }

    #[test]
    fn tile_flags_constants_no_overlap() {
        let all = TileFlags::POWERED
            | TileFlags::WATERED
            | TileFlags::ROAD_ACCESS
            | TileFlags::UNDER_CONSTRUCTION
            | TileFlags::BULLDOZABLE
            | TileFlags::ON_FIRE;
        // Each constant must occupy a unique bit.
        assert_eq!(all.0.count_ones(), 6);
    }

    // ── TileMap construction ─────────────────────────────────────────────────

    #[test]
    fn tilemap_new_dimensions() {
        let m = TileMap::new(10, 20);
        assert_eq!(m.width(), 10);
        assert_eq!(m.height(), 20);
    }

    #[test]
    fn tilemap_new_fills_default() {
        let m = TileMap::new(4, 4);
        for (_, _, t) in m.iter() {
            assert_eq!(t, TileValue::DEFAULT);
        }
    }

    #[test]
    fn tilemap_in_bounds() {
        let m = TileMap::new(5, 5);
        assert!(m.in_bounds(0, 0));
        assert!(m.in_bounds(4, 4));
        assert!(!m.in_bounds(5, 0));
        assert!(!m.in_bounds(0, 5));
    }

    #[test]
    fn tilemap_set_and_get() {
        let mut m = TileMap::new(8, 8);
        let mut v = TileValue::DEFAULT;
        v.kind = TileKind::Road;
        v.flags.insert(TileFlags::ROAD_ACCESS);
        assert!(m.set(3, 4, v));
        let got = m.get(3, 4).expect("in bounds");
        assert_eq!(got.kind, TileKind::Road);
        assert!(got.flags.contains(TileFlags::ROAD_ACCESS));
    }

    #[test]
    fn tilemap_set_out_of_bounds_returns_false() {
        let mut m = TileMap::new(4, 4);
        assert!(!m.set(4, 0, TileValue::DEFAULT));
        assert!(!m.set(0, 4, TileValue::DEFAULT));
    }

    #[test]
    fn tilemap_set_flags_and_clear_flags() {
        let mut m = TileMap::new(4, 4);
        assert!(m.set_flags(1, 1, TileFlags::POWERED));
        assert!(m.get(1, 1).unwrap().flags.contains(TileFlags::POWERED));
        assert!(m.clear_flags(1, 1, TileFlags::POWERED));
        assert!(!m.get(1, 1).unwrap().flags.contains(TileFlags::POWERED));
    }

    #[test]
    fn tilemap_tile_index_matches_free_fn() {
        let m = TileMap::new(12, 8);
        for y in 0..8_u32 {
            for x in 0..12_u32 {
                assert_eq!(
                    m.tile_index(x, y),
                    Some(tile_index(x, y, 12))
                );
            }
        }
    }

    #[test]
    fn tilemap_index_to_coord_round_trip() {
        let m = TileMap::new(7, 9);
        for i in 0..(7 * 9) {
            let (x, y) = m.index_to_coord(i).expect("valid index");
            assert_eq!(m.tile_index(x, y), Some(i));
        }
    }

    #[test]
    fn tilemap_neighbors_delegate_to_free_fn() {
        let m = TileMap::new(6, 6);
        let expected = tile_neighbors_4(3, 3, 6, 6);
        assert_eq!(m.tile_neighbors(3, 3), expected);
    }

    #[test]
    fn tilemap_iter_covers_all_tiles() {
        let m = TileMap::new(5, 3);
        let count = m.iter().count();
        assert_eq!(count, 5 * 3);
    }

    #[test]
    fn tilemap_raw_length() {
        let m = TileMap::new(10, 10);
        assert_eq!(m.raw().len(), 100);
    }
}
