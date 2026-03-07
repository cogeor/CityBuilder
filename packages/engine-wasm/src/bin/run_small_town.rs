//! Quick test binary for the predefined small_town map.
//!
//! This is a stub — use `cargo run -p city_game` for the real renderer.

fn main() {
    use townbuilder_engine::core::predefined_maps;

    let (world, _registry, road_graph) = predefined_maps::build_small_town();
    println!("Small Town loaded:");
    println!("  Map: {}x{}", world.tiles.width(), world.tiles.height());
    println!("  Entities: {}", world.entities.iter_alive().count());
    println!("  Road nodes: {}", road_graph.node_count());
    println!("  Treasury: ${:.2}", world.treasury as f64 / 100.0);
    println!();
    println!("Use `cargo run -p city_game` for the graphical renderer.");
}
