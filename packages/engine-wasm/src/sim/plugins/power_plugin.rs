/// PowerPlugin is superseded by UtilityRegistry / ElectricitySystem.
/// Kept as a no-op stub so the module tree compiles without breakage.
use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct PowerPlugin;

impl SimulationPlugin for PowerPlugin {
    fn name(&self) -> &'static str { "power" }
    fn tick(&mut self, _w: &mut SimWorld<'_>, _tick: u64) {}
}
