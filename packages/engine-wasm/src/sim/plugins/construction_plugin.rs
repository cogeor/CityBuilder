use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct ConstructionPlugin;

impl SimulationPlugin for ConstructionPlugin {
    fn name(&self) -> &'static str { "construction" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        crate::sim::systems::construction::tick_construction(
            &mut w.world.entities,
            w.registry,
            w.events,
            tick,
        );
    }
}
