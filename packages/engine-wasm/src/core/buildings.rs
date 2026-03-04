//! City-builder building definitions and zoning compatibility helpers.
//!
//! This module owns the engine-side baseline archetypes and category rules
//! required for a functional city-builder loop.

use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use crate::core_types::ZoneType;

pub const ARCH_RES_SMALL_HOUSE: u16 = 100;
pub const ARCH_UTIL_POWER_PLANT: u16 = 200;
pub const ARCH_CIV_HOSPITAL: u16 = 300;
pub const ARCH_COM_CORNER_SHOP: u16 = 400;
pub const ARCH_CIV_SCHOOL: u16 = 500;
pub const ARCH_IND_LIGHT_FACTORY: u16 = 600;

pub fn register_base_city_builder_archetypes(registry: &mut ArchetypeRegistry) {
    for def in base_city_builder_archetypes() {
        registry.register(def);
    }
}

pub fn zone_for_archetype(def: &ArchetypeDefinition) -> Option<ZoneType> {
    if def.has_tag(ArchetypeTag::Residential) {
        Some(ZoneType::Residential)
    } else if def.has_tag(ArchetypeTag::Commercial) {
        Some(ZoneType::Commercial)
    } else if def.has_tag(ArchetypeTag::Industrial) {
        Some(ZoneType::Industrial)
    } else if def.has_tag(ArchetypeTag::Civic) {
        Some(ZoneType::Civic)
    } else {
        None
    }
}

pub fn archetype_matches_zone(def: &ArchetypeDefinition, zone: ZoneType) -> bool {
    match zone_for_archetype(def) {
        Some(required) => required == zone,
        None => false,
    }
}

pub fn is_special_building(def: &ArchetypeDefinition) -> bool {
    def.has_tag(ArchetypeTag::Utility)
        || def.has_tag(ArchetypeTag::Service)
        || def.has_tag(ArchetypeTag::Transport)
}

fn base_city_builder_archetypes() -> Vec<ArchetypeDefinition> {
    vec![
        ArchetypeDefinition {
            id: ARCH_RES_SMALL_HOUSE,
            name: "Small House".to_string(),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 15_000,
            base_upkeep_cents_per_tick: 1,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 2,
            water_supply: 0,
            service_radius: 0,
            desirability_radius: 2,
            desirability_magnitude: 1,
            pollution: 0,
            noise: 1,
            build_time_ticks: 120,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
        },
        ArchetypeDefinition {
            id: ARCH_UTIL_POWER_PLANT,
            name: "Coal Power Plant".to_string(),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 3,
            footprint_h: 3,
            coverage_ratio_pct: 70,
            floors: 2,
            usable_ratio_pct: 90,
            base_cost_cents: 250_000,
            base_upkeep_cents_per_tick: 15,
            power_demand_kw: 0,
            power_supply_kw: 500,
            water_demand: 10,
            water_supply: 0,
            service_radius: 0,
            desirability_radius: 8,
            desirability_magnitude: -5,
            pollution: 30,
            noise: 20,
            build_time_ticks: 500,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 25,
            living_space_per_person_m2: 0,
        },
        ArchetypeDefinition {
            id: ARCH_CIV_HOSPITAL,
            name: "Hospital".to_string(),
            tags: vec![ArchetypeTag::Civic, ArchetypeTag::Service],
            footprint_w: 3,
            footprint_h: 3,
            coverage_ratio_pct: 60,
            floors: 4,
            usable_ratio_pct: 75,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 25,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 20,
            water_supply: 0,
            service_radius: 30,
            desirability_radius: 5,
            desirability_magnitude: 3,
            pollution: 0,
            noise: 5,
            build_time_ticks: 700,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 15,
            living_space_per_person_m2: 0,
        },
        ArchetypeDefinition {
            id: ARCH_COM_CORNER_SHOP,
            name: "Corner Shop".to_string(),
            tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 2,
            coverage_ratio_pct: 80,
            floors: 1,
            usable_ratio_pct: 85,
            base_cost_cents: 25_000,
            base_upkeep_cents_per_tick: 3,
            power_demand_kw: 8,
            power_supply_kw: 0,
            water_demand: 3,
            water_supply: 0,
            service_radius: 10,
            desirability_radius: 3,
            desirability_magnitude: 2,
            pollution: 1,
            noise: 3,
            build_time_ticks: 160,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 20,
            living_space_per_person_m2: 0,
        },
        ArchetypeDefinition {
            id: ARCH_CIV_SCHOOL,
            name: "Elementary School".to_string(),
            tags: vec![ArchetypeTag::Civic, ArchetypeTag::Service],
            footprint_w: 2,
            footprint_h: 3,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 70,
            base_cost_cents: 200_000,
            base_upkeep_cents_per_tick: 12,
            power_demand_kw: 20,
            power_supply_kw: 0,
            water_demand: 8,
            water_supply: 0,
            service_radius: 20,
            desirability_radius: 4,
            desirability_magnitude: 4,
            pollution: 0,
            noise: 10,
            build_time_ticks: 350,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 20,
            living_space_per_person_m2: 0,
        },
        ArchetypeDefinition {
            id: ARCH_IND_LIGHT_FACTORY,
            name: "Light Factory".to_string(),
            tags: vec![ArchetypeTag::Industrial, ArchetypeTag::LowDensity],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 75,
            floors: 1,
            usable_ratio_pct: 85,
            base_cost_cents: 75_000,
            base_upkeep_cents_per_tick: 8,
            power_demand_kw: 22,
            power_supply_kw: 0,
            water_demand: 6,
            water_supply: 0,
            service_radius: 0,
            desirability_radius: 4,
            desirability_magnitude: -3,
            pollution: 12,
            noise: 8,
            build_time_ticks: 240,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 30,
            living_space_per_person_m2: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_base_archetypes_populates_registry() {
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        assert!(registry.count() >= 6);
        assert!(registry.get(ARCH_RES_SMALL_HOUSE).is_some());
        assert!(registry.get(ARCH_IND_LIGHT_FACTORY).is_some());
    }

    #[test]
    fn zone_compatibility_matches_expected_categories() {
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);

        let house = registry.get(ARCH_RES_SMALL_HOUSE).unwrap();
        assert!(archetype_matches_zone(house, ZoneType::Residential));
        assert!(!archetype_matches_zone(house, ZoneType::Industrial));

        let shop = registry.get(ARCH_COM_CORNER_SHOP).unwrap();
        assert!(archetype_matches_zone(shop, ZoneType::Commercial));

        let hospital = registry.get(ARCH_CIV_HOSPITAL).unwrap();
        assert!(archetype_matches_zone(hospital, ZoneType::Civic));
        assert!(is_special_building(hospital));

        let plant = registry.get(ARCH_UTIL_POWER_PLANT).unwrap();
        assert!(is_special_building(plant));
        assert_eq!(zone_for_archetype(plant), None);
    }
}
