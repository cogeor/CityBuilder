//! Archetype system: declarative building/road definitions.
//!
//! Archetypes describe how to compute a building's parameters.
//! Store only archetype_id + level; derive capacity, cost, etc. on demand.

use crate::core_types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tags that classify an archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ArchetypeTag {
    Residential,
    Commercial,
    Industrial,
    Civic,
    Utility,
    Transport,
    Service,
    LowDensity,
    MediumDensity,
    HighDensity,
    /// Power-line infrastructure: tiles occupied by this archetype act as
    /// electrical conductors (BFS propagation).
    PowerLine,
}

/// Prerequisite for placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prerequisite {
    RoadAccess,
    PowerConnection,
    WaterConnection,
}

/// Full archetype definition loaded from content plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeDefinition {
    pub id: ArchetypeId,
    pub name: String,
    pub tags: Vec<ArchetypeTag>,
    /// Footprint in tiles.
    pub footprint_w: u8,
    pub footprint_h: u8,
    /// Building coverage ratio (0-100 percent).
    pub coverage_ratio_pct: u8,
    /// Number of floors/stories.
    pub floors: u8,
    /// Net-to-gross usable ratio (0-100 percent).
    pub usable_ratio_pct: u8,
    /// Base construction cost in cents.
    pub base_cost_cents: MoneyCents,
    /// Upkeep cost per tick in cents (at level 1).
    pub base_upkeep_cents_per_tick: MoneyCents,
    /// Power demand in kW (at level 1). 0 for power producers.
    pub power_demand_kw: u32,
    /// Power supply in kW (only for power plants). 0 for consumers.
    pub power_supply_kw: u32,
    /// Water demand in units.
    pub water_demand: u32,
    /// Water supply in units (only for water facilities).
    pub water_supply: u32,
    /// Radius in tiles over which a water facility distributes supply.
    /// 0 for buildings that are not water sources.
    pub water_coverage_radius: u8,
    /// When true, tiles occupied by this archetype act as water
    /// conductors (BFS propagation, analogous to PowerLine).
    pub is_water_pipe: bool,
    /// Service radius in tiles (for service buildings like hospitals, schools).
    pub service_radius: u8,
    /// Desirability effect: positive for parks, negative for industry.
    /// Radius in tiles and magnitude.
    pub desirability_radius: u8,
    pub desirability_magnitude: i16,
    /// Pollution intensity (0 = none).
    pub pollution: u8,
    /// Noise intensity (0 = none).
    pub noise: u8,
    /// Build time in ticks.
    pub build_time_ticks: u32,
    /// Maximum upgrade level.
    pub max_level: u8,
    /// Prerequisites for placement.
    pub prerequisites: Vec<Prerequisite>,
    /// Workspace per job in m² (for commercial/industrial; 0 for residential).
    pub workspace_per_job_m2: u32,
    /// Living space per person in m² (for residential; 0 for non-residential).
    pub living_space_per_person_m2: u32,
}

impl ArchetypeDefinition {
    /// Compute gross floor area for this archetype.
    pub fn gross_floor_area_m2(&self) -> u32 {
        crate::core::scale::gross_floor_area_m2(
            self.footprint_w,
            self.footprint_h,
            self.coverage_ratio_pct,
            self.floors,
        )
    }

    /// Compute residential capacity (number of residents).
    /// Returns 0 for non-residential archetypes.
    pub fn resident_capacity(&self) -> u32 {
        if self.living_space_per_person_m2 == 0 {
            return 0;
        }
        crate::core::scale::residents_from_floor_area(
            self.gross_floor_area_m2(),
            self.living_space_per_person_m2,
        )
    }

    /// Compute job capacity.
    /// Returns 0 for non-employment archetypes.
    pub fn job_capacity(&self) -> u32 {
        if self.workspace_per_job_m2 == 0 {
            return 0;
        }
        crate::core::scale::jobs_from_floor_area(
            self.gross_floor_area_m2(),
            self.workspace_per_job_m2,
        )
    }

    /// Compute construction cost scaled by level.
    pub fn cost_at_level(&self, level: u8) -> MoneyCents {
        // Each level costs 50% more than the previous
        let multiplier = 100 + (level.saturating_sub(1) as i64) * 50;
        self.base_cost_cents * multiplier / 100
    }

    /// Compute upkeep per tick scaled by level.
    pub fn upkeep_at_level(&self, level: u8) -> MoneyCents {
        let multiplier = 100 + (level.saturating_sub(1) as i64) * 30;
        self.base_upkeep_cents_per_tick * multiplier / 100
    }

    /// Compute power demand scaled by level.
    pub fn power_demand_at_level(&self, level: u8) -> u32 {
        self.power_demand_kw * (100 + (level.saturating_sub(1) as u32) * 20) / 100
    }

    /// Check if archetype has a given tag.
    pub fn has_tag(&self, tag: ArchetypeTag) -> bool {
        self.tags.contains(&tag)
    }
}

/// Registry that holds all loaded archetype definitions.
#[derive(Debug, Default)]
pub struct ArchetypeRegistry {
    archetypes: HashMap<ArchetypeId, ArchetypeDefinition>,
}

impl ArchetypeRegistry {
    pub fn new() -> Self {
        ArchetypeRegistry {
            archetypes: HashMap::new(),
        }
    }

    /// Register an archetype. Overwrites if id already exists.
    pub fn register(&mut self, def: ArchetypeDefinition) {
        self.archetypes.insert(def.id, def);
    }

    /// Get an archetype by ID.
    pub fn get(&self, id: ArchetypeId) -> Option<&ArchetypeDefinition> {
        self.archetypes.get(&id)
    }

    /// List all archetype IDs.
    pub fn list_ids(&self) -> Vec<ArchetypeId> {
        self.archetypes.keys().copied().collect()
    }

    /// List archetypes that have a specific tag.
    pub fn list_by_tag(&self, tag: ArchetypeTag) -> Vec<&ArchetypeDefinition> {
        self.archetypes.values().filter(|a| a.has_tag(tag)).collect()
    }

    /// Number of registered archetypes.
    pub fn count(&self) -> usize {
        self.archetypes.len()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a small_house archetype (1x1, residential, 2 floors, 50% coverage).
    fn make_small_house() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 1,
            name: "Small House".to_string(),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 100_000,
            base_upkeep_cents_per_tick: 10,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 2,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2,
            desirability_magnitude: 5,
            pollution: 0,
            noise: 1,
            build_time_ticks: 500,
            max_level: 3,
            prerequisites: vec![Prerequisite::RoadAccess, Prerequisite::PowerConnection],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
        }
    }

    /// Helper: create a power_plant archetype (3x3, utility, power_supply_kw = 5000).
    fn make_power_plant() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 2,
            name: "Power Plant".to_string(),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 3,
            footprint_h: 3,
            coverage_ratio_pct: 60,
            floors: 2,
            usable_ratio_pct: 70,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 50,
            power_demand_kw: 0,
            power_supply_kw: 5000,
            water_demand: 10,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 5,
            desirability_magnitude: -20,
            pollution: 8,
            noise: 6,
            build_time_ticks: 2000,
            max_level: 5,
            prerequisites: vec![Prerequisite::RoadAccess],
            workspace_per_job_m2: 50,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create a shop archetype (1x2, commercial).
    fn make_shop() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 3,
            name: "Shop".to_string(),
            tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 2,
            coverage_ratio_pct: 70,
            floors: 1,
            usable_ratio_pct: 85,
            base_cost_cents: 80_000,
            base_upkeep_cents_per_tick: 15,
            power_demand_kw: 10,
            power_supply_kw: 0,
            water_demand: 3,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 3,
            desirability_magnitude: 3,
            pollution: 0,
            noise: 2,
            build_time_ticks: 300,
            max_level: 3,
            prerequisites: vec![
                Prerequisite::RoadAccess,
                Prerequisite::PowerConnection,
                Prerequisite::WaterConnection,
            ],
            workspace_per_job_m2: 25,
            living_space_per_person_m2: 0,
        }
    }

    // ─── Floor Area Tests ───────────────────────────────────────────────────

    #[test]
    fn small_house_gross_floor_area() {
        let house = make_small_house();
        // 1x1 tile = 256 m², 50% coverage, 2 floors = 256 * 50 * 2 / 100 = 256
        assert_eq!(house.gross_floor_area_m2(), 256);
    }

    #[test]
    fn power_plant_gross_floor_area() {
        let plant = make_power_plant();
        // 3x3 tiles = 9 * 256 = 2304 m², 60% coverage, 2 floors
        // = 2304 * 60 * 2 / 100 = 2764
        assert_eq!(plant.gross_floor_area_m2(), 2764);
    }

    #[test]
    fn shop_gross_floor_area() {
        let shop = make_shop();
        // 1x2 tiles = 2 * 256 = 512 m², 70% coverage, 1 floor
        // = 512 * 70 * 1 / 100 = 358
        assert_eq!(shop.gross_floor_area_m2(), 358);
    }

    // ─── Resident Capacity Tests ────────────────────────────────────────────

    #[test]
    fn small_house_resident_capacity() {
        let house = make_small_house();
        // 256 m² / 40 m²/person = 6
        assert_eq!(house.resident_capacity(), 6);
    }

    #[test]
    fn non_residential_resident_capacity_is_zero() {
        let plant = make_power_plant();
        assert_eq!(plant.resident_capacity(), 0);

        let shop = make_shop();
        assert_eq!(shop.resident_capacity(), 0);
    }

    // ─── Job Capacity Tests ─────────────────────────────────────────────────

    #[test]
    fn shop_job_capacity() {
        let shop = make_shop();
        // 358 m² / 25 m²/job = 14
        assert_eq!(shop.job_capacity(), 14);
    }

    #[test]
    fn power_plant_job_capacity() {
        let plant = make_power_plant();
        // 2764 m² / 50 m²/job = 55
        assert_eq!(plant.job_capacity(), 55);
    }

    #[test]
    fn residential_job_capacity_is_zero() {
        let house = make_small_house();
        assert_eq!(house.job_capacity(), 0);
    }

    // ─── Cost At Level Tests ────────────────────────────────────────────────

    #[test]
    fn cost_at_level_1() {
        let house = make_small_house();
        // Level 1: base cost * 100 / 100 = 100_000
        assert_eq!(house.cost_at_level(1), 100_000);
    }

    #[test]
    fn cost_at_level_2() {
        let house = make_small_house();
        // Level 2: base * 150 / 100 = 150_000
        assert_eq!(house.cost_at_level(2), 150_000);
    }

    #[test]
    fn cost_at_level_3() {
        let house = make_small_house();
        // Level 3: base * 200 / 100 = 200_000
        assert_eq!(house.cost_at_level(3), 200_000);
    }

    #[test]
    fn cost_at_level_0_saturates() {
        let house = make_small_house();
        // Level 0: saturating_sub(1) = 0, so multiplier = 100
        assert_eq!(house.cost_at_level(0), 100_000);
    }

    // ─── Upkeep At Level Tests ──────────────────────────────────────────────

    #[test]
    fn upkeep_at_level_1() {
        let house = make_small_house();
        // Level 1: base * 100 / 100 = 10
        assert_eq!(house.upkeep_at_level(1), 10);
    }

    #[test]
    fn upkeep_at_level_2() {
        let house = make_small_house();
        // Level 2: base * 130 / 100 = 13
        assert_eq!(house.upkeep_at_level(2), 13);
    }

    #[test]
    fn upkeep_at_level_3() {
        let house = make_small_house();
        // Level 3: base * 160 / 100 = 16
        assert_eq!(house.upkeep_at_level(3), 16);
    }

    // ─── Power Demand At Level Tests ────────────────────────────────────────

    #[test]
    fn power_demand_at_level_1() {
        let house = make_small_house();
        // Level 1: 5 * 100 / 100 = 5
        assert_eq!(house.power_demand_at_level(1), 5);
    }

    #[test]
    fn power_demand_at_level_2() {
        let house = make_small_house();
        // Level 2: 5 * 120 / 100 = 6
        assert_eq!(house.power_demand_at_level(2), 6);
    }

    #[test]
    fn power_demand_at_level_3() {
        let house = make_small_house();
        // Level 3: 5 * 140 / 100 = 7
        assert_eq!(house.power_demand_at_level(3), 7);
    }

    #[test]
    fn power_producer_demand_is_zero() {
        let plant = make_power_plant();
        // power_demand_kw = 0, so any level is 0
        assert_eq!(plant.power_demand_at_level(1), 0);
        assert_eq!(plant.power_demand_at_level(5), 0);
    }

    // ─── Tag Tests ──────────────────────────────────────────────────────────

    #[test]
    fn has_tag_residential() {
        let house = make_small_house();
        assert!(house.has_tag(ArchetypeTag::Residential));
        assert!(house.has_tag(ArchetypeTag::LowDensity));
        assert!(!house.has_tag(ArchetypeTag::Commercial));
        assert!(!house.has_tag(ArchetypeTag::Utility));
    }

    #[test]
    fn has_tag_utility() {
        let plant = make_power_plant();
        assert!(plant.has_tag(ArchetypeTag::Utility));
        assert!(!plant.has_tag(ArchetypeTag::Residential));
    }

    #[test]
    fn has_tag_commercial() {
        let shop = make_shop();
        assert!(shop.has_tag(ArchetypeTag::Commercial));
        assert!(shop.has_tag(ArchetypeTag::LowDensity));
        assert!(!shop.has_tag(ArchetypeTag::Industrial));
    }

    // ─── Registry Tests ─────────────────────────────────────────────────────

    #[test]
    fn registry_empty() {
        let reg = ArchetypeRegistry::new();
        assert_eq!(reg.count(), 0);
        assert!(reg.get(1).is_none());
        assert!(reg.list_ids().is_empty());
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        assert_eq!(reg.count(), 1);
        let house = reg.get(1).unwrap();
        assert_eq!(house.name, "Small House");
        assert_eq!(house.id, 1);
    }

    #[test]
    fn registry_register_multiple() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        reg.register(make_power_plant());
        reg.register(make_shop());
        assert_eq!(reg.count(), 3);
    }

    #[test]
    fn registry_overwrite() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        let mut updated = make_small_house();
        updated.name = "Updated House".to_string();
        reg.register(updated);
        assert_eq!(reg.count(), 1);
        assert_eq!(reg.get(1).unwrap().name, "Updated House");
    }

    #[test]
    fn registry_get_missing() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        assert!(reg.get(999).is_none());
    }

    #[test]
    fn registry_list_ids() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        reg.register(make_power_plant());
        reg.register(make_shop());
        let mut ids = reg.list_ids();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn registry_list_by_tag_residential() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        reg.register(make_power_plant());
        reg.register(make_shop());
        let residential = reg.list_by_tag(ArchetypeTag::Residential);
        assert_eq!(residential.len(), 1);
        assert_eq!(residential[0].name, "Small House");
    }

    #[test]
    fn registry_list_by_tag_low_density() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        reg.register(make_power_plant());
        reg.register(make_shop());
        let low_density = reg.list_by_tag(ArchetypeTag::LowDensity);
        assert_eq!(low_density.len(), 2);
        let mut names: Vec<&str> = low_density.iter().map(|a| a.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["Shop", "Small House"]);
    }

    #[test]
    fn registry_list_by_tag_none_match() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        reg.register(make_shop());
        let industrial = reg.list_by_tag(ArchetypeTag::Industrial);
        assert!(industrial.is_empty());
    }

    #[test]
    fn registry_default_is_empty() {
        let reg = ArchetypeRegistry::default();
        assert_eq!(reg.count(), 0);
    }
}
