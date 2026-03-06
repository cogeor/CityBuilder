use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct JobsPlugin;

impl SimulationPlugin for JobsPlugin {
    fn name(&self) -> &'static str { "jobs" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        crate::sim::systems::jobs::tick_jobs(
            &mut w.world.entities,
            w.registry,
            w.events,
            tick,
            *w.population,
        );
    }
}
