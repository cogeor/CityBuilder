//! Overlay pipeline: runs after effects propagation each relevant tick.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::entity::EntityStore;
use crate::core::tilemap::TileMap;
use crate::core::world_vars::WorldVars;
use crate::sim::sim_map::{SimMap, SimMapRegistry};
use crate::sim::systems::effects::EffectMap;
use crate::sim::systems::land_value::compute_anchor_land_value;
use crate::sim::systems::service_system::service_system;
use crate::sim::systems::traffic_system::traffic_system;
use crate::core::archetypes::EffectKind;

/// Normalise the EffectMap Pollution layer (i16) into SimMap::Pollution (f32 0..1).
fn bridge_pollution(effect_map: &EffectMap, maps: &mut SimMapRegistry) {
    maps.clear_next(SimMap::Pollution);
    let layer = &effect_map.maps[EffectKind::Pollution as usize];
    let next = maps.next_mut(SimMap::Pollution);
    for (i, &raw) in layer.iter().enumerate() {
        // Pollution values in EffectMap are negative (harmful); take absolute value.
        let normalised = (raw.unsigned_abs() as f32 / i16::MAX as f32).clamp(0.0, 1.0);
        next[i] = normalised;
    }
}

/// Run the full overlay pipeline: service → traffic → pollution bridge → land value → swap.
///
/// Called from `tick.rs` after `propagate_effects`. Contains all f32 logic so
/// tick.rs stays float-free (determinism guard).
pub fn run_overlay_pipeline(
    maps: &mut SimMapRegistry,
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    tile_map: &TileMap,
    effect_map: &EffectMap,
    world_vars: &WorldVars,
) {
    service_system(entities, registry, effect_map, world_vars, maps);
    traffic_system(entities, registry, tile_map, world_vars, maps);
    bridge_pollution(effect_map, maps);
    compute_anchor_land_value(maps, world_vars);
    maps.swap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::ArchetypeRegistry;
    use crate::core::entity::EntityStore;
    use crate::core::tilemap::TileMap;
    use crate::core::world_vars::WorldVars;
    use crate::sim::sim_map::{SimMap, SimMapRegistry};
    use crate::sim::systems::effects::EffectMap;

    #[test]
    fn pipeline_runs_without_panic_on_empty_world() {
        let mut maps = SimMapRegistry::new(4, 4);
        let entities = EntityStore::new(0);
        let registry = ArchetypeRegistry::new();
        let tile_map = TileMap::new(4, 4);
        let effect_map = EffectMap::new(4, 4);
        let world_vars = WorldVars::default();
        // Should not panic
        run_overlay_pipeline(&mut maps, &entities, &registry, &tile_map, &effect_map, &world_vars);
        // After pipeline + swap, LandValue current buffer should be all 0 (no services)
        for v in maps.current(SimMap::LandValue) {
            assert_eq!(*v, 0.0);
        }
    }
}
