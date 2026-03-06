//! `UtilityRegistry` — uniform dispatch for all utility systems.
//!
//! Owns a `Vec<Box<dyn UtilitySystem>>` and drives each system per tick.
//! Replaces the scalar `power_shortage` / `water_shortage` booleans on
//! `SimulationEngine` with an index-aligned `prev_shortages` vec.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::events::EventBus;
use crate::core::world::WorldState;
use crate::core_types::*;
use crate::sim::systems::utility_system::{UtilityBalance, UtilitySystem};

/// Registry that owns and drives all utility simulation systems.
#[derive(Debug)]
pub struct UtilityRegistry {
    systems: Vec<Box<dyn UtilitySystem>>,
    /// Shortage state from the previous tick, index-aligned with `systems`.
    prev_shortages: Vec<bool>,
}

impl UtilityRegistry {
    pub fn new() -> Self {
        UtilityRegistry {
            systems: Vec::new(),
            prev_shortages: Vec::new(),
        }
    }

    /// Register a new utility system.
    pub fn register<U: UtilitySystem + 'static>(&mut self, system: U) {
        self.prev_shortages.push(false);
        self.systems.push(Box::new(system));
    }

    /// Run all registered utility systems for the current tick.
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

    /// Query whether a tile position is served by the named utility system.
    pub fn is_served(&self, pos: TileCoord, utility_name: &str) -> bool {
        self.systems
            .iter()
            .find(|s| s.name() == utility_name)
            .map(|s| s.tile_served(pos))
            .unwrap_or(false)
    }

    /// Get the latest balance metrics for a named utility.
    pub fn metrics(&self, utility_name: &str) -> Option<UtilityBalance> {
        self.systems
            .iter()
            .find(|s| s.name() == utility_name)
            .map(|s| s.metrics_snapshot())
    }

    /// Get all system metrics as `(name, UtilityBalance)` pairs.
    pub fn all_metrics(&self) -> Vec<(&str, UtilityBalance)> {
        self.systems.iter().map(|s| (s.name(), s.metrics_snapshot())).collect()
    }

    /// Whether the named utility has an active shortage.
    pub fn has_shortage(&self, utility_name: &str) -> bool {
        self.systems
            .iter()
            .find(|s| s.name() == utility_name)
            .map(|s| s.has_shortage())
            .unwrap_or(false)
    }

    /// Unmet demand in raw units for the named utility (0 = no shortage).
    pub fn deficit(&self, utility_name: &str) -> u32 {
        self.metrics(utility_name)
            .map(|b| b.deficit())
            .unwrap_or(0)
    }
}

impl Default for UtilityRegistry {
    fn default() -> Self { Self::new() }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::systems::utility_system::{ElectricitySystem, WaterSystem};

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
