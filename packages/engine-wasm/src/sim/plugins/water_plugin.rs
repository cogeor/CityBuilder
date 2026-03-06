/// WaterPlugin is superseded by UtilityRegistry / WaterSystem.
/// Kept as a no-op stub so the module tree compiles without breakage.
use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct WaterPlugin;

impl SimulationPlugin for WaterPlugin {
    fn name(&self) -> &'static str { "water" }
    fn tick(&mut self, _w: &mut SimWorld<'_>, _tick: u64) {}
}
