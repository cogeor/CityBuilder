use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct WaterPlugin;

impl SimulationPlugin for WaterPlugin {
    fn name(&self) -> &'static str { "water" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        use crate::sim::phase_wheel::Phase;
        use crate::sim::systems::utilities::{compute_water_coverage, tick_water};

        // BFS spatial coverage — expensive, amortised to Utilities phase.
        if w.phase_wheel.should_run_expensive(tick, Phase::Utilities) {
            let state = compute_water_coverage(w.world, w.registry);
            *w.water_shortage    = state.deficit > 0;
            *w.water_shortage_kl = state.deficit;
        }

        // Demand/supply balance — runs every tick to keep HAS_WATER flags current.
        tick_water(
            &mut w.world.entities,
            w.registry,
            w.events,
            tick,
            *w.water_shortage,
        );
    }
}
