use city_core::StatusFlags;
use crate::archetype::ArchetypeRegistry;
use city_engine::entity::EntityStore;

use super::dirty_tiles::DirtyTileSet;

/// Offset applied to desirability values so that i16 can be stored as u16.
pub const DESIRABILITY_OFFSET: u16 = 32768;

/// A generic u16-per-tile grid used for analysis overlays.
#[derive(Debug, Clone)]
pub struct AnalysisMap {
    pub(crate) data: Vec<u16>,
    width: u16,
    height: u16,
}

impl AnalysisMap {
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        AnalysisMap {
            data: vec![0u16; len],
            width,
            height,
        }
    }

    #[inline]
    pub fn get(&self, x: i16, y: i16) -> u16 {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return 0;
        }
        let idx = y as usize * self.width as usize + x as usize;
        self.data[idx]
    }

    #[inline]
    pub fn set(&mut self, x: i16, y: i16, value: u16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        let idx = y as usize * self.width as usize + x as usize;
        self.data[idx] = value;
    }

    #[inline]
    pub fn add_saturating(&mut self, x: i16, y: i16, amount: u16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        let idx = y as usize * self.width as usize + x as usize;
        self.data[idx] = self.data[idx].saturating_add(amount);
    }

    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    #[inline]
    pub fn fill(&mut self, value: u16) {
        self.data.fill(value);
    }

    #[inline]
    pub fn width(&self) -> u16 { self.width }

    #[inline]
    pub fn height(&self) -> u16 { self.height }

    #[inline]
    pub fn get_by_index(&self, idx: usize) -> u16 {
        if idx < self.data.len() { self.data[idx] } else { 0 }
    }

    #[inline]
    pub fn set_by_index(&mut self, idx: usize, value: u16) {
        if idx < self.data.len() { self.data[idx] = value; }
    }

    #[inline]
    pub fn len(&self) -> usize { self.data.len() }

    #[inline]
    pub fn is_empty(&self) -> bool { self.data.is_empty() }
}

/// Add `value` to all tiles within manhattan distance <= `radius` of (center_x, center_y).
pub fn spread_value(map: &mut AnalysisMap, center_x: i16, center_y: i16, radius: u8, value: u16) {
    let r = radius as i16;
    for dy in -r..=r {
        let remaining = r - dy.abs();
        for dx in -remaining..=remaining {
            map.add_saturating(center_x + dx, center_y + dy, value);
        }
    }
}

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

fn spread_value_dirty(
    map: &mut AnalysisMap,
    cx: i16, cy: i16, radius: u8, value: u16,
    map_width: usize, dirty: &DirtyTileSet,
) {
    let r = radius as i16;
    let w = map.width() as i16;
    let h = map.height() as i16;
    for dy in -r..=r {
        let rem = r - dy.abs();
        for dx in -rem..=rem {
            let tx = cx + dx;
            let ty = cy + dy;
            if tx < 0 || ty < 0 || tx >= w || ty >= h { continue; }
            if !dirty.is_set(tx, ty) { continue; }
            let idx = ty as usize * map_width + tx as usize;
            map.data[idx] = map.data[idx].saturating_add(value);
        }
    }
}

fn spread_desirability_dirty(
    map: &mut AnalysisMap,
    cx: i16, cy: i16, radius: u8, magnitude: i16,
    map_width: usize, dirty: &DirtyTileSet,
) {
    let r = radius as i16;
    let w = map.width() as i16;
    let h = map.height() as i16;
    for dy in -r..=r {
        let rem = r - dy.abs();
        for dx in -rem..=rem {
            let tx = cx + dx;
            let ty = cy + dy;
            if tx < 0 || ty < 0 || tx >= w || ty >= h { continue; }
            if !dirty.is_set(tx, ty) { continue; }
            let idx = ty as usize * map_width + tx as usize;
            let current = map.data[idx] as i32;
            let new_val = (current + magnitude as i32).clamp(0, u16::MAX as i32);
            map.data[idx] = new_val as u16;
        }
    }
}

/// Container for all derived analysis maps.
#[derive(Debug, Clone)]
pub struct AnalysisMaps {
    pub pollution: AnalysisMap,
    pub crime: AnalysisMap,
    pub noise: AnalysisMap,
    pub desirability: AnalysisMap,
    pub land_value: AnalysisMap,
}

impl AnalysisMaps {
    pub fn new(width: u16, height: u16) -> Self {
        let mut desirability = AnalysisMap::new(width, height);
        desirability.fill(DESIRABILITY_OFFSET);
        AnalysisMaps {
            pollution: AnalysisMap::new(width, height),
            crime: AnalysisMap::new(width, height),
            noise: AnalysisMap::new(width, height),
            desirability,
            land_value: AnalysisMap::new(width, height),
        }
    }

    pub fn rebuild_full(&mut self, entities: &EntityStore, registry: &ArchetypeRegistry) {
        self.pollution.clear();
        self.crime.clear();
        self.noise.clear();
        self.desirability.fill(DESIRABILITY_OFFSET);
        self.land_value.clear();

        for handle in entities.iter_alive() {
            let flags = match entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }

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

            if def.pollution > 0 {
                let radius = def.pollution.saturating_mul(2);
                spread_value(&mut self.pollution, pos.x, pos.y, radius, def.pollution as u16);
            }
            if def.noise > 0 {
                spread_value(&mut self.noise, pos.x, pos.y, def.noise, def.noise as u16);
            }
            if def.desirability_radius > 0 && def.desirability_magnitude != 0 {
                spread_desirability(
                    &mut self.desirability, pos.x, pos.y,
                    def.desirability_radius, def.desirability_magnitude,
                );
            }
        }

        self.compute_land_value();
    }

    pub fn rebuild_dirty(
        &mut self,
        entities: &EntityStore,
        registry: &ArchetypeRegistry,
        dirty: &DirtyTileSet,
    ) {
        if !dirty.any() { return; }

        let w = self.pollution.width() as usize;

        for idx in dirty.iter_dirty_indices() {
            self.pollution.set_by_index(idx, 0);
            self.crime.set_by_index(idx, 0);
            self.noise.set_by_index(idx, 0);
            self.desirability.set_by_index(idx, DESIRABILITY_OFFSET);
            self.land_value.set_by_index(idx, 0);
        }

        for handle in entities.iter_alive() {
            let flags = match entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
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

            if def.pollution > 0 {
                let radius = def.pollution.saturating_mul(2);
                spread_value_dirty(&mut self.pollution, pos.x, pos.y, radius, def.pollution as u16, w, dirty);
            }
            if def.noise > 0 {
                spread_value_dirty(&mut self.noise, pos.x, pos.y, def.noise, def.noise as u16, w, dirty);
            }
            if def.desirability_radius > 0 && def.desirability_magnitude != 0 {
                spread_desirability_dirty(
                    &mut self.desirability, pos.x, pos.y,
                    def.desirability_radius, def.desirability_magnitude, w, dirty,
                );
            }
        }

        for idx in dirty.iter_dirty_indices() {
            let desirability_raw = self.desirability.get_by_index(idx) as i32 - DESIRABILITY_OFFSET as i32;
            let pollution = self.pollution.get_by_index(idx) as i32;
            let crime = self.crime.get_by_index(idx) as i32;
            let noise = self.noise.get_by_index(idx) as i32;
            let lv = desirability_raw * 2 + 100 - pollution - crime - noise;
            self.land_value.set_by_index(idx, lv.clamp(0, 65535) as u16);
        }
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::{ArchetypeDefinition, ArchetypeTag, Prerequisite};

    fn make_factory() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 10,
            name: "Factory".to_string(),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 2, footprint_h: 2,
            coverage_ratio_pct: 80, floors: 1, usable_ratio_pct: 90,
            base_cost_cents: 200_000,
            base_upkeep_cents_per_tick: 30,
            power_demand_kw: 50, power_supply_kw: 0,
            water_demand: 10, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 3, desirability_magnitude: -10,
            pollution: 4, noise: 3,
            build_time_ticks: 1000, max_level: 3,
            prerequisites: vec![Prerequisite::RoadAccess],
            workspace_per_job_m2: 30, living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    fn make_park() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 20,
            name: "Park".to_string(),
            tags: vec![ArchetypeTag::Civic],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 10, floors: 1, usable_ratio_pct: 100,
            base_cost_cents: 50_000,
            base_upkeep_cents_per_tick: 5,
            power_demand_kw: 0, power_supply_kw: 0,
            water_demand: 1, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2, desirability_magnitude: 15,
            pollution: 0, noise: 0,
            build_time_ticks: 200, max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

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
    }

    #[test]
    fn analysis_map_add_saturating_works() {
        let mut map = AnalysisMap::new(4, 4);
        map.add_saturating(1, 1, 100);
        assert_eq!(map.get(1, 1), 100);
        map.add_saturating(1, 1, 200);
        assert_eq!(map.get(1, 1), 300);
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
    }

    #[test]
    fn analysis_map_clear_resets_all() {
        let mut map = AnalysisMap::new(4, 4);
        for y in 0..4i16 { for x in 0..4i16 { map.set(x, y, 42); } }
        map.clear();
        for y in 0..4i16 { for x in 0..4i16 { assert_eq!(map.get(x, y), 0); } }
    }

    #[test]
    fn analysis_map_width_height() {
        let map = AnalysisMap::new(16, 32);
        assert_eq!(map.width(), 16);
        assert_eq!(map.height(), 32);
        assert_eq!(map.len(), 16 * 32);
    }

    #[test]
    fn spread_value_covers_correct_tiles() {
        let mut map = AnalysisMap::new(10, 10);
        spread_value(&mut map, 5, 5, 2, 10);
        assert_eq!(map.get(5, 5), 10);
        assert_eq!(map.get(4, 5), 10);
        assert_eq!(map.get(3, 5), 10);
        assert_eq!(map.get(2, 5), 0);
    }

    #[test]
    fn spread_value_radius_zero_only_center() {
        let mut map = AnalysisMap::new(5, 5);
        spread_value(&mut map, 2, 2, 0, 50);
        assert_eq!(map.get(2, 2), 50);
        assert_eq!(map.get(1, 2), 0);
    }

    #[test]
    fn rebuild_full_populates_pollution() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());
        let h = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);
        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);
        assert_eq!(maps.pollution.get(5, 5), 4);
    }

    #[test]
    fn rebuild_full_computes_land_value() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_park());
        let h = entities.alloc(20, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);
        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);
        assert_eq!(maps.land_value.get(5, 5), 130);
        assert_eq!(maps.land_value.get(15, 15), 100);
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
                assert_eq!(maps.desirability.get(x, y), DESIRABILITY_OFFSET);
                assert_eq!(maps.land_value.get(x, y), 100);
            }
        }
    }

    #[test]
    fn rebuild_full_skips_under_construction() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());
        let _h = entities.alloc(10, 5, 5, 0).unwrap();
        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);
        assert_eq!(maps.pollution.get(5, 5), 0);
    }

    #[test]
    fn multiple_entities_accumulate() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());
        let h1 = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h1, StatusFlags::POWERED);
        let h2 = entities.alloc(10, 5, 5, 0).unwrap();
        entities.set_flags(h2, StatusFlags::POWERED);
        let mut maps = AnalysisMaps::new(16, 16);
        maps.rebuild_full(&entities, &registry);
        assert_eq!(maps.pollution.get(5, 5), 8);
    }

    #[test]
    fn rebuild_dirty_noop_when_no_dirty() {
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();
        let mut maps = AnalysisMaps::new(8, 8);
        maps.pollution.set(3, 3, 99);
        let dirty = DirtyTileSet::new(8, 8);
        maps.rebuild_dirty(&entities, &registry, &dirty);
        assert_eq!(maps.pollution.get(3, 3), 99);
    }

    #[test]
    fn rebuild_dirty_accumulates_entity_effect() {
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_factory());
        let h = entities.alloc(10, 4, 4, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);
        let mut maps = AnalysisMaps::new(16, 16);
        let mut dirty = DirtyTileSet::new(16, 16);
        dirty.mark(4, 4);
        maps.rebuild_dirty(&entities, &registry, &dirty);
        assert_eq!(maps.pollution.get(4, 4), 4);
        assert_eq!(maps.pollution.get(0, 0), 0);
    }

    #[test]
    fn analysis_map_fill_sets_all_tiles() {
        let mut map = AnalysisMap::new(4, 4);
        map.fill(42);
        for y in 0..4i16 {
            for x in 0..4i16 {
                assert_eq!(map.get(x, y), 42);
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
        assert_eq!(maps.land_value.get(5, 5), 73);
    }
}
