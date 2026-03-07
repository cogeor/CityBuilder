use city_core::App;
use city_core::terrain::TerrainRegistry;
use city_core::MapSize;
use city_engine::archetype::ArchetypeRegistry;
use city_engine::engine::SimulationEngine;
use city_engine::network::RoadNetwork;
use city_game::plugins::terrain::TerrainPlugin;
use city_render::projection;
use city_render::renderer;
use city_sim::plugin::{SimCorePlugin, SimConfig};
use city_sim::mapgen;
use city_sim::tilemap::{TileKind, TileValue};
use city_sim::types::ZoneType;
use city_sim::world::WorldState;

/// Map a tile to its GPU pattern ID.
///
/// Pattern ID layout (matches TileVisualRegistry):
///   0-4:   Terrain (Grass, Water, Sand, Forest, Rock)
///   7:     Road
///  11-16:  Zones (empty — Residential, Commercial, Industrial, Civic, Park, Transport)
///  21-24:  Buildings (occupied — Residential, Commercial, Industrial, Civic)
fn tile_to_pattern(tile: &TileValue) -> u32 {
    match tile.kind {
        TileKind::Road => 7,
        TileKind::Building => {
            // Occupied zone → building pattern (20 + zone_type)
            match tile.zone {
                ZoneType::Residential => 21,
                ZoneType::Commercial  => 22,
                ZoneType::Industrial  => 23,
                ZoneType::Civic       => 24,
                _ => 0,
            }
        }
        _ => {
            // Zoned but not yet built → zone pattern (10 + zone_type)
            match tile.zone {
                ZoneType::None        => 0,  // Grass
                ZoneType::Residential => 11,
                ZoneType::Commercial  => 12,
                ZoneType::Industrial  => 13,
                ZoneType::Civic       => 14,
                ZoneType::Park        => 15,
                ZoneType::Transport   => 16,
            }
        }
    }
}

fn main() {
    let empty = std::env::args().any(|a| a == "--empty");
    let map_size = if empty {
        MapSize::new(100, 100)
    } else {
        MapSize::new(1000, 1000)
    };

    // Build app with plugins
    let mut app = App::new();
    app.add_plugins(TerrainPlugin);
    app.add_plugins(SimCorePlugin::new(SimConfig {
        map_size,
        seed: 42,
        city_name: "New Town".into(),
    }));

    // Register default archetypes
    {
        let registry = app.get_resource_mut::<ArchetypeRegistry>().unwrap();
        mapgen::register_default_archetypes(registry);
    }

    // Insert RoadNetwork resource
    app.insert_resource(RoadNetwork::new());

    // Load small town by default
    if !empty {
        let mut world = app.remove_resource::<WorldState>().unwrap();
        let mut road_net = app.remove_resource::<RoadNetwork>().unwrap();
        mapgen::build_small_town(&mut world, &mut road_net);
        app.insert_resource(world);
        app.insert_resource(road_net);
    }

    // Print info
    let terrain_reg = app.get_resource::<TerrainRegistry>().unwrap();
    println!("City Builder Engine — Simulation + Renderer");
    println!("  Terrain types: {}", terrain_reg.count());
    {
        let world = app.get_resource::<WorldState>().unwrap();
        let size = world.map_size();
        let entity_count = world.entities.iter_alive().count();
        println!("  World: {}x{} ({} tiles)", size.width, size.height, size.tile_count());
        println!("  Entities: {}", entity_count);
        println!("  Treasury: ${:.2}", world.treasury as f64 / 100.0);
    }

    // Run warmup simulation ticks
    let mut engine = SimulationEngine::from_app(app);
    for _ in 0..100 {
        engine.tick();
    }
    println!("  Simulated 100 warmup ticks");

    // Build initial tile data from world state AFTER warmup
    let max_dim = map_size.width.max(map_size.height);
    let instances = build_instances_from_engine(&engine, max_dim);

    {
        let world = engine.get_resource::<WorldState>().unwrap();
        let entity_count = world.entities.iter_alive().count();
        println!("  Entities after warmup: {}", entity_count);
        println!("  Treasury after warmup: ${:.2}", world.treasury as f64 / 100.0);
    }
    println!("  Instances: {}", instances.len());

    let (cam_x, cam_y) = projection::map_center_screen(map_size.width, map_size.height);
    println!("  Camera: ({:.0}, {:.0})", cam_x, cam_y);
    println!("  Controls: WASD/Arrows to pan, scroll to zoom, Escape to quit");
    println!();

    // Zoom out so full map fits in ~800px window
    let map_screen_width = max_dim as f32 * 64.0;
    let zoom = (map_screen_width / 800.0).max(1.0);
    let cam_speed = max_dim as f32 * 4.0;

    // Run with live simulation — engine ticks every few render frames
    renderer::run_with_sim(instances, cam_x, cam_y, cam_speed, zoom, move || {
        engine.tick();
        build_instances_from_engine(&engine, max_dim)
    });
}

/// Extract tile data from the engine and build GPU instances.
fn build_instances_from_engine(engine: &SimulationEngine, max_dim: u16) -> Vec<city_render::instance::GpuInstance> {
    let world = engine.get_resource::<WorldState>().unwrap();
    let w = world.tiles.width();
    let h = world.tiles.height();
    let mut tiles = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let pattern_id = match world.tiles.get(x, y) {
                Some(tile) => tile_to_pattern(&tile),
                None => 0,
            };
            tiles.push((x as i16, y as i16, pattern_id));
        }
    }
    renderer::build_terrain_instances(&tiles, max_dim)
}
