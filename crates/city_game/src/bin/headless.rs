//! Headless simulation — run N ticks and print population growth curve.
//!
//! Usage: cargo run -p city_game --bin headless --release [-- --ticks N]
//!
//! Creates a compact 64x64 town with roads, zones, and pre-placed buildings.
//! Assumes water/electricity needs are met (flags set on entities).

use city_core::{App, MapSize, StatusFlags};
use city_engine::archetype::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use city_engine::engine::SimulationEngine;
use city_engine::network::RoadNetwork;
use city_game::plugins::terrain::TerrainPlugin;
use city_sim::plugin::{SimCorePlugin, SimConfig};
use city_sim::systems::sim_tick::SimRunState;
use city_sim::tilemap::TileKind;
use city_sim::types::{ZoneDensity, ZoneType};
use city_sim::world::WorldState;

/// Register archetypes with SHORT build times so we can see growth quickly.
fn register_fast_archetypes(registry: &mut ArchetypeRegistry) {
    // 1: Small House — 10 tick build time, capacity ~6 people
    registry.register(ArchetypeDefinition {
        id: 1, name: "Small House".into(),
        tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
        footprint_w: 1, footprint_h: 1,
        coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
        base_cost_cents: 10_000, base_upkeep_cents_per_tick: 1,
        power_demand_kw: 5, power_supply_kw: 0,
        water_demand: 2, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 2, desirability_magnitude: 5,
        pollution: 0, noise: 1,
        build_time_ticks: 10, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 0, living_space_per_person_m2: 40,
        effects: vec![],
    });

    // 3: Shop — 8 tick build time
    registry.register(ArchetypeDefinition {
        id: 3, name: "Shop".into(),
        tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
        footprint_w: 1, footprint_h: 1,
        coverage_ratio_pct: 70, floors: 1, usable_ratio_pct: 85,
        base_cost_cents: 8_000, base_upkeep_cents_per_tick: 1,
        power_demand_kw: 10, power_supply_kw: 0,
        water_demand: 3, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 3, desirability_magnitude: 3,
        pollution: 0, noise: 2,
        build_time_ticks: 8, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 25, living_space_per_person_m2: 0,
        effects: vec![],
    });

    // 4: Factory — 12 tick build time
    registry.register(ArchetypeDefinition {
        id: 4, name: "Factory".into(),
        tags: vec![ArchetypeTag::Industrial, ArchetypeTag::LowDensity],
        footprint_w: 1, footprint_h: 1,
        coverage_ratio_pct: 60, floors: 1, usable_ratio_pct: 80,
        base_cost_cents: 12_000, base_upkeep_cents_per_tick: 2,
        power_demand_kw: 15, power_supply_kw: 0,
        water_demand: 5, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 4, desirability_magnitude: -10,
        pollution: 5, noise: 4,
        build_time_ticks: 12, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 50, living_space_per_person_m2: 0,
        effects: vec![],
    });
}

/// Build a compact town on a 64x64 map.
///
/// Layout:
///   Roads: horizontal at y=10,20,30,40; vertical at x=10,30,50
///   Residential: (11-29, 11-19) and (31-49, 11-19) — 2 blocks, 342 tiles
///   Commercial:  (11-29, 21-29) — 1 block, 171 tiles
///   Industrial:  (31-49, 21-29) — 1 block, 171 tiles
///   Pre-placed: 4 houses (completed), starting pop moving in
fn build_compact_town(world: &mut WorldState) {
    world.treasury = 50_000_000; // $500k

    let w = world.tiles.width();
    let h = world.tiles.height();

    // Roads
    for &ry in &[10u32, 20, 30, 40] {
        for x in 8..52u32 {
            if x < w && ry < h {
                world.tiles.set_kind(x, ry, TileKind::Road);
            }
        }
    }
    for &rx in &[10u32, 30, 50] {
        for y in 8..42u32 {
            if rx < w && y < h {
                world.tiles.set_kind(rx, y, TileKind::Road);
            }
        }
    }

    // Mark road-adjacent tiles with ROAD_ACCESS
    for y in 0..h {
        for x in 0..w {
            if let Some(tile) = world.tiles.get(x, y) {
                if tile.kind == TileKind::Road {
                    // Mark neighbors
                    for (dx, dy) in [(-1i32,0),(1,0),(0,-1),(0,1)] {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                            use city_sim::tilemap::TileFlags;
                            world.tiles.set_flags(nx as u32, ny as u32, TileFlags::ROAD_ACCESS);
                        }
                    }
                }
            }
        }
    }

    // Residential zones
    for y in 11..20u32 {
        for x in 11..30u32 {
            world.tiles.set_zone(x, y, ZoneType::Residential);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }
    for y in 11..20u32 {
        for x in 31..50u32 {
            world.tiles.set_zone(x, y, ZoneType::Residential);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Commercial zone
    for y in 21..30u32 {
        for x in 11..30u32 {
            world.tiles.set_zone(x, y, ZoneType::Commercial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Industrial zone
    for y in 21..30u32 {
        for x in 31..50u32 {
            world.tiles.set_zone(x, y, ZoneType::Industrial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Pre-place 4 completed houses so there's immediate housing capacity
    for &(x, y) in &[(12i16, 12i16), (14, 12), (16, 12), (18, 12)] {
        if let Some(handle) = world.place_entity(1, x, y, 0) {
            world.entities.set_construction_progress(handle, 0xFFFF);
            // Mark as powered + watered so desirability is high
            world.entities.set_flags(handle, StatusFlags::POWERED | StatusFlags::WATER_CONNECTED);
        }
    }
}

fn main() {
    let total_ticks: u64 = std::env::args()
        .position(|a| a == "--ticks")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let map_size = MapSize::new(64, 64);

    let mut app = App::new();
    app.add_plugins(TerrainPlugin);
    app.add_plugins(SimCorePlugin::new(SimConfig {
        map_size,
        seed: 42,
        city_name: "Headless Town".into(),
    }));

    {
        let registry = app.get_resource_mut::<ArchetypeRegistry>().unwrap();
        register_fast_archetypes(registry);
    }

    app.insert_resource(RoadNetwork::new());

    {
        let world = app.get_resource_mut::<WorldState>().unwrap();
        build_compact_town(world);
    }

    let mut engine = SimulationEngine::from_app(app);

    println!("Headless Simulation — {} ticks, {}x{} map", total_ticks, map_size.width, map_size.height);
    println!("{:>6} {:>6} {:>8} {:>8} {:>10} {:>8} {:>6} {:>6}",
        "tick", "pop", "housing", "entities", "treasury", "bldgs", "zones", "roads");
    println!("{}", "-".repeat(76));

    let report_interval = (total_ticks / 50).max(1);

    for t in 1..=total_ticks {
        engine.tick();

        if t % report_interval == 0 || t == 1 || t == total_ticks {
            let world = engine.get_resource::<WorldState>().unwrap();
            let run_state = engine.get_resource::<SimRunState>().unwrap();
            let registry = engine.get_resource::<ArchetypeRegistry>().unwrap();

            let entity_count = world.entities.iter_alive().count();
            let mut completed_buildings = 0u32;
            let mut under_construction = 0u32;
            for handle in world.entities.iter_alive() {
                if let Some(flags) = world.entities.get_flags(handle) {
                    if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
                        under_construction += 1;
                    } else {
                        completed_buildings += 1;
                    }
                }
            }

            let mut road_tiles = 0u32;
            let mut zone_tiles = 0u32;
            let w = world.tiles.width();
            let h = world.tiles.height();
            for y in 0..h {
                for x in 0..w {
                    if let Some(tile) = world.tiles.get(x, y) {
                        match tile.kind {
                            TileKind::Road => road_tiles += 1,
                            TileKind::Zone => zone_tiles += 1,
                            TileKind::Building => {} // counted via entities
                            _ => {}
                        }
                    }
                }
            }

            let pop = run_state.population;
            let treasury_dollars = world.treasury as f64 / 100.0;
            let housing = city_sim::systems::population::compute_housing_capacity(
                &world.entities, registry,
            );

            println!("{:>6} {:>6} {:>8} {:>5}({:>2}) {:>10.0} {:>8} {:>6} {:>6}",
                t, pop, housing, completed_buildings, under_construction,
                treasury_dollars, completed_buildings, zone_tiles, road_tiles);

            if t == total_ticks {
                println!("{}", "-".repeat(76));
                println!("Final: pop={}, housing_cap={}, buildings={} (+{} building)",
                    pop, housing, completed_buildings, under_construction);
                println!("  Zone tiles: {}, Road tiles: {}", zone_tiles, road_tiles);
                println!("  Treasury: ${:.2}", treasury_dollars);
            }
        }
    }
}
