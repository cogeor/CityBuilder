use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct PowerPlugin;

impl SimulationPlugin for PowerPlugin {
    fn name(&self) -> &'static str { "power" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        use crate::sim::phase_wheel::Phase;
        use crate::sim::systems::electricity::propagate_power;

        // BFS is expensive; only run on Utilities phase (every 4 ticks).
        if !w.phase_wheel.should_run_expensive(tick, Phase::Utilities) {
            return;
        }

        let state = propagate_power(w.world, w.registry);
        *w.power_shortage    = state.deficit_kw > 0;
        *w.power_shortage_kw = state.deficit_kw;
    }
}
