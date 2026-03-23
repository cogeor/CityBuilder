//! UtilityRegistry — uniform dispatch for all utility systems.

use city_core::{TileCoord, Tick};
use crate::archetype::ArchetypeRegistry;

use crate::events::EventBus;
use crate::world::WorldState;
use crate::systems::utility_system::{UtilityBalance, UtilitySystem};

/// Registry that owns and drives all utility simulation systems.
#[derive(Debug)]
pub struct UtilityRegistry {
    systems: Vec<Box<dyn UtilitySystem>>,
    prev_shortages: Vec<bool>,
}

impl UtilityRegistry {
    pub fn new() -> Self {
        UtilityRegistry {
            systems: Vec::new(),
            prev_shortages: Vec::new(),
        }
    }

    pub fn register<U: UtilitySystem + 'static>(&mut self, system: U) {
        self.prev_shortages.push(false);
        self.systems.push(Box::new(system));
    }

    pub fn update_all(
        &mut self,
        world: &mut WorldState,
        registry: &ArchetypeRegistry,
        events: &mut EventBus,
        tick: Tick,
    ) {
        for (i, system) in self.systems.iter_mut().enumerate() {
            let prev = self.prev_shortages.get(i).copied().unwrap_or(false);
            let balance = system.update(world, registry, events, tick, prev);
            if let Some(prev_ref) = self.prev_shortages.get_mut(i) {
                *prev_ref = balance.has_shortage();
            }
        }
    }

    pub fn is_served(&self, pos: TileCoord, utility_name: &str) -> bool {
        self.systems.iter()
            .find(|s| s.name() == utility_name)
            .map(|s| s.tile_served(pos))
            .unwrap_or(false)
    }

    pub fn metrics(&self, utility_name: &str) -> Option<UtilityBalance> {
        self.systems.iter()
            .find(|s| s.name() == utility_name)
            .map(|s| s.metrics_snapshot())
    }

    pub fn all_metrics(&self) -> Vec<(&str, UtilityBalance)> {
        self.systems.iter().map(|s| (s.name(), s.metrics_snapshot())).collect()
    }

    pub fn has_shortage(&self, utility_name: &str) -> bool {
        self.systems.iter()
            .find(|s| s.name() == utility_name)
            .map(|s| s.has_shortage())
            .unwrap_or(false)
    }

    pub fn deficit(&self, utility_name: &str) -> u32 {
        self.metrics(utility_name).map(|b| b.deficit()).unwrap_or(0)
    }
}

impl Default for UtilityRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::utility_system::{ElectricitySystem, WaterSystem};

    #[test]
    fn registry_starts_empty() {
        let reg = UtilityRegistry::new();
        assert!(reg.all_metrics().is_empty());
        assert!(reg.metrics("electricity").is_none());
    }

    #[test]
    fn registry_register_and_query() {
        let mut reg = UtilityRegistry::new();
        reg.register(ElectricitySystem::new());
        reg.register(WaterSystem::new());
        assert_eq!(reg.all_metrics().len(), 2);
        assert!(reg.metrics("electricity").is_some());
        assert!(reg.metrics("water").is_some());
        assert!(reg.metrics("unknown").is_none());
    }

    #[test]
    fn prev_shortages_len_matches_systems() {
        let mut reg = UtilityRegistry::new();
        reg.register(ElectricitySystem::new());
        assert_eq!(reg.prev_shortages.len(), 1);
        reg.register(WaterSystem::new());
        assert_eq!(reg.prev_shortages.len(), 2);
    }
}
