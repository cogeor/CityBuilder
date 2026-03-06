use city_core::App;
use city_core::terrain::TerrainRegistry;
use city_core::MapSize;
use city_game::plugins::terrain::TerrainPlugin;
use city_render::projection;
use city_render::renderer;
use rand::Rng;

fn main() {
    // Build app with plugins
    let mut app = App::new();
    app.add_plugins(TerrainPlugin);

    let terrain_reg = app.get_resource::<TerrainRegistry>().unwrap();
    let terrain_count = terrain_reg.count() as u8;
    println!("City Builder Engine — Isometric Renderer");
    println!("  Terrain types: {}", terrain_count);
    for t in terrain_reg.all() {
        println!("    [{}] {}", t.id, t.name);
    }

    // Build flat tile data for a 100x100 map with random terrain
    let size = MapSize::new(100, 100);
    let mut tiles: Vec<(i16, i16, u8)> = Vec::with_capacity(size.tile_count() as usize);
    let mut rng = rand::thread_rng();
    for y in 0..size.height as i16 {
        for x in 0..size.width as i16 {
            let terrain_id = rng.gen_range(0..terrain_count);
            tiles.push((x, y, terrain_id));
        }
    }

    println!("  World: {}x{} ({} tiles)", size.width, size.height, size.tile_count());

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
