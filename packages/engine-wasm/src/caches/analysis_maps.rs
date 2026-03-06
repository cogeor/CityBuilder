//! Derived heatmaps and coverage maps for analysis overlays.
//!
//! Each map is a u16-per-tile grid. Desirability uses an offset encoding
//! (value + 32768) to store signed i16 values in unsigned storage.
//! Land value is a composite derived from desirability, pollution, crime, and noise.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::entity::EntityStore;
use crate::core_types::*;

/// Offset applied to desirability values so that i16 can be stored as u16.
/// A stored value of 32768 represents 0; 0 represents -32768; 65535 represents +32767.
pub const DESIRABILITY_OFFSET: u16 = 32768;

// ---- AnalysisMap ---------------------------------------------------------------

/// A generic u16-per-tile grid used for analysis overlays.
#[derive(Debug, Clone)]
pub struct AnalysisMap {
    data: Vec<u16>,
    width: u16,
    height: u16,
}

impl AnalysisMap {
    /// Create a new zeroed map of the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        AnalysisMap {
            data: vec![0u16; len],
            width,
            height,
        }
    }

    /// Get the value at (x, y). Returns 0 for out-of-bounds coordinates.
    #[inline]
    pub fn get(&self, x: i16, y: i16) -> u16 {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return 0;
        }
        let idx = y as usize * self.width as usize + x as usize;
        self.data[idx]
    }

    /// Set the value at (x, y). No-op for out-of-bounds coordinates.
    #[inline]
    pub fn set(&mut self, x: i16, y: i16, value: u16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        let idx = y as usize * self.width as usize + x as usize;
        self.data[idx] = value;
    }

    /// Add a value with saturation at u16::MAX. No-op for out-of-bounds coordinates.
    #[inline]
    pub fn add_saturating(&mut self, x: i16, y: i16, amount: u16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        let idx = y as usize * self.width as usize + x as usize;
        self.data[idx] = self.data[idx].saturating_add(amount);
    }

    /// Clear all values to zero.
    pub fn clear(&mut self) {
        for v in self.data.iter_mut() {
            *v = 0;
        }
    }

    /// Map width in tiles.
    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Map height in tiles.
    #[inline]
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Get a raw value by flat index. Returns 0 for out-of-bounds.
    #[inline]
    pub fn get_by_index(&self, idx: usize) -> u16 {
        if idx < self.data.len() {
            self.data[idx]
        } else {
            0
        }
    }

    /// Set a raw value by flat index. No-op for out-of-bounds.
    #[inline]
    pub fn set_by_index(&mut self, idx: usize, value: u16) {
        if idx < self.data.len() {
            self.data[idx] = value;
        }
    }

    /// Total number of tiles.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the map has no tiles.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// ---- Spread helper -------------------------------------------------------------

/// Add `value` to all tiles within manhattan distance <= `radius` of (center_x, center_y).
pub fn spread_value(map: &mut AnalysisMap, center_x: i16, center_y: i16, radius: u8, value: u16) {
    let r = radius as i16;
    for dy in -r..=r {
        let remaining = r - dy.abs();
        for dx in -remaining..=remaining {
            let tx = center_x + dx;
            let ty = center_y + dy;
            map.add_saturating(tx, ty, value);
        }
    }
}

/// Add a signed value to tiles within manhattan distance <= `radius`.
/// Uses the desirability offset encoding: stored = actual + DESIRABILITY_OFFSET.
fn spread_desirability(
    map: &mut AnalysisMap,
    center_x: i16,
    center_y: i16,
    radius: u8,
    magnitude: i16,
) {
    let r = radius as i16;
    for dy in -r..=r {
        let remaining = r - dy.abs();
        for dx in -remaining..=remaining {
            let tx = center_x + dx;
            let ty = center_y + dy;
            if tx < 0 || ty < 0 || tx >= map.width as i16 || ty >= map.height as i16 {
                continue;
            }
            let idx = ty as usize * map.width as usize + tx as usize;
            let current = map.data[idx] as i32;
            let new_val = (current + magnitude as i32).clamp(0, u16::MAX as i32);
            map.data[idx] = new_val as u16;
        }
    }
}

// ---- AnalysisMaps --------------------------------------------------------------

/// Container for all derived analysis maps.
#[derive(Debug, Clone)]
pub struct AnalysisMaps {
    pub pollution: AnalysisMap,
    pub crime: AnalysisMap,
    pub noise: AnalysisMap,
    /// Desirability stored with offset encoding: stored = actual + 32768.
    pub desirability: AnalysisMap,
    pub land_value: AnalysisMap,
}

impl AnalysisMaps {
    /// Create new zeroed analysis maps for the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let mut desirability = AnalysisMap::new(width, height);
        // Initialize desirability to the offset (representing 0)
        for i in 0..desirability.len() {
            desirability.set_by_index(i, DESIRABILITY_OFFSET);
        }

        AnalysisMaps {
            pollution: AnalysisMap::new(width, height),
            crime: AnalysisMap::new(width, height),
            noise: AnalysisMap::new(width, height),
            desirability,
            land_value: AnalysisMap::new(width, height),
        }
    }

    /// Rebuild all analysis maps from scratch.
    ///
    /// Clears all maps and iterates every active entity (not under construction)
    /// to accumulate pollution, noise, and desirability. Then computes land_value
    /// as a composite.
    pub fn rebuild_full(&mut self, entities: &EntityStore, registry: &ArchetypeRegistry) {
        // Clear all maps
        self.pollution.clear();
        self.crime.clear();
        self.noise.clear();
        // Reset desirability to offset baseline
        self.desirability.clear();
        for i in 0..self.desirability.len() {
            self.desirability.set_by_index(i, DESIRABILITY_OFFSET);
        }
        self.land_value.clear();

        // Accumulate from entities
        for handle in entities.iter_alive() {
            let flags = match entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
            // Skip entities under construction
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
                continue;
            }

            let pos = match entities.get_pos(handle) {
                Some(p) => p,
                None => continue,
            };
            let arch_id = match entities.get_archetype(handle) {
                Some(a) => a,
                None => continue,
            };
            let def = match registry.get(arch_id) {
                Some(d) => d,
                None => continue,
            };

            // Pollution: spread within pollution * 2 tile radius
            if def.pollution > 0 {
                let radius = def.pollution.saturating_mul(2);
                spread_value(
                    &mut self.pollution,
                    pos.x,
                    pos.y,
                    radius,
                    def.pollution as u16,
                );
            }

            // Noise: spread within noise tile radius
            if def.noise > 0 {
                spread_value(&mut self.noise, pos.x, pos.y, def.noise, def.noise as u16);
            }

            // Desirability: spread within desirability_radius
            if def.desirability_radius > 0 && def.desirability_magnitude != 0 {
                spread_desirability(
                    &mut self.desirability,
                    pos.x,
                    pos.y,
                    def.desirability_radius,
                    def.desirability_magnitude,
                );
            }
        }

        // Compute land_value composite
        self.compute_land_value();
    }

    /// Rebuild analysis maps for a partial scan window.
    ///
    /// Only clears and recomputes tiles in the range [start_idx, start_idx + count).
    /// Entities are still iterated fully, but only contributions landing in the
    /// window are applied.
    pub fn rebuild_partial(
        &mut self,
        entities: &EntityStore,
        registry: &ArchetypeRegistry,
        start_idx: usize,
        count: usize,
    ) {
        let w = self.pollution.width() as usize;
        let end_idx = (start_idx + count).min(self.pollution.len());

        // Clear only the window region
        for i in start_idx..end_idx {
            self.pollution.set_by_index(i, 0);
            self.crime.set_by_index(i, 0);
            self.noise.set_by_index(i, 0);
            self.desirability.set_by_index(i, DESIRABILITY_OFFSET);
            self.land_value.set_by_index(i, 0);
        }

        // Accumulate from entities, but only write into the window
        for handle in entities.iter_alive() {
            let flags = match entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
                continue;
            }

            let pos = match entities.get_pos(handle) {
                Some(p) => p,
                None => continue,
            };
            let arch_id = match entities.get_archetype(handle) {
                Some(a) => a,
                None => continue,
            };
            let def = match registry.get(arch_id) {
                Some(d) => d,
                None => continue,
            };

            // Pollution
            if def.pollution > 0 {
                let radius = def.pollution.saturating_mul(2);
                spread_value_partial(
                    &mut self.pollution,
                    pos.x,
                    pos.y,
                    radius,
                    def.pollution as u16,
                    w,
                    start_idx,
                    end_idx,
                );
            }

            // Noise
            if def.noise > 0 {
                spread_value_partial(
                    &mut self.noise,
                    pos.x,
                    pos.y,
                    def.noise,
                    def.noise as u16,
                    w,
                    start_idx,
                    end_idx,
                );
            }

            // Desirability
            if def.desirability_radius > 0 && def.desirability_magnitude != 0 {
                spread_desirability_partial(
                    &mut self.desirability,
                    pos.x,
                    pos.y,
                    def.desirability_radius,
                    def.desirability_magnitude,
                    w,
                    start_idx,
                    end_idx,
                );
            }
        }

        // Compute land_value for window only
        self.compute_land_value_partial(start_idx, end_idx);
    }

    /// Compute land_value = desirability * 2 + 100 - pollution - crime - noise, clamped [0, 65535].
    fn compute_land_value(&mut self) {
        for i in 0..self.land_value.len() {
            let desirability_raw = self.desirability.get_by_index(i) as i32 - DESIRABILITY_OFFSET as i32;
            let pollution = self.pollution.get_by_index(i) as i32;
            let crime = self.crime.get_by_index(i) as i32;
            let noise = self.noise.get_by_index(i) as i32;

            let lv = desirability_raw * 2 + 100 - pollution - crime - noise;
            self.land_value.set_by_index(i, lv.clamp(0, 65535) as u16);
        }
    }

    /// Compute land_value for a partial window only.
    fn compute_land_value_partial(&mut self, start_idx: usize, end_idx: usize) {
        for i in start_idx..end_idx {
            let desirability_raw = self.desirability.get_by_index(i) as i32 - DESIRABILITY_OFFSET as i32;
            let pollution = self.pollution.get_by_index(i) as i32;
            let crime = self.crime.get_by_index(i) as i32;
            let noise = self.noise.get_by_index(i) as i32;

            let lv = desirability_raw * 2 + 100 - pollution - crime - noise;
            self.land_value.set_by_index(i, lv.clamp(0, 65535) as u16);
        }
    }
}

/// Spread value but only write to tiles whose flat index falls in [start_idx, end_idx).
fn spread_value_partial(
    map: &mut AnalysisMap,
    center_x: i16,
    center_y: i16,
    radius: u8,
    value: u16,
    map_width: usize,
    start_idx: usize,
    end_idx: usize,
) {
    let r = radius as i16;
    let w = map.width() as i16;
    let h = map.height() as i16;
    for dy in -r..=r {
        let remaining = r - dy.abs();
        for dx in -remaining..=remaining {
            let tx = center_x + dx;
            let ty = center_y + dy;
            if tx < 0 || ty < 0 || tx >= w || ty >= h {
                continue;
            }
            let idx = ty as usize * map_width + tx as usize;
            if idx >= start_idx && idx < end_idx {
                map.data[idx] = map.data[idx].saturating_add(value);
            }
        }
    }
}

/// Spread desirability but only write to tiles whose flat index falls in [start_idx, end_idx).
fn spread_desirability_partial(
    map: &mut AnalysisMap,
    center_x: i16,
    center_y: i16,
    radius: u8,
    magnitude: i16,
    map_width: usize,
    start_idx: usize,
    end_idx: usize,
) {
    let r = radius as i16;
    let w = map.width() as i16;
    let h = map.height() as i16;
    for dy in -r..=r {
        let remaining = r - dy.abs();
        for dx in -remaining..=remaining {
            let tx = center_x + dx;
            let ty = center_y + dy;
            if tx < 0 || ty < 0 || tx >= w || ty >= h {
                continue;
            }
            let idx = ty as usize * map_width + tx as usize;
            if idx >= start_idx && idx < end_idx {
                let current = map.data[idx] as i32;
                let new_val = (current + magnitude as i32).clamp(0, u16::MAX as i32);
                map.data[idx] = new_val as u16;
            }
        }
    }
}

// ---- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeTag, Prerequisite};

    /// Helper: create a factory archetype with pollution=4, noise=3, negative desirability.
    fn make_factory() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 10,
            name: "Factory".to_string(),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 80,
            floors: 1,
            usable_ratio_pct: 90,
            base_cost_cents: 200_000,
            base_upkeep_cents_per_tick: 30,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 10,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 3,
            desirability_magnitude: -10,
            pollution: 4,
            noise: 3,
            build_time_ticks: 1000,
            max_level: 3,
            prerequisites: vec![Prerequisite::RoadAccess],
            workspace_per_job_m2: 30,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create a park archetype with positive desirability, no pollution/noise.
    fn make_park() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 20,
            name: "Park".to_string(),
            tags: vec![ArchetypeTag::Civic],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 10,
            floors: 1,
            usable_ratio_pct: 100,
            base_cost_cents: 50_000,
            base_upkeep_cents_per_tick: 5,
            power_demand_kw: 0,
            power_supply_kw: 0,
            water_demand: 1,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2,
            desirability_magnitude: 15,
            pollution: 0,
            noise: 0,
            build_time_ticks: 200,
            max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 0,
        }
    }

    // ---- AnalysisMap basic tests -----------------------------------------------

    #[test]
    fn analysis_map_new_is_zeroed() {
        let map = AnalysisMap::new(10, 10);
        for y in 0..10i16 {
            for x in 0..10i16 {
                assert_eq!(map.get(x, y), 0);
            }
        }
    }

    #[test]
    fn analysis_map_get_set_works() {
        let mut map = AnalysisMap::new(8, 8);
        map.set(3, 5, 42);
        assert_eq!(map.get(3, 5), 42);
        map.set(0, 0, 100);
        assert_eq!(map.get(0, 0), 100);
        map.set(7, 7, 999);
        assert_eq!(map.get(7, 7), 999);
    }

    #[test]
    fn analysis_map_add_saturating_works() {
        let mut map = AnalysisMap::new(4, 4);
        map.add_saturating(1, 1, 100);
        assert_eq!(map.get(1, 1), 100);
        map.add_saturating(1, 1, 200);
        assert_eq!(map.get(1, 1), 300);
        // Test saturation
        map.set(2, 2, u16::MAX - 10);
        map.add_saturating(2, 2, 100);
        assert_eq!(map.get(2, 2), u16::MAX);
    }

    #[test]
    fn analysis_map_out_of_bounds_returns_zero() {
        let map = AnalysisMap::new(4, 4);
        assert_eq!(map.get(-1, 0), 0);
        assert_eq!(map.get(0, -1), 0);
        assert_eq!(map.get(4, 0), 0);
        assert_eq!(map.get(0, 4), 0);
        assert_eq!(map.get(100, 100), 0);
        assert_eq!(map.get(-100, -100), 0);
    }

    #[test]
    fn analysis_map_out_of_bounds_set_is_noop() {
        let mut map = AnalysisMap::new(4, 4);
        map.set(-1, 0, 999);
        map.set(0, -1, 999);
        map.set(4, 0, 999);
        map.set(0, 4, 999);
        // Ensure no panic and internal data is unmodified
        for y in 0..4i16 {
            for x in 0..4i16 {
                assert_eq!(map.get(x, y), 0);
            }
        }
    }

    #[test]
    fn analysis_map_clear_resets_all() {
        let mut map = AnalysisMap::new(4, 4);
        for y in 0..4i16 {
            for x in 0..4i16 {
                map.set(x, y, 42);
            }
        }
        map.clear();
        for y in 0..4i16 {
            for x in 0..4i16 {
                assert_eq!(map.get(x, y), 0);
            }
        }
    }

    #[test]
    fn analysis_map_width_height() {
        let map = AnalysisMap::new(16, 32);
        assert_eq!(map.width(), 16);
        assert_eq!(map.height(), 32);
        assert_eq!(map.len(), 16 * 32);
    }

    // ---- spread_value tests ----------------------------------------------------

    #[test]
    fn spread_value_covers_correct_tiles() {
        let mut map = AnalysisMap::new(10, 10);
        // Center at (5,5), radius 2
        spread_value(&mut map, 5, 5, 2, 10);

        // Manhattan distance 0 (center)
        assert_eq!(map.get(5, 5), 10);

        // Manhattan distance 1
        assert_eq!(map.get(4, 5), 10);
        assert_eq!(map.get(6, 5), 10);
        assert_eq!(map.get(5, 4), 10);
        assert_eq!(map.get(5, 6), 10);

        // Manhattan distance 2
        assert_eq!(map.get(3, 5), 10);
        assert_eq!(map.get(7, 5), 10);
        assert_eq!(map.get(5, 3), 10);
        assert_eq!(map.get(5, 7), 10);
        assert_eq!(map.get(4, 4), 10);
        assert_eq!(map.get(6, 6), 10);
        assert_eq!(map.get(4, 6), 10);
        assert_eq!(map.get(6, 4), 10);

        // Manhattan distance 3 (outside radius)
        assert_eq!(map.get(2, 5), 0);
        assert_eq!(map.get(8, 5), 0);
        assert_eq!(map.get(5, 2), 0);
        assert_eq!(map.get(5, 8), 0);
    }

    #[test]
    fn spread_value_radius_zero_only_center() {
        let mut map = AnalysisMap::new(5, 5);
        spread_value(&mut map, 2, 2, 0, 50);
        assert_eq!(map.get(2, 2), 50);
        assert_eq!(map.get(1, 2), 0);
        assert_eq!(map.get(3, 2), 0);
        assert_eq!(map.get(2, 1), 0);
        assert_eq!(map.get(2, 3), 0);
    }

    #[test]
    fn spread_value_clips_to_map_edges() {
        let mut map = AnalysisMap::new(4, 4);
        // Center at (0,0) with radius 2 -- should not panic
        spread_value(&mut map, 0, 0, 2, 5);
        assert_eq!(map.get(0, 0), 5);
        assert_eq!(map.get(1, 0), 5);
        assert_eq!(map.get(0, 1), 5);
        assert_eq!(map.get(2, 0), 5);
        assert_eq!(map.get(0, 2), 5);
        assert_eq!(map.get(1, 1), 5);
        // Out of bounds tiles should be 0
        assert_eq!(map.get(-1, 0), 0);
        assert_eq!(map.get(0, -1), 0);
    }

    // ---- rebuild_full tests ----------------------------------------------------

    #[test]
    fn rebuild_full_populates_pollution_from_entities() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        // Place a factory at (5, 5) -- mark as NOT under construction
        let h = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);

        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);

        // Factory has pollution=4, radius=4*2=8
        // Center should have pollution value
        assert!(maps.pollution.get(5, 5) > 0);
        assert_eq!(maps.pollution.get(5, 5), 4); // pollution value
    }

    #[test]
    fn rebuild_full_populates_noise() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        let h = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);

        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);

        // Factory has noise=3, radius=3
        assert!(maps.noise.get(5, 5) > 0);
        assert_eq!(maps.noise.get(5, 5), 3); // noise value at center
        // Within radius 3
        assert!(maps.noise.get(8, 5) > 0);
        // Outside radius 3
        assert_eq!(maps.noise.get(9, 5), 0);
    }

    #[test]
    fn rebuild_full_computes_land_value() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_park());

        // Place a park at (5, 5)
        let h = entities.alloc(20, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);

        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);

        // At center: desirability = +15, pollution = 0, crime = 0, noise = 0
        // land_value = 15 * 2 + 100 - 0 - 0 - 0 = 130
        assert_eq!(maps.land_value.get(5, 5), 130);

        // Far away tile with no influence: desirability = 0
        // land_value = 0 * 2 + 100 - 0 - 0 - 0 = 100
        assert_eq!(maps.land_value.get(15, 15), 100);
    }

    #[test]
    fn desirability_can_be_negative_stored_offset() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        let h = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);

        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);

        // Factory has desirability_magnitude = -10
        // Stored value at center = DESIRABILITY_OFFSET + (-10) = 32758
        let stored = maps.desirability.get(5, 5);
        let actual = stored as i32 - DESIRABILITY_OFFSET as i32;
        assert_eq!(actual, -10);
    }

    #[test]
    fn empty_world_all_zeros() {
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();

        let mut maps = AnalysisMaps::new(8, 8);
        maps.rebuild_full(&entities, &registry);

        for y in 0..8i16 {
            for x in 0..8i16 {
                assert_eq!(maps.pollution.get(x, y), 0);
                assert_eq!(maps.crime.get(x, y), 0);
                assert_eq!(maps.noise.get(x, y), 0);
                // Desirability should be at offset (meaning 0 actual)
                assert_eq!(maps.desirability.get(x, y), DESIRABILITY_OFFSET);
                // Land value: 0*2 + 100 - 0 - 0 - 0 = 100
                assert_eq!(maps.land_value.get(x, y), 100);
            }
        }
    }

    #[test]
    fn rebuild_full_skips_under_construction() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        // Entity is under construction (default alloc flag)
        let _h = entities.alloc(10, 5, 5, 0).unwrap();
        // Do NOT clear the UNDER_CONSTRUCTION flag

        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);

        // Should not have any pollution since entity is under construction
        assert_eq!(maps.pollution.get(5, 5), 0);
        assert_eq!(maps.noise.get(5, 5), 0);
    }

    #[test]
    fn rebuild_partial_only_affects_window() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        let h = entities.alloc(10, 1, 1, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);

        let mut maps = AnalysisMaps::new(8, 8);
        // Do a full rebuild first to populate everything
        maps.rebuild_full(&entities, &registry);

        let pollution_at_1_1_before = maps.pollution.get(1, 1);
        assert!(pollution_at_1_1_before > 0);

        // Now do a partial rebuild on a window that does NOT include tile (1,1)
        // Tile (1,1) is at flat index 1*8+1 = 9
        // Use a window that starts after index 9
        let start_idx = 16; // row 2
        let count = 16; // rows 2-3

        // First manually set a value in the window to confirm it gets cleared
        maps.pollution.set_by_index(start_idx, 999);

        maps.rebuild_partial(&entities, &registry, start_idx, count);

        // Tiles outside the window should be unchanged (row 0-1 untouched)
        assert_eq!(maps.pollution.get(1, 1), pollution_at_1_1_before);

        // Tiles inside the window should have been rebuilt
        // The factory at (1,1) with pollution radius 8 should affect tiles in rows 2-3
        // that are within manhattan distance 8 of (1,1)
        let val_at_start = maps.pollution.get_by_index(start_idx);
        // start_idx=16 is tile (0,2), manhattan dist from (1,1) = 1+1=2, within radius 8
        assert!(val_at_start > 0);
    }

    #[test]
    fn rebuild_partial_window_beyond_map_is_safe() {
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();

        let mut maps = AnalysisMaps::new(4, 4); // 16 tiles total
        // Window extends beyond the map
        maps.rebuild_partial(&entities, &registry, 10, 100);
        // Should not panic, and tiles in range should be at baseline
        assert_eq!(maps.land_value.get_by_index(10), 100);
        assert_eq!(maps.land_value.get_by_index(15), 100);
    }

    #[test]
    fn multiple_entities_accumulate() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        // Place two factories at the same position
        let h1 = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h1, StatusFlags::POWERED);
        let h2 = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h2, StatusFlags::POWERED);

        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);

        // Pollution at center should be double
        assert_eq!(maps.pollution.get(5, 5), 8); // 4 + 4
        assert_eq!(maps.noise.get(5, 5), 6); // 3 + 3
    }

    #[test]
    fn analysis_maps_new_desirability_offset() {
        let maps = AnalysisMaps::new(4, 4);
        // All desirability tiles should be at the offset baseline
        for y in 0..4i16 {
            for x in 0..4i16 {
                assert_eq!(maps.desirability.get(x, y), DESIRABILITY_OFFSET);
            }
        }
    }

    #[test]
    fn land_value_negative_desirability_reduces_value() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        let h = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);

        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);

        // At center: desirability = -10, pollution = 4, noise = 3, crime = 0
        // land_value = (-10) * 2 + 100 - 4 - 0 - 3 = -20 + 100 - 7 = 73
        assert_eq!(maps.land_value.get(5, 5), 73);
    }

    #[test]
    fn land_value_clamps_to_zero() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());

        // Place many factories at the same spot to drive land value negative
        for _ in 0..10 {
            let h = entities.alloc(10, 2, 2, 0).unwrap();
            entities.set_flags(h, StatusFlags::POWERED);
        }

        let mut maps = AnalysisMaps::new(8, 8);
        maps.rebuild_full(&entities, &registry);

        // With 10 factories: desirability = -100, pollution = 40, noise = 30
        // land_value = -200 + 100 - 40 - 0 - 30 = -170 -> clamped to 0
        assert_eq!(maps.land_value.get(2, 2), 0);
    }

    #[test]
    fn spread_value_count_tiles_radius_1() {
        // Manhattan diamond of radius 1: center + 4 neighbors = 5 tiles
        let mut map = AnalysisMap::new(10, 10);
        spread_value(&mut map, 5, 5, 1, 1);
        let mut count = 0;
        for y in 0..10i16 {
            for x in 0..10i16 {
                if map.get(x, y) > 0 {
                    count += 1;
                }
            }
        }
        assert_eq!(count, 5);
    }

    #[test]
    fn spread_value_count_tiles_radius_2() {
        // Manhattan diamond of radius 2: 1 + 4 + 8 = 13 tiles
        let mut map = AnalysisMap::new(10, 10);
        spread_value(&mut map, 5, 5, 2, 1);
        let mut count = 0;
        for y in 0..10i16 {
            for x in 0..10i16 {
                if map.get(x, y) > 0 {
                    count += 1;
                }
            }
        }
        assert_eq!(count, 13);
    }
}
