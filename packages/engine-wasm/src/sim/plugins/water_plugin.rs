use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct WaterPlugin;

impl SimulationPlugin for WaterPlugin {
    fn name(&self) -> &'static str { "water" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        let balance = crate::sim::systems::utilities::tick_water(
            &mut w.world.entities,
            w.registry,
            w.events,
            tick,
            *w.water_shortage,
        );
        *w.water_shortage = balance.has_shortage();
    }
}
