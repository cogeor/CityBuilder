//! Per-tile suitability scoring for zone growth.
//!
//! `tile_suitability` mirrors the SimCity per-tile evaluation that gates
//! stochastic zone development: if `demand + suitability <= GROWTH_THRESHOLD`
//! the tile is skipped; otherwise growth can fire.

use crate::caches::analysis_maps::AnalysisMaps;
use crate::core::tilemap::{TileFlags, TileValue};
use crate::core_types::ZoneType;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Tiles with pollution above this level reject residential development.
pub const MAX_RESIDENTIAL_POLLUTION: u16 = 128;

/// Growth fires only when `demand + suitability > GROWTH_THRESHOLD`.
pub const GROWTH_THRESHOLD: i32 = -350;

/// Score bonus per adjacent tile with road access.
pub const ROAD_ADJACENCY_BONUS: i32 = 250;

// ─── SuitabilityScore ────────────────────────────────────────────────────────

/// Components of a per-tile suitability score.
#[derive(Debug, Clone, Copy, Default)]
pub struct SuitabilityScore {
    /// Primary weighted component (land value, distance, zone type base).
    pub base: i32,
    /// Pollution penalty (0 or negative).
    pub pollution_penalty: i32,
    /// Road adjacency and infrastructure bonus.
    pub accessibility_bonus: i32,
}

impl SuitabilityScore {
    /// Aggregated score clamped to `[-3000, +3000]`.
    pub fn total(&self) -> i32 {
        (self.base + self.pollution_penalty + self.accessibility_bonus)
            .clamp(-3000, 3000)
    }
}

// ─── tile_suitability ────────────────────────────────────────────────────────

/// Compute the suitability of a tile for zone growth.
///
/// Returns `None` if the tile is a hard-blocked site (high pollution for
/// residential, or no road access for commercial/industrial).
///
/// - `tile`: current tile state (flags, zone, terrain)
/// - `x`, `y`: tile coordinates
/// - `maps`: current analysis overlay maps
/// - `city_center`: tile coordinates of city centre (for distance bonus)
/// - `zone`: zone type being evaluated for growth
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
    let road_adj   = adjacent_road_count(tile); // 0-4

    match zone {
        ZoneType::Residential => {
            // Hard block: too much pollution.
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
            // Hard block: no road access.
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
            // Hard block: no road access.
            if !has_road {
                return None;
            }
            Some(SuitabilityScore {
                base: 0,
                pollution_penalty: 0,
                accessibility_bonus: road_adj * ROAD_ADJACENCY_BONUS,
            })
        }
        // Civic / Park / Transport: always eligible (demand gates separately).
        ZoneType::Civic | ZoneType::Park | ZoneType::Transport | ZoneType::None => {
            Some(SuitabilityScore {
                base: 0,
                pollution_penalty: 0,
                accessibility_bonus: 0,
            })
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Count of adjacent cardinal tiles with road access, 0–4.
///
/// The tile's own `ROAD_ACCESS` flag is a proxy; we use the tile itself
/// since we don't have neighbours here (the caller may extend with tilemap).
#[inline]
fn adjacent_road_count(tile: &TileValue) -> i32 {
    // Proxy: if the tile itself has road access, assume 1 adjacent road.
    // Full 4-neighbour check is done in the caller if tilemap is passed.
    if tile.flags.contains(TileFlags::ROAD_ACCESS) { 1 } else { 0 }
}

/// Chebyshev distance (max of |dx|, |dy|).
#[inline]
fn chebyshev_dist(x: i16, y: i16, cx: i16, cy: i16) -> u32 {
    let dx = (x - cx).unsigned_abs() as u32;
    let dy = (y - cy).unsigned_abs() as u32;
    dx.max(dy)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caches::analysis_maps::AnalysisMaps;
    use crate::core::tilemap::{TileFlags, TileValue};
    use crate::core_types::{TerrainType, ZoneDensity, ZoneType};

    fn make_tile(flags: TileFlags, zone: ZoneType) -> TileValue {
        TileValue {
            terrain: TerrainType::Grass,
            kind: crate::core::tilemap::TileKind::Empty,
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
        let maps = make_maps(8, 8, 200, 100); // pollution > 128
        let tile = make_tile(TileFlags::ROAD_ACCESS, ZoneType::Residential);
        let result = tile_suitability(&tile, 0, 0, &maps, (0, 0), ZoneType::Residential);
        assert!(result.is_none(), "Expected None for high-pollution residential tile");
    }

    #[test]
    fn residential_allowed_with_low_pollution() {
        let maps = make_maps(8, 8, 10, 200); // low pollution, high land value
        let tile = make_tile(TileFlags::ROAD_ACCESS, ZoneType::Residential);
        let result = tile_suitability(&tile, 0, 0, &maps, (0, 0), ZoneType::Residential);
        assert!(result.is_some());
    }

    #[test]
    fn residential_score_increases_with_land_value() {
        let maps_low = make_maps(8, 8, 10, 50);
        let maps_high = make_maps(8, 8, 10, 500);
        let tile = make_tile(TileFlags::ROAD_ACCESS, ZoneType::Residential);
        let low = tile_suitability(&tile, 0, 0, &maps_low, (4, 4), ZoneType::Residential).unwrap();
        let high = tile_suitability(&tile, 0, 0, &maps_high, (4, 4), ZoneType::Residential).unwrap();
        assert!(
            high.total() > low.total(),
            "Higher land value should produce higher suitability: low={}, high={}",
            low.total(), high.total()
        );
    }

    #[test]
    fn commercial_blocked_without_road() {
        let maps = make_maps(8, 8, 0, 100);
        let tile = make_tile(TileFlags::NONE, ZoneType::Commercial);
        let result = tile_suitability(&tile, 2, 2, &maps, (0, 0), ZoneType::Commercial);
        assert!(result.is_none(), "Expected None for commercial tile with no road");
    }

    #[test]
    fn commercial_near_center_scores_higher() {
        let maps = make_maps(20, 20, 0, 100);
        let tile = make_tile(TileFlags::ROAD_ACCESS, ZoneType::Commercial);
        let center = (10i16, 10i16);
        let near = tile_suitability(&tile, 10, 10, &maps, center, ZoneType::Commercial).unwrap();
        let far  = tile_suitability(&tile, 0,  0,  &maps, center, ZoneType::Commercial).unwrap();
        assert!(
            near.total() > far.total(),
            "Near-center commercial should score higher: near={}, far={}",
            near.total(), far.total()
        );
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
