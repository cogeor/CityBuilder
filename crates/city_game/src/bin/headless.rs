//! Headless simulation — run N ticks and print population growth curve.
//!
//! Usage: cargo run -p city_game --bin headless --release [-- --ticks N]

use city_core::StatusFlags;
use city_engine::archetype::ArchetypeRegistry;
use city_game::scenario;
use city_sim::systems::sim_tick::SimRunState;
use city_sim::tilemap::TileKind;
use city_sim::world::WorldState;

fn main() {
    let total_ticks: u64 = std::env::args()
        .position(|a| a == "--ticks")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let (mut engine, map_size) = scenario::create_compact_engine();

    println!("Headless Simulation — {} ticks, {}x{} map", total_ticks, map_size.width, map_size.height);
    println!("{:>6} {:>6} {:>8} {:>8} {:>10} {:>8} {:>6}",
        "tick", "pop", "housing", "entities", "treasury", "bldgs", "roads");
    println!("{}", "-".repeat(68));

    let report_interval = (total_ticks / 50).max(1);

    for t in 1..=total_ticks {
        engine.tick();

        if t % report_interval == 0 || t == 1 || t == total_ticks {
            let world = engine.get_resource::<WorldState>().unwrap();
            let run_state = engine.get_resource::<SimRunState>().unwrap();
            let registry = engine.get_resource::<ArchetypeRegistry>().unwrap();

            let mut completed = 0u32;
            let mut constructing = 0u32;
            for handle in world.entities.iter_alive() {
                if let Some(flags) = world.entities.get_flags(handle) {
                    if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
                        constructing += 1;
                    } else {
                        completed += 1;
                    }
                }
            }

            let mut road_tiles = 0u32;
            let (w, h) = (world.tiles.width(), world.tiles.height());
            for y in 0..h {
                for x in 0..w {
                    if let Some(tile) = world.tiles.get(x, y) {
                        if tile.kind == TileKind::Road { road_tiles += 1; }
                    }
                }
            }

            let pop = run_state.population;
            let housing = city_sim::systems::population::compute_housing_capacity(
                &world.entities, registry,
            );

            println!("{:>6} {:>6} {:>8} {:>5}({:>2}) {:>10.0} {:>8} {:>6}",
                t, pop, housing, completed, constructing,
                world.treasury as f64 / 100.0, completed, road_tiles);

            if t == total_ticks {
                println!("{}", "-".repeat(68));
                println!("Final: pop={}, housing={}, buildings={} (+{})",
                    pop, housing, completed, constructing);
            }
        }
    }
}
