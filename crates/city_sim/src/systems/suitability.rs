//! Per-tile suitability scoring for zone growth.
//!
//! Gates stochastic zone development: if `demand + suitability <= GROWTH_THRESHOLD`
//! the tile is skipped; otherwise growth can fire.

use crate::caches::analysis_maps::AnalysisMaps;
use crate::tilemap::{TileFlags, TileValue};
use crate::types::ZoneType;

/// Tiles with pollution above this level reject residential development.
pub const MAX_RESIDENTIAL_POLLUTION: u16 = 128;

/// Growth fires only when `demand + suitability > GROWTH_THRESHOLD`.
pub const GROWTH_THRESHOLD: i32 = -350;

/// Score bonus per adjacent tile with road access.
pub const ROAD_ADJACENCY_BONUS: i32 = 250;

/// Components of a per-tile suitability score.
#[derive(Debug, Clone, Copy, Default)]
pub struct SuitabilityScore {
    pub base: i32,
    pub pollution_penalty: i32,
    pub accessibility_bonus: i32,
}

impl SuitabilityScore {
    /// Aggregated score clamped to `[-3000, +3000]`.
    pub fn total(&self) -> i32 {
        (self.base + self.pollution_penalty + self.accessibility_bonus)
            .clamp(-3000, 3000)
    }
}

/// Compute the suitability of a tile for zone growth.
///
/// Returns `None` if the tile is a hard-blocked site.
pub fn tile_suitability(
    tile: &TileValue,
    x: i16,
    y: i16,
    maps: &AnalysisMaps,
    city_center: (i16, i16),
    zone: ZoneType,
) -> Option<SuitabilityScore> {
    let idx = (y as i32 * maps.pollution.width() as i32 + x as i32) as usize;
    let pollution  = maps.pollution.get_by_index(idx);
    let land_value = maps.land_value.get_by_index(idx) as i32;
    let has_road   = tile.flags.contains(TileFlags::ROAD_ACCESS);
    let road_adj   = adjacent_road_count(tile);

    match zone {
        ZoneType::Residential => {
            if pollution > MAX_RESIDENTIAL_POLLUTION {
                return None;
            }
            let base = (land_value - pollution as i32) * 32 - 3000;
            let poll_pen = -(pollution as i32 * 16);
            let acc_bonus = road_adj * ROAD_ADJACENCY_BONUS;
            Some(SuitabilityScore {
                base: base.clamp(-3000, 3000),
                pollution_penalty: poll_pen,
                accessibility_bonus: acc_bonus,
            })
        }
        ZoneType::Commercial => {
            if !has_road {
                return None;
            }
            let dist = chebyshev_dist(x, y, city_center.0, city_center.1) as i32;
            let base = (64 - 4 * dist).clamp(-3000, 3000);
            let acc_bonus = road_adj * ROAD_ADJACENCY_BONUS;
            Some(SuitabilityScore {
                base,
                pollution_penalty: 0,
                accessibility_bonus: acc_bonus,
            })
        }
        ZoneType::Industrial => {
            if !has_road {
                return None;
            }
            Some(SuitabilityScore {
                base: 0,
                pollution_penalty: 0,
                accessibility_bonus: road_adj * ROAD_ADJACENCY_BONUS,
            })
        }
        ZoneType::Civic | ZoneType::Park | ZoneType::Transport | ZoneType::None => {
            Some(SuitabilityScore {
                base: 0,
                pollution_penalty: 0,
                accessibility_bonus: 0,
            })
        }
    }
}

#[inline]
fn adjacent_road_count(tile: &TileValue) -> i32 {
    if tile.flags.contains(TileFlags::ROAD_ACCESS) { 1 } else { 0 }
}

#[inline]
fn chebyshev_dist(x: i16, y: i16, cx: i16, cy: i16) -> u32 {
    let dx = (x - cx).unsigned_abs() as u32;
    let dy = (y - cy).unsigned_abs() as u32;
    dx.max(dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tilemap::{TileFlags, TileKind, TileValue};
    use crate::types::{TerrainType, ZoneDensity, ZoneType};

    fn make_tile(flags: TileFlags, zone: ZoneType) -> TileValue {
        TileValue {
            terrain: TerrainType::Grass,
            kind: TileKind::Empty,
            zone,
            density: ZoneDensity::Low,
            flags,
        }
    }

    fn make_maps(w: u16, h: u16, pollution: u16, lv: u16) -> AnalysisMaps {
        let mut maps = AnalysisMaps::new(w, h);
        for i in 0..(w as usize * h as usize) {
            maps.pollution.set_by_index(i, pollution);
            maps.land_value.set_by_index(i, lv);
        }
        maps
    }

    #[test]
    fn residential_blocked_by_high_pollution() {
        let maps = make_maps(8, 8, 200, 100);
        let tile = make_tile(TileFlags::ROAD_ACCESS, ZoneType::Residential);
        let result = tile_suitability(&tile, 0, 0, &maps, (0, 0), ZoneType::Residential);
        assert!(result.is_none());
    }

    #[test]
    fn residential_allowed_with_low_pollution() {
        let maps = make_maps(8, 8, 10, 200);
        let tile = make_tile(TileFlags::ROAD_ACCESS, ZoneType::Residential);
        let result = tile_suitability(&tile, 0, 0, &maps, (0, 0), ZoneType::Residential);
        assert!(result.is_some());
    }

    #[test]
    fn commercial_blocked_without_road() {
        let maps = make_maps(8, 8, 0, 100);
        let tile = make_tile(TileFlags::NONE, ZoneType::Commercial);
        let result = tile_suitability(&tile, 2, 2, &maps, (0, 0), ZoneType::Commercial);
        assert!(result.is_none());
    }

    #[test]
    fn industrial_blocked_without_road() {
        let maps = make_maps(8, 8, 0, 50);
        let tile = make_tile(TileFlags::NONE, ZoneType::Industrial);
        let result = tile_suitability(&tile, 0, 0, &maps, (0, 0), ZoneType::Industrial);
        assert!(result.is_none());
    }

    #[test]
    fn growth_threshold_constant() {
        assert_eq!(GROWTH_THRESHOLD, -350);
    }
}
