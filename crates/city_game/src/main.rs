//! Visual city builder — renders the compact 64x64 town with live simulation.

use city_game::scenario;
use city_render::projection;
use city_render::renderer;

fn main() {
    let (mut engine, map_size) = scenario::create_compact_engine();
    let max_dim = map_size.width.max(map_size.height);

    // Warmup
    for _ in 0..20 {
        engine.tick();
    }

    let frame = scenario::build_frame_data(&engine, max_dim);
    let (cam_x, cam_y) = projection::map_center_screen(map_size.width, map_size.height);

    println!("City Builder — {}x{}, {} terrain + {} sprites",
        map_size.width, map_size.height, frame.terrain.len(), frame.sprites.len());
    println!("  Controls: WASD/Arrows pan, scroll zoom, Escape quit");

    let map_screen_width = max_dim as f32 * 64.0;
    let zoom = (map_screen_width / 1200.0).max(1.0);
    let cam_speed = max_dim as f32 * 4.0;

    renderer::run_with_sim(frame.terrain, frame.sprites, cam_x, cam_y, cam_speed, zoom, 10.0, move || {
        engine.tick();
        scenario::build_frame_data(&engine, max_dim)
    });
}
