use crate::sim::plugin::{SimulationPlugin, SimWorld};
use crate::sim::systems::buildings::{
    DevelopmentConfig, DevelopmentState, ZoneDemand,
    tick_zoned_development_with_config,
};

/// Plugin for zoned development. Owns `DevelopmentState` so stripe-walk
/// cursors persist between ticks.
#[derive(Debug)]
pub struct BuildingsPlugin {
    state: Option<DevelopmentState>,
}

impl BuildingsPlugin {
    pub fn new() -> Self {
        BuildingsPlugin { state: None }
    }
}

impl Default for BuildingsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulationPlugin for BuildingsPlugin {
    fn name(&self) -> &'static str { "buildings" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        let map_size = w.world.map_size();
        let state = self.state.get_or_insert_with(|| {
            DevelopmentState::new(map_size.width, map_size.height)
        });
        tick_zoned_development_with_config(
            w.world,
            w.registry,
            tick,
            w.rng,
            DevelopmentConfig::default(),
            state,
            ZoneDemand::FULL,
        );
    }
}
