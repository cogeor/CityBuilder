//! Micropolis / SimCity tile-ID range constants and legacy-tile classifier.
//!
//! The numeric values below are transcribed directly from the Micropolis
//! open-source `sim.h` (Micropolis, Unix Version — Electronic Arts /
//! One Laptop Per Child, GPL v3).  Only the boundary constants required to
//! implement `kind_from_legacy` are included; animation-frame offsets, sprite
//! IDs, and overlay-map constants are omitted.
//!
//! # Status bits
//!
//! Each 16-bit tile word in the original SimCity format packs a 10-bit tile
//! index into the low bits and six status flags into the high bits:
//!
//! ```text
//! bit 15 — PWRBIT  (powered)
//! bit 14 — CONDBIT (conductive / conducting)
//! bit 13 — BURNBIT (burnable)
//! bit 12 — BULLBIT (bulldozable)
//! bit 11 — ANIMBIT (animated)
//! bit 10 — ZONEBIT (zone centre)
//! bits 9-0 — tile index (LOMASK)
//! ```
//!
//! `kind_from_legacy` always strips the upper 6 bits before classifying.

use crate::core::tilemap::TileKind;

// ─── Status-bit mask ─────────────────────────────────────────────────────────

/// Low-10-bit mask that strips the six SimCity status bits from a raw tile word.
///
/// Equivalent to `LOMASK` in Micropolis `sim.h`.
pub const LOMASK: u16 = 1023;

// ─── Tile-index boundary constants ───────────────────────────────────────────

/// Bare land (maps to `TileKind::Empty`).
pub const TILE_DIRT: u16 = 0;

/// First open-water tile (river / sea).
pub const TILE_RIVER: u16 = 2;

/// Last water-edge tile.
pub const TILE_LASTRIVEDGE: u16 = 20;

/// First tree tile (sparse).
pub const TILE_TREEBASE: u16 = 21;

/// Last sparse-tree tile.
pub const TILE_LASTTREE: u16 = 36;

/// Dense woods start.
pub const TILE_WOODS: u16 = 37;

/// Dense woods end.
pub const TILE_WOODS5: u16 = 43;

/// First rubble tile.
pub const TILE_RUBBLE: u16 = 44;

/// Last rubble tile.
pub const TILE_LASTRUBBLE: u16 = 47;

/// First flood tile.
pub const TILE_FLOOD: u16 = 48;

/// Last flood tile.
pub const TILE_LASTFLOOD: u16 = 51;

/// Radiation-contamination tile.
pub const TILE_RADTILE: u16 = 52;

/// First fire tile.
pub const TILE_FIREBASE: u16 = 56;

/// Last fire tile.
pub const TILE_LASTFIRE: u16 = 63;

/// First road tile (horizontal bridge).
pub const TILE_ROADBASE: u16 = 64;

/// Last road tile.
pub const TILE_LASTROAD: u16 = 206;

/// First power-line tile.
pub const TILE_POWERBASE: u16 = 208;

/// Last power-line tile.
pub const TILE_LASTPOWER: u16 = 222;

/// First rail tile.
pub const TILE_RAILBASE: u16 = 224;

/// Last rail tile.
pub const TILE_LASTRAIL: u16 = 238;

/// Residential zone base tile.
pub const TILE_RESBASE: u16 = 240;

/// Commercial zone base tile.
pub const TILE_COMBASE: u16 = 423;

/// Industrial zone base tile.
pub const TILE_INDBASE: u16 = 612;

/// Seaport base tile.
pub const TILE_PORTBASE: u16 = 693;

/// Last seaport tile.
pub const TILE_LASTPORT: u16 = 708;

/// Airport base tile.
pub const TILE_AIRPORTBASE: u16 = 709;

/// Coal power-plant base tile.
pub const TILE_COALBASE: u16 = 745;

/// Last coal power-plant tile.
pub const TILE_LASTPOWERPLANT: u16 = 760;

/// Nuclear power-plant base tile.
pub const TILE_NUCLEARBASE: u16 = 811;

/// Last zone tile (upper bound for all zone structures).
pub const TILE_LASTZONE: u16 = 826;

/// Total number of distinct tile indices in the Micropolis tile sheet.
pub const TILE_COUNT: u16 = 960;

// ─── Classifier ──────────────────────────────────────────────────────────────

/// Map a raw Micropolis / SimCity tile word to the engine's `TileKind`.
///
/// The upper six status bits (PWRBIT, CONDBIT, BURNBIT, BULLBIT, ANIMBIT,
/// ZONEBIT) are masked out with [`LOMASK`] before classification, so the same
/// result is returned regardless of whether a tile is powered, animated, etc.
///
/// # Mapping
///
/// | Tile-index range | `TileKind`             |
/// |------------------|------------------------|
/// | 0                | `Empty`                |
/// | 2–20             | `Water`                |
/// | 21–43            | `Nature`               |
/// | 44–47            | `Rubble`               |
/// | 48–51            | `Flood`                |
/// | 52               | `Radiation`            |
/// | 56–63            | `Fire`                 |
/// | 64–206           | `Road`                 |
/// | 208–222          | `PowerLine`            |
/// | 224–238          | `Rail`                 |
/// | 240–422          | `Building` (Res)       |
/// | 423–611          | `Building` (Com)       |
/// | 612–692          | `Building` (Ind)       |
/// | 693–708          | `Port`                 |
/// | 709–826          | `Special` (airports, power plants, stadiums, etc.) |
/// | _                | `Special` (fallback)   |
///
/// The 709–826 range is grouped as `Special` in this first pass to avoid
/// fine-grained sub-range enumeration.  Callers that need finer classification
/// (e.g. distinguishing airport tiles from coal-plant tiles) should inspect the
/// raw tile index directly against the exported `TILE_*` constants.
pub fn kind_from_legacy(tile_id: u16) -> TileKind {
    let id = tile_id & LOMASK;
    match id {
        0                               => TileKind::Empty,
        2..=20                          => TileKind::Water,
        21..=43                         => TileKind::Nature,
        44..=47                         => TileKind::Rubble,
        48..=51                         => TileKind::Flood,
        52                              => TileKind::Radiation,
        56..=63                         => TileKind::Fire,
        64..=206                        => TileKind::Road,
        208..=222                       => TileKind::PowerLine,
        224..=238                       => TileKind::Rail,
        240..=422                       => TileKind::Building,
        423..=611                       => TileKind::Building,
        612..=692                       => TileKind::Building,
        693..=708                       => TileKind::Port,
        709..=826                       => TileKind::Special,
        _                               => TileKind::Special,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Terrain / natural tiles ───────────────────────────────────────────────

    #[test]
    fn bare_land_is_empty() {
        assert_eq!(kind_from_legacy(0), TileKind::Empty);
    }

    #[test]
    fn water_range() {
        assert_eq!(kind_from_legacy(2),  TileKind::Water);
        assert_eq!(kind_from_legacy(20), TileKind::Water);
    }

    #[test]
    fn nature_range() {
        assert_eq!(kind_from_legacy(21), TileKind::Nature); // TREEBASE
        assert_eq!(kind_from_legacy(37), TileKind::Nature); // WOODS
        assert_eq!(kind_from_legacy(43), TileKind::Nature); // WOODS5
    }

    #[test]
    fn rubble_range() {
        assert_eq!(kind_from_legacy(44), TileKind::Rubble);
        assert_eq!(kind_from_legacy(47), TileKind::Rubble);
    }

    #[test]
    fn flood_range() {
        assert_eq!(kind_from_legacy(48), TileKind::Flood);
        assert_eq!(kind_from_legacy(51), TileKind::Flood);
    }

    #[test]
    fn radiation_tile() {
        assert_eq!(kind_from_legacy(52), TileKind::Radiation);
    }

    #[test]
    fn fire_range() {
        assert_eq!(kind_from_legacy(56), TileKind::Fire);
        assert_eq!(kind_from_legacy(63), TileKind::Fire);
    }

    // ── Infrastructure tiles ──────────────────────────────────────────────────

    #[test]
    fn road_range() {
        assert_eq!(kind_from_legacy(64),  TileKind::Road);
        assert_eq!(kind_from_legacy(206), TileKind::Road);
    }

    #[test]
    fn power_line_range() {
        assert_eq!(kind_from_legacy(208), TileKind::PowerLine);
        assert_eq!(kind_from_legacy(222), TileKind::PowerLine);
    }

    #[test]
    fn rail_range() {
        assert_eq!(kind_from_legacy(224), TileKind::Rail);
        assert_eq!(kind_from_legacy(238), TileKind::Rail);
    }

    // ── Zone / building tiles ─────────────────────────────────────────────────

    #[test]
    fn residential_building() {
        // HOUSE = 249, LHTHR = 249
        assert_eq!(kind_from_legacy(249), TileKind::Building);
    }

    #[test]
    fn commercial_building() {
        assert_eq!(kind_from_legacy(TILE_COMBASE),     TileKind::Building);
        assert_eq!(kind_from_legacy(TILE_COMBASE + 1), TileKind::Building);
    }

    #[test]
    fn industrial_building() {
        assert_eq!(kind_from_legacy(TILE_INDBASE),     TileKind::Building);
        assert_eq!(kind_from_legacy(TILE_INDBASE + 50), TileKind::Building);
    }

    #[test]
    fn seaport_range() {
        assert_eq!(kind_from_legacy(693), TileKind::Port);
        assert_eq!(kind_from_legacy(708), TileKind::Port);
    }

    // ── Special / out-of-range tiles ──────────────────────────────────────────

    #[test]
    fn airport_and_special_structures_are_special() {
        // AIRPORTBASE = 709 through LASTZONE = 826
        assert_eq!(kind_from_legacy(709), TileKind::Special);
        assert_eq!(kind_from_legacy(750), TileKind::Special); // POWERPLANT
        assert_eq!(kind_from_legacy(826), TileKind::Special); // LASTZONE
    }

    #[test]
    fn out_of_range_tile_is_special() {
        // 960 & LOMASK = 960 & 1023 = 960; no range covers it
        assert_eq!(kind_from_legacy(960), TileKind::Special);
    }

    // ── Status-bit masking ────────────────────────────────────────────────────

    #[test]
    fn status_bits_stripped_road() {
        // PWRBIT (0x8000) set on a road tile (64) must still return Road
        assert_eq!(kind_from_legacy(64 | 0x8000), TileKind::Road);
    }

    #[test]
    fn all_status_bits_stripped() {
        // ALLBITS = 0xFC00; road tile 64 with all status bits set
        let with_all_bits: u16 = 64 | 0xFC00;
        assert_eq!(kind_from_legacy(with_all_bits), TileKind::Road);
    }
}
