use crate::sim::plugin::{SimulationPlugin, SimWorld};
use crate::sim::systems::buildings::{
    compute_zone_demand, DevelopmentConfig, DevelopmentState,
    tick_zoned_development_with_config,
};

/// Plugin for zoned development. Owns `DevelopmentState` so stripe-walk
/// cursors persist between ticks, and computes ZoneDemand each tick from
/// housing capacity and tax rates (wires GROWTH_MODIFIERS from base.economy).
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

        // Compute demand signals from housing surplus/deficit and tax policy.
        // This wires the GROWTH_MODIFIERS table from base.economy into the Rust
        // simulation: tax_rate modifier maps to demand offset in [-25..+25].
        let demand = compute_zone_demand(w.world, w.registry, *w.population);

        tick_zoned_development_with_config(
            w.world,
            w.registry,
            tick,
            w.rng,
            DevelopmentConfig::default(),
            state,
            demand,
            Some((w.analysis_maps, w.city_center)),
        );
    }
}
