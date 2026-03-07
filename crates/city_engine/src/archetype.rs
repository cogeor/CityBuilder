//! Archetype system: declarative building/road definitions.
//!
//! Archetypes describe how to compute a building's parameters.
//! Store only archetype_id + level; derive capacity, cost, etc. on demand.

use city_core::{ArchetypeId, MoneyCents, TILE_AREA_M2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kind of spatial effect a building emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EffectKind {
    Pollution        = 0,
    LandValue        = 1,
    Crime            = 2,
    FireProtection   = 3,
    PoliceProtection = 4,
    Power            = 5,
    Water            = 6,
    Noise            = 7,
}

/// Number of distinct effect kinds (used to size fixed arrays).
pub const EFFECT_KIND_COUNT: usize = 8;

/// A single building effect.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Effect {
    pub kind: EffectKind,
    pub value: i32,
    pub radius: u8,
}

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
    /// Power supply in kW (only for power plants).
    pub power_supply_kw: u32,
    /// Water demand in units.
    pub water_demand: u32,
    /// Water supply in units.
    pub water_supply: u32,
    /// Water coverage radius in tiles (for pumps).
    pub water_coverage_radius: u8,
    /// Whether this building acts as a water pipe.
    pub is_water_pipe: bool,
    /// Service radius in tiles.
    pub service_radius: u8,
    /// Desirability effect: radius and magnitude.
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
    /// Workspace per job in m².
    pub workspace_per_job_m2: u32,
    /// Living space per person in m².
    pub living_space_per_person_m2: u32,
    /// Spatial effects emitted by this building.
    #[serde(default)]
    pub effects: Vec<Effect>,
}

impl ArchetypeDefinition {
    /// Compute gross floor area in m².
    pub fn gross_floor_area_m2(&self) -> u32 {
        let footprint_m2 = self.footprint_w as u32 * self.footprint_h as u32 * TILE_AREA_M2;
        footprint_m2 * self.coverage_ratio_pct as u32 * self.floors as u32 / 100
    }

    /// Compute residential capacity (0 for non-residential).
    pub fn resident_capacity(&self) -> u32 {
        if self.living_space_per_person_m2 == 0 { return 0; }
        self.gross_floor_area_m2() / self.living_space_per_person_m2
    }

    /// Compute job capacity (0 for non-employment).
    pub fn job_capacity(&self) -> u32 {
        if self.workspace_per_job_m2 == 0 { return 0; }
        self.gross_floor_area_m2() / self.workspace_per_job_m2
    }

    /// Construction cost scaled by level (+50% per level).
    pub fn cost_at_level(&self, level: u8) -> MoneyCents {
        let multiplier = 100 + (level.saturating_sub(1) as i64) * 50;
        self.base_cost_cents * multiplier / 100
    }

    /// Upkeep per tick scaled by level (+30% per level).
    pub fn upkeep_at_level(&self, level: u8) -> MoneyCents {
        let multiplier = 100 + (level.saturating_sub(1) as i64) * 30;
        self.base_upkeep_cents_per_tick * multiplier / 100
    }

    /// Power demand scaled by level (+20% per level).
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
    pub fn new() -> Self { Self::default() }

    /// Register an archetype. Overwrites if id already exists.
    pub fn register(&mut self, def: ArchetypeDefinition) {
        self.archetypes.insert(def.id, def);
    }

    pub fn get(&self, id: ArchetypeId) -> Option<&ArchetypeDefinition> {
        self.archetypes.get(&id)
    }

    pub fn list_ids(&self) -> Vec<ArchetypeId> {
        self.archetypes.keys().copied().collect()
    }

    pub fn list_by_tag(&self, tag: ArchetypeTag) -> Vec<&ArchetypeDefinition> {
        self.archetypes.values().filter(|a| a.has_tag(tag)).collect()
    }

    pub fn count(&self) -> usize { self.archetypes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_small_house() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 1,
            name: "Small House".into(),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
            base_cost_cents: 100_000,
            base_upkeep_cents_per_tick: 10,
            power_demand_kw: 5, power_supply_kw: 0,
            water_demand: 2, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2, desirability_magnitude: 5,
            pollution: 0, noise: 1,
            build_time_ticks: 500, max_level: 3,
            prerequisites: vec![Prerequisite::RoadAccess, Prerequisite::PowerConnection],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
            effects: vec![],
        }
    }

    fn make_power_plant() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 2,
            name: "Power Plant".into(),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 3, footprint_h: 3,
            coverage_ratio_pct: 60, floors: 2, usable_ratio_pct: 70,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 50,
            power_demand_kw: 0, power_supply_kw: 5000,
            water_demand: 10, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 5, desirability_magnitude: -20,
            pollution: 8, noise: 6,
            build_time_ticks: 2000, max_level: 5,
            prerequisites: vec![Prerequisite::RoadAccess],
            workspace_per_job_m2: 50,
            living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    fn make_shop() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 3,
            name: "Shop".into(),
            tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
            footprint_w: 1, footprint_h: 2,
            coverage_ratio_pct: 70, floors: 1, usable_ratio_pct: 85,
            base_cost_cents: 80_000,
            base_upkeep_cents_per_tick: 15,
            power_demand_kw: 10, power_supply_kw: 0,
            water_demand: 3, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 3, desirability_magnitude: 3,
            pollution: 0, noise: 2,
            build_time_ticks: 300, max_level: 3,
            prerequisites: vec![
                Prerequisite::RoadAccess,
                Prerequisite::PowerConnection,
                Prerequisite::WaterConnection,
            ],
            workspace_per_job_m2: 25,
            living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    #[test]
    fn small_house_floor_area() {
        let h = make_small_house();
        // 1x1 tile=256m², 50% coverage, 2 floors = 256
        assert_eq!(h.gross_floor_area_m2(), 256);
    }

    #[test]
    fn power_plant_floor_area() {
        let p = make_power_plant();
        // 3x3=9*256=2304m², 60% coverage, 2 floors = 2764
        assert_eq!(p.gross_floor_area_m2(), 2764);
    }

    #[test]
    fn shop_floor_area() {
        let s = make_shop();
        // 1x2=2*256=512m², 70% coverage, 1 floor = 358
        assert_eq!(s.gross_floor_area_m2(), 358);
    }

    #[test]
    fn resident_capacity() {
        assert_eq!(make_small_house().resident_capacity(), 6); // 256/40
        assert_eq!(make_power_plant().resident_capacity(), 0);
    }

    #[test]
    fn job_capacity() {
        assert_eq!(make_shop().job_capacity(), 14); // 358/25
        assert_eq!(make_power_plant().job_capacity(), 55); // 2764/50
        assert_eq!(make_small_house().job_capacity(), 0);
    }

    #[test]
    fn cost_at_level() {
        let h = make_small_house();
        assert_eq!(h.cost_at_level(1), 100_000);
        assert_eq!(h.cost_at_level(2), 150_000);
        assert_eq!(h.cost_at_level(3), 200_000);
        assert_eq!(h.cost_at_level(0), 100_000); // saturates
    }

    #[test]
    fn upkeep_at_level() {
        let h = make_small_house();
        assert_eq!(h.upkeep_at_level(1), 10);
        assert_eq!(h.upkeep_at_level(2), 13);
        assert_eq!(h.upkeep_at_level(3), 16);
    }

    #[test]
    fn power_demand_at_level() {
        let h = make_small_house();
        assert_eq!(h.power_demand_at_level(1), 5);
        assert_eq!(h.power_demand_at_level(2), 6);
        assert_eq!(h.power_demand_at_level(3), 7);
        assert_eq!(make_power_plant().power_demand_at_level(1), 0);
    }

    #[test]
    fn tags() {
        let h = make_small_house();
        assert!(h.has_tag(ArchetypeTag::Residential));
        assert!(h.has_tag(ArchetypeTag::LowDensity));
        assert!(!h.has_tag(ArchetypeTag::Commercial));
    }

    #[test]
    fn registry() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        reg.register(make_power_plant());
        reg.register(make_shop());
        assert_eq!(reg.count(), 3);
        assert_eq!(reg.get(1).unwrap().name, "Small House");
        let residential = reg.list_by_tag(ArchetypeTag::Residential);
        assert_eq!(residential.len(), 1);
        let mut ids = reg.list_ids();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn registry_overwrite() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_small_house());
        let mut updated = make_small_house();
        updated.name = "Updated".into();
        reg.register(updated);
        assert_eq!(reg.count(), 1);
        assert_eq!(reg.get(1).unwrap().name, "Updated");
    }
}
