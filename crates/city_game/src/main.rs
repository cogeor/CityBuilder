use city_core::App;
use city_core::terrain::TerrainRegistry;
use city_core::MapSize;
use city_engine::archetype::ArchetypeRegistry;
use city_engine::engine::SimulationEngine;
use city_game::plugins::terrain::TerrainPlugin;
use city_render::projection;
use city_render::renderer;
use city_sim::plugin::{SimCorePlugin, SimConfig};
use city_sim::mapgen;
use city_sim::world::WorldState;

fn main() {
    let use_predefined = std::env::args().any(|a| a == "--small-town");

    let map_size = if use_predefined {
        MapSize::new(100, 100)
    } else {
        MapSize::new(100, 100)
    };

    // Build app with plugins
    let mut app = App::new();
    app.add_plugins(TerrainPlugin);
    app.add_plugins(SimCorePlugin::new(SimConfig {
        map_size,
        seed: 42,
        city_name: "New Town".into(),
    }));

    // Register default archetypes and optionally load predefined map
    {
        let registry = app.get_resource_mut::<ArchetypeRegistry>().unwrap();
        mapgen::register_default_archetypes(registry);
    }
    if use_predefined {
        let world = app.get_resource_mut::<WorldState>().unwrap();
        mapgen::build_small_town(world);
        println!("Loaded predefined map: small_town");
    }

    // Print terrain info
    let terrain_reg = app.get_resource::<TerrainRegistry>().unwrap();
    let terrain_count = terrain_reg.count() as u8;
    println!("City Builder Engine — Simulation + Renderer");
    println!("  Terrain types: {}", terrain_count);
    for t in terrain_reg.all() {
        println!("    [{}] {}", t.id, t.name);
    }

    // Print world info
    {
        let world = app.get_resource::<WorldState>().unwrap();
        let size = world.map_size();
        let entity_count = world.entities.iter_alive().count();
        println!("  World: {}x{} ({} tiles)", size.width, size.height, size.tile_count());
        println!("  Entities: {}", entity_count);
        println!("  Treasury: ${:.2}", world.treasury as f64 / 100.0);
    }

    // Run a few simulation ticks before rendering
    let mut engine = SimulationEngine::from_app(app);
    let warmup_ticks = if use_predefined { 100 } else { 0 };
    for _ in 0..warmup_ticks {
        engine.tick();
    }
    if warmup_ticks > 0 {
        println!("  Simulated {} warmup ticks", warmup_ticks);
    }

    // Build tile data for rendering from the world state
    // After SimulationEngine::from_app, resources are inside the engine.
    // For now, build a simple terrain-only render from map size.
    let size = map_size;
    let mut tiles: Vec<(i16, i16, u8)> = Vec::with_capacity(size.tile_count() as usize);
    let mut rng = rand::thread_rng();
    use rand::Rng;
    for y in 0..size.height as i16 {
        for x in 0..size.width as i16 {
            let terrain_id = rng.gen_range(0..terrain_count);
            tiles.push((x, y, terrain_id));
        }
    }

    // Build render instances
    let max_dim = size.width.max(size.height);
    let instances = renderer::build_terrain_instances(&tiles, max_dim);
    println!("  Instances: {}", instances.len());

    // Compute camera start position (center of map)
    let (cam_x, cam_y) = projection::map_center_screen(size.width, size.height);
    println!("  Camera: ({:.0}, {:.0})", cam_x, cam_y);
    println!("  Controls: WASD/Arrows to pan, Escape to quit");
    println!();

    // Launch renderer
    renderer::run(instances, cam_x, cam_y);
}
