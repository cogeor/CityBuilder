//! SimCorePlugin — initializes all simulation resources.

use city_core::{App, MapSize, Plugin};
use city_core::schedule::Schedule;
use city_engine::archetype::ArchetypeRegistry;

use crate::caches::analysis_maps::AnalysisMaps;
use crate::caches::cache_manager::CacheManager;
use crate::events::EventBus;
use crate::phase_wheel::PhaseWheel;
use crate::sim_map::SimMapRegistry;
use crate::systems::effects::EffectMap;
use crate::systems::sim_tick::{SimRunState, SimTickSystem};
use crate::world::WorldState;
use crate::world_vars::WorldVars;

/// Configuration for creating a new simulation world.
pub struct SimConfig {
    pub map_size: MapSize,
    pub seed: u64,
    pub city_name: String,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            map_size: MapSize::new(128, 128),
            seed: 42,
            city_name: "New Town".into(),
        }
    }
}

/// Plugin that initializes all core simulation resources.
///
/// Inserts WorldState, WorldVars, EventBus, CacheManager,
/// AnalysisMaps, SimMapRegistry, and PhaseWheel into the App.
pub struct SimCorePlugin {
    pub config: SimConfig,
}

impl SimCorePlugin {
    pub fn new(config: SimConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(SimConfig::default())
    }
}

impl Plugin for SimCorePlugin {
    fn build(&self, app: &mut App) {
        let size = self.config.map_size;
        let w = size.width;
        let h = size.height;

        let mut world = WorldState::new(size, self.config.seed);
        world.city_name = self.config.city_name.clone();

        app.insert_resource(ArchetypeRegistry::new());
        app.insert_resource(world);
        app.insert_resource(WorldVars::default());
        app.insert_resource(EventBus::new());
        app.insert_resource(CacheManager::new());
        app.insert_resource(AnalysisMaps::new(w, h));
        app.insert_resource(SimMapRegistry::new(w as usize, h as usize));
        app.insert_resource(EffectMap::new(w as u32, h as u32));
        app.insert_resource(PhaseWheel::new());
        app.insert_resource(SimRunState::new(self.config.seed));
        app.add_systems(Schedule::Tick, SimTickSystem);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::App;

    #[test]
    fn sim_core_plugin_inserts_all_resources() {
        let mut app = App::new();
        app.add_plugins(SimCorePlugin::with_defaults());

        assert!(app.get_resource::<WorldState>().is_some());
        assert!(app.get_resource::<WorldVars>().is_some());
        assert!(app.get_resource::<EventBus>().is_some());
        assert!(app.get_resource::<CacheManager>().is_some());
        assert!(app.get_resource::<AnalysisMaps>().is_some());
        assert!(app.get_resource::<SimMapRegistry>().is_some());
        assert!(app.get_resource::<PhaseWheel>().is_some());
    }

    #[test]
    fn sim_core_plugin_custom_config() {
        let mut app = App::new();
        app.add_plugins(SimCorePlugin::new(SimConfig {
            map_size: MapSize::new(64, 32),
            seed: 9999,
            city_name: "Test City".into(),
        }));

        let world = app.get_resource::<WorldState>().unwrap();
        assert_eq!(world.map_size(), MapSize::new(64, 32));
        assert_eq!(world.seeds.global_seed, 9999);
        assert_eq!(world.city_name, "Test City");
    }

    #[test]
    fn sim_core_plugin_resources_accessible_from_app() {
        let mut app = App::new();
        app.add_plugins(SimCorePlugin::with_defaults());

        // Verify resources are accessible mutably (as systems would use them)
        let world = app.get_resource_mut::<WorldState>().unwrap();
        assert_eq!(world.tick, 0);
        world.tick = 1;

        let world = app.get_resource::<WorldState>().unwrap();
        assert_eq!(world.tick, 1);
    }
}
