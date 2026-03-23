//! Typed render bridge — compile-time checked tile-to-pattern mapping.

use crate::tilemap::{TileKind, TileValue};
use crate::types::{TerrainType, ZoneType};

/// Pattern ID for the tile visual renderer.
///
/// Pattern ID layout (matches TileVisualRegistry):
///   0-4:   Terrain (Grass=0, Water=1, Sand=2, Forest=3, Rock=4)
///   7:     Road
///   8:     PowerLine
///   9:     Rail
///  10:     Port/Airport/Special
///  11-16:  Zones (Residential=11, Commercial=12, Industrial=13, Civic=14, Park=15, Transport=16)
///  20-26:  Buildings (None=20, Residential=21, Commercial=22, Industrial=23, Civic=24, Park=25, Transport=26)
///
/// Adding a TileKind or ZoneType variant without updating this function
/// will cause a compiler error.
pub fn pattern_id_for_tile(tile: &TileValue) -> u32 {
    match tile.kind {
        TileKind::Empty => terrain_pattern(tile.terrain),
        TileKind::Water => 1,
        TileKind::Nature => 3,
        TileKind::Rubble => 4,
        TileKind::Flood => 1,
        TileKind::Radiation => 5,
        TileKind::Fire => 6,
        TileKind::Road => 7,
        TileKind::PowerLine => 8,
        TileKind::Rail => 9,
        TileKind::Zone => zone_pattern(tile.zone),
        TileKind::Building => building_pattern(tile.zone),
        TileKind::Port => 10,
        TileKind::Airport => 10,
        TileKind::Special => 10,
    }
}

fn terrain_pattern(terrain: TerrainType) -> u32 {
    match terrain {
        TerrainType::Grass  => 0,
        TerrainType::Water  => 1,
        TerrainType::Sand   => 2,
        TerrainType::Forest => 3,
        TerrainType::Rock   => 4,
    }
}

fn zone_pattern(zone: ZoneType) -> u32 {
    match zone {
        ZoneType::None        => 0,
        ZoneType::Residential => 11,
        ZoneType::Commercial  => 12,
        ZoneType::Industrial  => 13,
        ZoneType::Civic       => 14,
        ZoneType::Park        => 15,
        ZoneType::Transport   => 16,
    }
}

fn building_pattern(zone: ZoneType) -> u32 {
    match zone {
        ZoneType::None        => 20,
        ZoneType::Residential => 21,
        ZoneType::Commercial  => 22,
        ZoneType::Industrial  => 23,
        ZoneType::Civic       => 24,
        ZoneType::Park        => 25,
        ZoneType::Transport   => 26,
    }
}
