use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct PopulationPlugin;

impl SimulationPlugin for PopulationPlugin {
    fn name(&self) -> &'static str { "population" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        let stats = crate::sim::systems::population::tick_population(
            &w.world.entities,
            w.registry,
            w.events,
            tick,
            w.rng,
            *w.population,
        );
        *w.population = stats.total_population;
    }
}
