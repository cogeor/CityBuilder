//! Predefined map generation for testing and quick-start.

use city_core::StatusFlags;
use city_engine::archetype::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use city_engine::network::RoadNetwork;

use crate::tilemap::{TileFlags, TileKind};
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

/// Draw a straight road line between two points, setting tile kind + flags.
/// Also adds segments to the road network and marks adjacent tiles with ROAD_ACCESS.
fn draw_road_line(world: &mut WorldState, road_net: &mut RoadNetwork, x0: i16, y0: i16, x1: i16, y1: i16) {
    let (dx, dy) = ((x1 - x0).signum(), (y1 - y0).signum());
    let (mut x, mut y) = (x0, y0);
    let mut prev: Option<(i16, i16)> = None;
    loop {
        // Set this tile as road
        world.tiles.set_kind(x as u32, y as u32, TileKind::Road);
        world.tiles.set_flags(x as u32, y as u32, TileFlags::ROAD_ACCESS);

        // Add segment to road network
        if let Some((px, py)) = prev {
            road_net.add_segment(
                city_core::TileCoord::new(px, py),
                city_core::TileCoord::new(x, y),
            );
        }

        // Mark cardinal neighbors with ROAD_ACCESS
        for &(nx, ny) in &[(x-1, y), (x+1, y), (x, y-1), (x, y+1)] {
            if nx >= 0 && ny >= 0 {
                world.tiles.set_flags(nx as u32, ny as u32, TileFlags::ROAD_ACCESS);
            }
        }

        prev = Some((x, y));
        if x == x1 && y == y1 { break; }
        if dx != 0 { x += dx; }
        if dy != 0 { y += dy; }
    }
}

/// Place an entity and immediately mark it as fully constructed.
fn place_completed(world: &mut WorldState, archetype_id: u16, x: i16, y: i16) {
    if let Some(handle) = world.place_entity(archetype_id, x, y, 0) {
        world.entities.set_construction_progress(handle, 0xFFFF);
        world.entities.set_flags(handle, StatusFlags::NONE);
    }
}

/// Build a small town on a 1000x1000 map with roads, zoned areas, and pre-placed buildings.
///
/// Layout matches engine-wasm's predefined_maps::build_small_town:
/// - Roads form a grid at y=400,460,520,580 and x=400,500,600
/// - Residential zones: (401-499, 401-459) and (501-599, 401-459)
/// - Commercial zones: (401-499, 461-519)
/// - Industrial zones: (501-599, 461-519)
/// - Pre-placed buildings: 6 houses, 3 shops, 1 power plant
/// - Treasury: $500,000
pub fn build_small_town(world: &mut WorldState, road_net: &mut RoadNetwork) {
    world.treasury = 50_000_000;

    // Roads: grid forming 6 blocks
    // Horizontal roads at y=400, 460, 520, 580 from x=400 to x=600
    for &y in &[400i16, 460, 520, 580] {
        draw_road_line(world, road_net, 400, y, 600, y);
    }
    // Vertical roads at x=400, 500, 600 from y=400 to y=580
    for &x in &[400i16, 500, 600] {
        draw_road_line(world, road_net, x, 400, x, 580);
    }

    // Zone: Residential blocks
    for y in 401..460u32 {
        for x in 401..500u32 {
            world.tiles.set_zone(x, y, ZoneType::Residential);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }
    for y in 401..460u32 {
        for x in 501..600u32 {
            world.tiles.set_zone(x, y, ZoneType::Residential);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Zone: Commercial block
    for y in 461..520u32 {
        for x in 401..500u32 {
            world.tiles.set_zone(x, y, ZoneType::Commercial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Zone: Industrial block
    for y in 461..520u32 {
        for x in 501..600u32 {
            world.tiles.set_zone(x, y, ZoneType::Industrial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Pre-placed houses in residential zone
    for &(x, y) in &[
        (410i16, 410i16), (420, 410), (430, 410),
        (510, 410), (520, 410), (530, 410),
    ] {
        place_completed(world, 1, x, y); // Small House
    }
    // Shops in commercial zone
    for &(x, y) in &[(410i16, 470i16), (420, 470), (430, 470)] {
        place_completed(world, 3, x, y); // Shop
    }
    // Power Plant at (410, 525)
    place_completed(world, 2, 410, 525);
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::MapSize;
    use city_engine::network::RoadNetwork;

    fn setup_small_town() -> (WorldState, RoadNetwork) {
        let mut world = WorldState::new(MapSize::new(1000, 1000), 42);
        let mut road_net = RoadNetwork::new();
        build_small_town(&mut world, &mut road_net);
        (world, road_net)
    }

    #[test]
    fn small_town_loads_without_panic() {
        let (world, _) = setup_small_town();
        assert_eq!(world.tiles.width(), 1000);
        assert_eq!(world.tiles.height(), 1000);
    }

    #[test]
    fn small_town_has_roads() {
        let (world, road_net) = setup_small_town();
        // Road at (450, 400) should exist (horizontal road at y=400)
        assert!(road_net.has_road(city_core::TileCoord::new(450, 400)));
        let tile = world.tiles.get(450, 400).unwrap();
        assert_eq!(tile.kind, crate::tilemap::TileKind::Road);
    }

    #[test]
    fn small_town_road_access_on_adjacent_tiles() {
        let (world, _) = setup_small_town();
        // Tile (401, 401) is adjacent to roads at y=400 and x=400
        let tile = world.tiles.get(401, 401).unwrap();
        assert!(tile.flags.contains(crate::tilemap::TileFlags::ROAD_ACCESS));
    }

    #[test]
    fn small_town_has_zones() {
        let (world, _) = setup_small_town();
        let tile = world.tiles.get(410, 410).unwrap();
        assert_eq!(tile.zone, ZoneType::Residential);
    }

    #[test]
    fn small_town_has_buildings() {
        let (world, _) = setup_small_town();
        let count = world.entities.iter_alive().count();
        assert!(count >= 10, "expected >= 10 entities, got {}", count);
    }

    #[test]
    fn buildings_are_complete() {
        let (world, _) = setup_small_town();
        for handle in world.entities.iter_alive() {
            let progress = world.entities.get_construction_progress(handle).unwrap();
            assert_eq!(progress, 0xFFFF);
            let flags = world.entities.get_flags(handle).unwrap();
            assert!(!flags.contains(StatusFlags::UNDER_CONSTRUCTION));
        }
    }
}
