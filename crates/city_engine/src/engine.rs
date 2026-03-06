//! Simulation engine — runs systems by schedule phase.
//!
//! This is the core tick loop, refactored to use plugin-registered systems
//! instead of a hardcoded system list.

use city_core::schedule::Schedule;
use city_core::system::{SimContext, SimSystem};
use city_core::resource::ResourceMap;
use city_core::Tick;
use city_core::App;
use std::collections::BTreeMap;

/// The simulation engine. Runs registered systems by schedule phase.
pub struct SimulationEngine {
    systems: BTreeMap<Schedule, Vec<Box<dyn SimSystem>>>,
    resources: ResourceMap,
    tick: Tick,
}

impl SimulationEngine {
    /// Build an engine from a configured App.
    pub fn from_app(mut app: App) -> Self {
        Self {
            systems: app.take_systems(),
            resources: app.resources,
            tick: 0,
        }
    }

    /// Run one simulation tick: execute all systems in schedule order.
    pub fn tick(&mut self) {
        self.tick += 1;

        // Execute systems in schedule order (BTreeMap is sorted by key).
        // We temporarily take the systems out to avoid borrow conflicts.
        let mut systems = std::mem::take(&mut self.systems);

        for (_schedule, phase_systems) in systems.iter_mut() {
            for system in phase_systems.iter_mut() {
                let mut ctx = SimContext {
                    tick: self.tick,
                    resources: &mut self.resources,
                };
                system.tick(&mut ctx);
            }
        }

        self.systems = systems;
    }

    /// Current tick number.
    pub fn current_tick(&self) -> Tick {
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::{App, Plugin};
    use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

    struct CounterSystem {
        counter: Arc<AtomicU32>,
    }

    impl SimSystem for CounterSystem {
        fn name(&self) -> &str { "counter" }
        fn tick(&mut self, _ctx: &mut SimContext) {
            self.counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct CounterPlugin {
        counter: Arc<AtomicU32>,
    }

    impl Plugin for CounterPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Schedule::Tick, CounterSystem {
                counter: self.counter.clone(),
            });
        }
    }

    #[test]
    fn engine_runs_systems() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut app = App::new();
        app.add_plugins(CounterPlugin { counter: counter.clone() });

        let mut engine = SimulationEngine::from_app(app);

        engine.tick();
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        engine.tick();
        engine.tick();
        assert_eq!(counter.load(Ordering::Relaxed), 3);
        assert_eq!(engine.current_tick(), 3);
    }
}
