//! SimulationPlugin trait and SimWorld context.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::commands::Command;
use crate::core::events::EventBus;
use crate::core::network::RoadGraph;
use crate::core::world::WorldState;
use crate::math::rng::Rng;
use crate::sim::phase_wheel::PhaseWheel;
use crate::sim::systems::city_events::CityEventState;
use crate::sim::systems::transport::TrafficGrid;

/// A view of all mutable engine state that a plugin may read or write
/// during a single tick. Passed by `&mut` so plugins can mutate freely.
pub struct SimWorld<'a> {
    pub world: &'a mut WorldState,
    pub registry: &'a ArchetypeRegistry,
    pub events: &'a mut EventBus,
    pub road_graph: &'a RoadGraph,
    pub rng: &'a mut Rng,
    pub traffic_grid: &'a mut TrafficGrid,
    pub city_event_state: &'a mut CityEventState,
    /// Current population carried across ticks.
    pub population: &'a mut u32,
    /// Shortage flags from the previous tick.
    pub power_shortage: &'a mut bool,
    /// Unmet power demand in kW from the previous tick.
    pub power_shortage_kw: &'a mut u32,
    pub water_shortage: &'a mut bool,
    /// Unmet water demand in kL from the previous tick.
    pub water_shortage_kl: &'a mut u32,
    /// Phase wheel for gating expensive computation.
    pub phase_wheel: &'a PhaseWheel,
}

/// One pluggable simulation system.
///
/// The engine calls `tick` once per simulation tick (phase 3).
/// `on_command` is called for every command applied in phase 1 —
/// the default implementation does nothing, override when needed.
pub trait SimulationPlugin: std::fmt::Debug {
    fn name(&self) -> &'static str;

    fn tick(&mut self, world: &mut SimWorld<'_>, tick: u64);

    fn on_command(&mut self, _cmd: &Command, _world: &mut SimWorld<'_>) {}
}
