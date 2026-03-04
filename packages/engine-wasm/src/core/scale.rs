//! Scale and unit normalization for the simulation engine.
//!
//! Defines the canonical world scale: tile size in meters, floor-area mapping,
//! population density derivation, vehicle speed conversions, and budget time base.
//!
//! This module supplements math::time (which defines time units).
//! Together they form the complete unit system.

use crate::core_types::*;

// ─── Floor-Area Formulas ─────────────────────────────────────────────────────

/// Compute gross floor area in square meters.
///
/// `footprint_w` and `footprint_h` in tiles, `coverage_ratio_pct` 0-100,
/// `floors` is number of stories.
#[inline]
pub const fn gross_floor_area_m2(
    footprint_w: u8,
    footprint_h: u8,
    coverage_ratio_pct: u8,
    floors: u8,
) -> u32 {
    let footprint = footprint_w as u32 * footprint_h as u32 * SIM_TILE_AREA_M2;
    footprint * coverage_ratio_pct as u32 * floors as u32 / 100
}

/// Compute net usable area (subtracts walls, stairs, corridors).
///
/// `usable_ratio_pct` 0-100, typically 65-85.
#[inline]
pub const fn net_usable_area_m2(gross_area_m2: u32, usable_ratio_pct: u8) -> u32 {
    gross_area_m2 * usable_ratio_pct as u32 / 100
}

// ─── Population / Employment Density ─────────────────────────────────────────

/// Derive residential population capacity from gross floor area.
///
/// `living_space_per_person_m2` comes from country presets (e.g., 40 for generic).
#[inline]
pub const fn residents_from_floor_area(
    gross_area_m2: u32,
    living_space_per_person_m2: u32,
) -> u32 {
    if living_space_per_person_m2 == 0 {
        return 0;
    }
    gross_area_m2 / living_space_per_person_m2
}

/// Derive job capacity from gross floor area.
///
/// `workspace_per_job_m2` depends on zone type (14 for office, 25 retail, etc.).
#[inline]
pub const fn jobs_from_floor_area(gross_area_m2: u32, workspace_per_job_m2: u32) -> u32 {
    if workspace_per_job_m2 == 0 {
        return 0;
    }
    gross_area_m2 / workspace_per_job_m2
}

// ─── Vehicle Speed Conversion ────────────────────────────────────────────────

/// Convert speed in km/h to tiles-per-tick as a fixed-point Q16.16 value.
///
/// Formula: tiles_per_tick = speed_km_h * 1000 / 3600 / SIM_TILE_M * GAME_SECONDS_PER_TICK
/// Simplified: tiles_per_tick = speed_km_h * 1000 * 12 / (3600 * 16 * 10)
///           = speed_km_h * 12000 / 576000
///           = speed_km_h / 48
///
/// Returns Q16.16 fixed-point value.
#[inline]
pub const fn speed_kmh_to_tiles_per_tick_q16(speed_km_h: u32) -> i32 {
    // We want: speed_km_h * 65536 / 48
    // = speed_km_h * 1365 (approximately, but let's be precise)
    // Exact: speed_km_h * 65536 / 48 = speed_km_h * 4096 / 3
    (speed_km_h as i64 * 65536 / 48) as i32
}

/// Convert speed in km/h to tiles-per-second (integer, truncated).
#[inline]
pub const fn speed_kmh_to_tiles_per_second_x1000(speed_km_h: u32) -> u32 {
    // tiles/sec = km_h * 1000 / 3600 / 16 = km_h * 1000 / 57600
    speed_km_h * 1000000 / 57600
}

// ─── Pathfinding Weight Conversion ───────────────────────────────────────────

/// Compute travel-time edge weight in 1/256 tile-cost fixed-point units.
///
/// `length_tiles_q16` is the segment length in Q16.16 fixed point.
/// `speed_tiles_per_tick_q16` is from speed_kmh_to_tiles_per_tick_q16.
/// Returns: floor(length / speed * 256).
#[inline]
pub const fn travel_time_weight(length_tiles_q16: i32, speed_tiles_per_tick_q16: i32) -> u32 {
    if speed_tiles_per_tick_q16 == 0 {
        return u32::MAX;
    }
    let time_q16 = (length_tiles_q16 as i64 * 65536 / speed_tiles_per_tick_q16 as i64) as i32;
    // Multiply by 256/65536 = 1/256 to convert to 1/256 tile-cost units
    // Actually: weight = (length / speed) * 256
    // In Q16.16: (length_q16 / speed_q16) gives result in Q16.16
    // Then * 256 and >> 16 to get integer 1/256 units
    ((time_q16 as i64 * 256) >> 16) as u32
}

// ─── Sub-Tile Coordinates ────────────────────────────────────────────────────

/// Convert continuous meter coordinates to tile coordinates (truncated).
#[inline]
pub const fn meters_to_tile(meters_x: i32, meters_y: i32) -> (i16, i16) {
    (
        (meters_x / SIM_TILE_M as i32) as i16,
        (meters_y / SIM_TILE_M as i32) as i16,
    )
}

/// Get the sub-tile offset in meters within a tile.
#[inline]
pub const fn sub_tile_offset(meters_x: i32, meters_y: i32) -> (i32, i32) {
    (
        meters_x % SIM_TILE_M as i32,
        meters_y % SIM_TILE_M as i32,
    )
}

/// Convert tile coordinates + offset back to meters.
#[inline]
pub const fn tile_to_meters(tile_x: i16, tile_y: i16, offset_x: i32, offset_y: i32) -> (i32, i32) {
    (
        tile_x as i32 * SIM_TILE_M as i32 + offset_x,
        tile_y as i32 * SIM_TILE_M as i32 + offset_y,
    )
}

// ─── Render Tile Resolution ─────────────────────────────────────────────────

/// Convert tile units (0..1024) to pixels at a given quality tier.
#[inline]
pub const fn tile_units_to_px_x(tile_units: u32, tile_w_px: u32) -> u32 {
    tile_units * tile_w_px / TILE_UNITS_PER_TILE
}

/// Convert tile units (0..1024) to pixels at a given quality tier.
#[inline]
pub const fn tile_units_to_px_y(tile_units: u32, tile_h_px: u32) -> u32 {
    tile_units * tile_h_px / TILE_UNITS_PER_TILE
}

/// Map area in square kilometers (integer * 100 for 2 decimal places).
#[inline]
pub const fn map_area_km2_x100(map_size: MapSize) -> u32 {
    let area_m2 = map_size.area() * SIM_TILE_AREA_M2;
    // km2 = area_m2 / 1_000_000, but we want * 100 for display
    // = area_m2 / 10_000
    area_m2 / 10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_area_small_house() {
        // 1x1, 50% coverage, 2 floors = 256 * 0.5 * 2 = 256 m²
        assert_eq!(gross_floor_area_m2(1, 1, 50, 2), 256);
    }

    #[test]
    fn floor_area_apartment_block() {
        // 2x2, 70% coverage, 6 floors = 1024 * 0.7 * 6 = 4300.8 -> 4300 (truncated)
        let area = gross_floor_area_m2(2, 2, 70, 6);
        assert_eq!(area, 4300); // 1024 * 70 * 6 / 100 = 430080/100 = 4300
    }

    #[test]
    fn floor_area_office_tower() {
        // 2x2, 60% coverage, 20 floors = 1024 * 0.6 * 20 = 12288
        assert_eq!(gross_floor_area_m2(2, 2, 60, 20), 12288);
    }

    #[test]
    fn floor_area_hospital() {
        // 5x5, 60% coverage, 4 floors = 6400 * 0.6 * 4 = 15360
        assert_eq!(gross_floor_area_m2(5, 5, 60, 4), 15360);
    }

    #[test]
    fn population_generic_preset() {
        // 4300 m² / 40 m²/person = 107 residents
        assert_eq!(residents_from_floor_area(4300, 40), 107);
    }

    #[test]
    fn population_us_preset() {
        // Same building, US preset (68 m²/person) = 63 residents
        assert_eq!(residents_from_floor_area(4300, 68), 63);
    }

    #[test]
    fn jobs_office() {
        // 12288 m² / 14 m²/job = 877 jobs
        assert_eq!(jobs_from_floor_area(12288, 14), 877);
    }

    #[test]
    fn jobs_retail() {
        // 4300 m² / 25 m²/job = 172 jobs
        assert_eq!(jobs_from_floor_area(4300, 25), 172);
    }

    #[test]
    fn vehicle_speed_conversion() {
        // 50 km/h should give ~1.042 tiles/tick
        // In Q16.16: 1.042 * 65536 ≈ 68267
        let q = speed_kmh_to_tiles_per_tick_q16(50);
        // 50 * 65536 / 48 = 3276800 / 48 = 68266
        assert_eq!(q, 68266);
        // Convert to float for verification: 68266 / 65536 ≈ 1.0416
        let f = q as f32 / 65536.0;
        assert!((f - 1.042).abs() < 0.01);
    }

    #[test]
    fn vehicle_speed_highway() {
        // 100 km/h ≈ 2.083 tiles/tick
        let q = speed_kmh_to_tiles_per_tick_q16(100);
        let f = q as f32 / 65536.0;
        assert!((f - 2.083).abs() < 0.01);
    }

    #[test]
    fn vehicle_speed_local() {
        // 30 km/h ≈ 0.625 tiles/tick
        let q = speed_kmh_to_tiles_per_tick_q16(30);
        let f = q as f32 / 65536.0;
        assert!((f - 0.625).abs() < 0.01);
    }

    #[test]
    fn meters_to_tile_conversion() {
        assert_eq!(meters_to_tile(0, 0), (0, 0));
        assert_eq!(meters_to_tile(16, 16), (1, 1));
        assert_eq!(meters_to_tile(100, 200), (6, 12));
    }

    #[test]
    fn sub_tile_offset_conversion() {
        let (ox, oy) = sub_tile_offset(100, 200);
        assert_eq!(ox, 100 % 16); // 4
        assert_eq!(oy, 200 % 16); // 8
    }

    #[test]
    fn tile_to_meters_roundtrip() {
        let mx = 100;
        let my = 200;
        let (tx, ty) = meters_to_tile(mx, my);
        let (ox, oy) = sub_tile_offset(mx, my);
        let (rx, ry) = tile_to_meters(tx, ty, ox, oy);
        assert_eq!((rx, ry), (mx, my));
    }

    #[test]
    fn tile_units_to_px_high() {
        // 512 tile_units * 128px / 1024 = 64 px
        assert_eq!(tile_units_to_px_x(512, 128), 64);
    }

    #[test]
    fn tile_units_to_px_low() {
        // 512 tile_units * 64px / 1024 = 32 px
        assert_eq!(tile_units_to_px_x(512, 64), 32);
    }

    #[test]
    fn map_area_small() {
        // 128*128*256 = 4,194,304 m² = 4.19 km² -> 419 (x100)
        assert_eq!(map_area_km2_x100(MAP_SIZE_SMALL), 419);
    }

    #[test]
    fn map_area_large() {
        // 256*256*256 = 16,777,216 m² = 16.78 km² -> 1677 (x100)
        assert_eq!(map_area_km2_x100(MAP_SIZE_LARGE), 1677);
    }

    #[test]
    fn density_sanity_no_building_exceeds_400_per_tile() {
        // Worst case: 1x1, 90% coverage, 40 floors, 25 m²/person
        let area = gross_floor_area_m2(1, 1, 90, 40);
        let residents = residents_from_floor_area(area, 25);
        // 256 * 0.9 * 40 / 25 = 368
        assert!(residents <= 400, "residents={}", residents);
    }

    #[test]
    fn render_tile_independence() {
        // Changing TILE_W_PX should not affect sim formulas
        let area1 = gross_floor_area_m2(2, 2, 70, 6);
        // SIM_TILE_M is constant at 16, TILE_W_PX is separate
        assert_eq!(area1, 4300);
    }
}
