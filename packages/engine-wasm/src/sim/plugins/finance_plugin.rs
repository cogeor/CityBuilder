use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct FinancePlugin;

impl SimulationPlugin for FinancePlugin {
    fn name(&self) -> &'static str { "finance" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        crate::sim::systems::finance::tick_finance(
            &w.world.entities,
            w.registry,
            w.events,
            tick,
            &w.world.policies,
            &mut w.world.treasury,
            *w.population,
        );
    }
}
