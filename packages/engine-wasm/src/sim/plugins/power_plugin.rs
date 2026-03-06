use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct PowerPlugin;

impl SimulationPlugin for PowerPlugin {
    fn name(&self) -> &'static str { "power" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        let balance = crate::sim::systems::utilities::tick_power(
            &mut w.world.entities,
            w.registry,
            w.events,
            tick,
            *w.power_shortage,
        );
        *w.power_shortage = balance.has_shortage();
    }
}
