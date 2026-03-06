use crate::sim::plugin::{SimulationPlugin, SimWorld};

#[derive(Debug)]
pub struct CityEventsPlugin;

impl SimulationPlugin for CityEventsPlugin {
    fn name(&self) -> &'static str { "city_events" }

    fn tick(&mut self, w: &mut SimWorld<'_>, tick: u64) {
        crate::sim::systems::city_events::tick_city_events(
            w.city_event_state,
            &mut w.world.entities,
            w.events,
            w.rng,
            tick,
            &w.world.policies,
        );
    }
}
