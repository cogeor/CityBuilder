//! Predefined map builder for testing and development.
//!
//! Provides ready-made map configurations with pre-placed roads, zones,
//! and buildings for quick iteration without manual city setup.

use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use crate::core::commands::{self, Command};
use crate::core::network::{RoadGraph, RoadType};
use crate::core::world::WorldState;
use crate::core_types::{EntityHandle, MapSize, StatusFlags, ZoneDensity, ZoneType};

/// Create a default archetype registry with 4 standard building types.
///
/// Archetypes use empty prerequisites so that predefined map placement
/// is not blocked by validation checks (road access, power, etc.).
pub fn make_default_registry() -> ArchetypeRegistry {
    let mut reg = ArchetypeRegistry::new();

    // 1: Small House (1x1, Residential + LowDensity)
    reg.register(ArchetypeDefinition {
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
        prerequisites: vec![],
        workspace_per_job_m2: 0,
        living_space_per_person_m2: 40,
        effects: vec![],
    });

    // 2: Power Plant (3x3, Utility)
    reg.register(ArchetypeDefinition {
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
        prerequisites: vec![],
        workspace_per_job_m2: 50,
        living_space_per_person_m2: 0,
        effects: vec![],
    });

    // 3: Shop (1x2, Commercial + LowDensity)
    reg.register(ArchetypeDefinition {
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
        prerequisites: vec![],
        workspace_per_job_m2: 25,
        living_space_per_person_m2: 0,
        effects: vec![],
    });

    // 4: Hospital (2x2, Service + Civic)
    reg.register(ArchetypeDefinition {
        id: 4,
        name: "Hospital".to_string(),
        tags: vec![ArchetypeTag::Service],
        footprint_w: 2,
        footprint_h: 2,
        coverage_ratio_pct: 65,
        floors: 3,
        usable_ratio_pct: 75,
        base_cost_cents: 200_000,
        base_upkeep_cents_per_tick: 30,
        power_demand_kw: 20,
        power_supply_kw: 0,
        water_demand: 5,
        water_supply: 0,
        water_coverage_radius: 0,
        is_water_pipe: false,
        service_radius: 30,
        desirability_radius: 4,
        desirability_magnitude: 10,
        pollution: 0,
        noise: 3,
        build_time_ticks: 1000,
        max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 15,
        living_space_per_person_m2: 0,
        effects: vec![],
    });

    reg
}

/// Place an entity and mark it as fully constructed.
fn place_and_complete(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    road_graph: &mut RoadGraph,
    archetype_id: u16,
    x: i16,
    y: i16,
) -> EntityHandle {
    let result = commands::apply_command_with_registry(
        world,
        Some(registry),
        Some(road_graph),
        &Command::PlaceEntity {
            archetype_id,
            x,
            y,
            rotation: 0,
        },
    );
    let handle = match result {
        Ok(crate::core::commands::CommandEffect::EntityPlaced { handle }) => handle,
        Ok(other) => panic!("unexpected command effect: {:?}", other),
        Err(e) => panic!(
            "PlaceEntity failed for archetype {} at ({},{}): {:?}",
            archetype_id, x, y, e
        ),
    };
    world.entities.set_construction_progress(handle, 0xFFFF);
    world.entities.set_flags(handle, StatusFlags::empty());
    handle
}

/// Draw a straight road line between two points.
fn draw_road_line(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    road_graph: &mut RoadGraph,
    x0: i16,
    y0: i16,
    x1: i16,
    y1: i16,
) {
    let _ = commands::apply_command_with_registry(
        world,
        Some(registry),
        Some(road_graph),
        &Command::SetRoadLine {
            x0,
            y0,
            x1,
            y1,
            road_type: RoadType::Local,
        },
    );
}

/// Paint a rectangular zone area.
fn zone_rect(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    road_graph: &mut RoadGraph,
    x: i16,
    y: i16,
    w: u8,
    h: u8,
    zone: ZoneType,
    density: ZoneDensity,
) {
    let _ = commands::apply_command_with_registry(
        world,
        Some(registry),
        Some(road_graph),
        &Command::SetZoning {
            x,
            y,
            w,
            h,
            zone,
            density,
        },
    );
}

/// Build a 1000x1000 flat grass map with a small town center.
///
/// Layout: a grid of districts centered around (400-600, 400-580):
///
/// - Roads form a grid of 6 blocks
/// - Zones painted on block interiors
/// - Pre-placed and completed buildings (power plant, hospital, houses, shops)
/// - Treasury set to 50,000,000 cents ($500,000)
pub fn build_small_town() -> (WorldState, ArchetypeRegistry, RoadGraph) {
    let mut world = WorldState::new(MapSize { width: 1000, height: 1000 }, 42);
    let registry = make_default_registry();
    let mut road_graph = RoadGraph::new();

    // Set treasury high enough for all placements.
    world.treasury = 50_000_000;

    // ── Roads: grid forming 6 blocks ──────────────────────────────────────
    // Horizontal roads at y=400, y=460, y=520, y=580 from x=400 to x=600
    for &y in &[400i16, 460, 520, 580] {
        draw_road_line(&mut world, &registry, &mut road_graph, 400, y, 600, y);
    }
    // Vertical roads at x=400, x=500, x=600 from y=400 to y=580
    for &x in &[400i16, 500, 600] {
        draw_road_line(&mut world, &registry, &mut road_graph, x, 400, x, 580);
    }

    // ── Zones: painted on block interiors (inset 1 tile from roads) ───────
    // (401-499, 401-459): Residential, Low density
    zone_rect(
        &mut world, &registry, &mut road_graph,
        401, 401, 99, 59, ZoneType::Residential, ZoneDensity::Low,
    );
    // (501-599, 401-459): Residential, Low density
    zone_rect(
        &mut world, &registry, &mut road_graph,
        501, 401, 99, 59, ZoneType::Residential, ZoneDensity::Low,
    );
    // (401-499, 461-519): Commercial, Low density
    zone_rect(
        &mut world, &registry, &mut road_graph,
        401, 461, 99, 59, ZoneType::Commercial, ZoneDensity::Low,
    );
    // (501-599, 461-519): Industrial, Low density
    zone_rect(
        &mut world, &registry, &mut road_graph,
        501, 461, 99, 59, ZoneType::Industrial, ZoneDensity::Low,
    );

    // ── Buildings: pre-placed and completed ───────────────────────────────
    // PowerPlant (3x3) at (410, 525) — Utility, skips zone check
    place_and_complete(&mut world, &registry, &mut road_graph, 2, 410, 525);
    // Hospital (2x2) at (420, 540) — Service+Civic, skips zone check
    place_and_complete(&mut world, &registry, &mut road_graph, 4, 420, 540);
    // 6 SmallHouses in Residential zones
    for &(x, y) in &[
        (410i16, 410i16),
        (420, 410),
        (430, 410),
        (510, 410),
        (520, 410),
        (530, 410),
    ] {
        place_and_complete(&mut world, &registry, &mut road_graph, 1, x, y);
    }
    // 3 Shops in Commercial zone
    for &(x, y) in &[(410i16, 470i16), (420, 470), (430, 470)] {
        place_and_complete(&mut world, &registry, &mut road_graph, 3, x, y);
    }

    (world, registry, road_graph)
}

/// Load a predefined map by name.
///
/// Currently supported maps:
/// - `"small_town"`: 1000x1000 flat grass with a small town center
pub fn load_predefined_map(name: &str) -> Option<(WorldState, ArchetypeRegistry, RoadGraph)> {
    match name {
        "small_town" => Some(build_small_town()),
        _ => None,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_town_loads_without_panic() {
        let (world, _registry, _road_graph) = load_predefined_map("small_town").unwrap();
        assert_eq!(world.tiles.width(), 1000);
        assert_eq!(world.tiles.height(), 1000);
    }

    #[test]
    fn small_town_has_roads() {
        let (_world, _, road_graph) = load_predefined_map("small_town").unwrap();
        // Road at (450, 400) should exist (horizontal road at y=400)
        assert!(road_graph.has_road_at(crate::core_types::TileCoord { x: 450, y: 400 }));
    }

    #[test]
    fn small_town_has_zones() {
        let (world, _, _) = load_predefined_map("small_town").unwrap();
        let tile = world.tiles.get(410, 410).unwrap();
        // Should be residential zone
        assert_ne!(tile.zone, ZoneType::None);
    }

    #[test]
    fn small_town_has_buildings() {
        let (world, _, _) = load_predefined_map("small_town").unwrap();
        let mut entity_count = 0;
        for _ in world.entities.iter_alive() {
            entity_count += 1;
        }
        // At least 10 buildings (power plant + hospital + 6 houses + 3 shops)
        assert!(
            entity_count >= 10,
            "expected >= 10 entities, got {}",
            entity_count
        );
    }

    #[test]
    fn unknown_map_returns_none() {
        assert!(load_predefined_map("nonexistent").is_none());
    }

    #[test]
    fn buildings_are_complete() {
        let (world, _, _) = load_predefined_map("small_town").unwrap();
        for handle in world.entities.iter_alive() {
            let progress = world.entities.get_construction_progress(handle).unwrap();
            assert_eq!(progress, 0xFFFF, "building should be complete");
            let flags = world.entities.get_flags(handle).unwrap();
            assert!(
                !flags.contains(StatusFlags::UNDER_CONSTRUCTION),
                "should not be under construction"
            );
        }
    }
}
