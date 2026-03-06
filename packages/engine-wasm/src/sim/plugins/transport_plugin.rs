use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct TransportPlugin;

impl SimulationPlugin for TransportPlugin {
    fn name(&self) -> &'static str { "transport" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        let map_size = w.world.map_size();
        crate::sim::systems::transport::tick_transport(
            w.traffic_grid,
            &w.world.entities,
            w.registry,
            w.road_graph,
            &w.world.tiles,
            w.events,
            tick,
            map_size,
        );
    }
}
