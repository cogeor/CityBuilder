use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct BuildingsPlugin;

impl SimulationPlugin for BuildingsPlugin {
    fn name(&self) -> &'static str { "buildings" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        crate::sim::systems::buildings::tick_zoned_development(
            w.world,
            w.registry,
            tick,
            w.rng,
        );
    }
}
