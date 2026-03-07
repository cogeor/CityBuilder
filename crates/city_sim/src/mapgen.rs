//! Predefined map generation for testing and quick-start.

use city_core::StatusFlags;
use city_engine::archetype::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};

use crate::types::{ZoneDensity, ZoneType};
use crate::world::WorldState;

/// Register a standard set of 4 building archetypes.
pub fn register_default_archetypes(registry: &mut ArchetypeRegistry) {
    // 1: Small House (1x1, Residential)
    registry.register(ArchetypeDefinition {
        id: 1,
        name: "Small House".into(),
        tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
        footprint_w: 1, footprint_h: 1,
        coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
        base_cost_cents: 100_000, base_upkeep_cents_per_tick: 10,
        power_demand_kw: 5, power_supply_kw: 0,
        water_demand: 2, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 2, desirability_magnitude: 5,
        pollution: 0, noise: 1,
        build_time_ticks: 500, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 0, living_space_per_person_m2: 40,
        effects: vec![],
    });

    // 2: Power Plant (3x3, Utility)
    registry.register(ArchetypeDefinition {
        id: 2,
        name: "Power Plant".into(),
        tags: vec![ArchetypeTag::Utility],
        footprint_w: 3, footprint_h: 3,
        coverage_ratio_pct: 60, floors: 2, usable_ratio_pct: 70,
        base_cost_cents: 500_000, base_upkeep_cents_per_tick: 50,
        power_demand_kw: 0, power_supply_kw: 5000,
        water_demand: 10, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 5, desirability_magnitude: -20,
        pollution: 8, noise: 6,
        build_time_ticks: 2000, max_level: 5,
        prerequisites: vec![],
        workspace_per_job_m2: 50, living_space_per_person_m2: 0,
        effects: vec![],
    });

    // 3: Shop (1x1, Commercial)
    registry.register(ArchetypeDefinition {
        id: 3,
        name: "Shop".into(),
        tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
        footprint_w: 1, footprint_h: 1,
        coverage_ratio_pct: 70, floors: 1, usable_ratio_pct: 85,
        base_cost_cents: 80_000, base_upkeep_cents_per_tick: 15,
        power_demand_kw: 10, power_supply_kw: 0,
        water_demand: 3, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 3, desirability_magnitude: 3,
        pollution: 0, noise: 2,
        build_time_ticks: 300, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 25, living_space_per_person_m2: 0,
        effects: vec![],
    });

    // 4: Factory (2x2, Industrial)
    registry.register(ArchetypeDefinition {
        id: 4,
        name: "Factory".into(),
        tags: vec![ArchetypeTag::Industrial, ArchetypeTag::LowDensity],
        footprint_w: 2, footprint_h: 2,
        coverage_ratio_pct: 60, floors: 1, usable_ratio_pct: 80,
        base_cost_cents: 120_000, base_upkeep_cents_per_tick: 20,
        power_demand_kw: 15, power_supply_kw: 0,
        water_demand: 5, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 4, desirability_magnitude: -10,
        pollution: 5, noise: 4,
        build_time_ticks: 600, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 50, living_space_per_person_m2: 0,
        effects: vec![],
    });
}

/// Place an entity and immediately mark it as fully constructed.
fn place_completed(world: &mut WorldState, archetype_id: u16, x: i16, y: i16) {
    if let Some(handle) = world.place_entity(archetype_id, x, y, 0) {
        world.entities.set_construction_progress(handle, 0xFFFF);
        world.entities.set_flags(handle, StatusFlags::NONE);
    }
}

/// Build a small town: 100x100 map with zoned areas and pre-placed buildings.
///
/// Layout:
/// - Roads not placed (city_sim doesn't have road commands yet)
/// - Residential zones: (10-40, 10-40)
/// - Commercial zones: (50-80, 10-40)
/// - Industrial zones: (10-40, 50-80)
/// - Pre-placed buildings: 6 houses, 3 shops, 1 power plant
/// - Treasury: $500,000
pub fn build_small_town(world: &mut WorldState) {
    world.treasury = 50_000_000;

    // Zone: Residential (10-40, 10-40)
    for y in 10..40u32 {
        for x in 10..40u32 {
            world.tiles.set_zone(x, y, ZoneType::Residential);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Zone: Commercial (50-80, 10-40)
    for y in 10..40u32 {
        for x in 50..80u32 {
            world.tiles.set_zone(x, y, ZoneType::Commercial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Zone: Industrial (10-40, 50-80)
    for y in 50..80u32 {
        for x in 10..40u32 {
            world.tiles.set_zone(x, y, ZoneType::Industrial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Pre-placed buildings
    for &(x, y) in &[
        (12i16, 12i16), (14, 12), (16, 12),
        (12, 14), (14, 14), (16, 14),
    ] {
        place_completed(world, 1, x, y); // Small House
    }
    for &(x, y) in &[(52i16, 12i16), (54, 12), (56, 12)] {
        place_completed(world, 3, x, y); // Shop
    }
    // Power Plant at (85, 85) — outside zones
    place_completed(world, 2, 85, 85);
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::MapSize;

    #[test]
    fn small_town_loads_without_panic() {
        let mut world = WorldState::new(MapSize::new(100, 100), 42);
        let mut registry = ArchetypeRegistry::new();
        register_default_archetypes(&mut registry);
        build_small_town(&mut world);
        assert_eq!(world.tiles.width(), 100);
        assert_eq!(world.tiles.height(), 100);
    }

    #[test]
    fn small_town_has_zones() {
        let mut world = WorldState::new(MapSize::new(100, 100), 42);
        build_small_town(&mut world);
        let tile = world.tiles.get(15, 15).unwrap();
        assert_eq!(tile.zone, ZoneType::Residential);
    }

    #[test]
    fn small_town_has_buildings() {
        let mut world = WorldState::new(MapSize::new(100, 100), 42);
        build_small_town(&mut world);
        let count = world.entities.iter_alive().count();
        assert!(count >= 10, "expected >= 10 entities, got {}", count);
    }

    #[test]
    fn buildings_are_complete() {
        let mut world = WorldState::new(MapSize::new(100, 100), 42);
        build_small_town(&mut world);
        for handle in world.entities.iter_alive() {
            let progress = world.entities.get_construction_progress(handle).unwrap();
            assert_eq!(progress, 0xFFFF);
            let flags = world.entities.get_flags(handle).unwrap();
            assert!(!flags.contains(StatusFlags::UNDER_CONSTRUCTION));
        }
    }
}
