//! Construction system: advances building construction progress.

use city_core::{EntityHandle, StatusFlags, Tick};
use crate::archetype::ArchetypeRegistry;
use city_engine::entity::EntityStore;

use crate::events::{EventBus, SimEvent};

const DEFAULT_BUILD_TIME_TICKS: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionPhase {
    Foundation,
    Framing,
    Roofed,
    Finishing,
    Complete,
}

pub fn construction_phase(progress: u16) -> ConstructionPhase {
    match progress {
        0x0000..=0x3FFF => ConstructionPhase::Foundation,
        0x4000..=0x7FFF => ConstructionPhase::Framing,
        0x8000..=0xBFFF => ConstructionPhase::Roofed,
        0xC000..=0xFFFE => ConstructionPhase::Finishing,
        0xFFFF          => ConstructionPhase::Complete,
    }
}

/// Advance construction for all entities under construction.
/// Returns the number of buildings completed this tick.
pub fn tick_construction(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
) -> u32 {
    let mut completed = 0;

    let under_construction: Vec<EntityHandle> = entities
        .iter_with_flags(StatusFlags::UNDER_CONSTRUCTION)
        .collect();

    for handle in under_construction {
        let archetype_id = match entities.get_archetype(handle) {
            Some(id) => id,
            None => continue,
        };
        let current_progress = match entities.get_construction_progress(handle) {
            Some(p) => p,
            None => continue,
        };
        let build_time = match registry.get(archetype_id) {
            Some(def) => def.build_time_ticks.max(1),
            None => DEFAULT_BUILD_TIME_TICKS,
        };
        let increment = ((0xFFFFu32 + build_time - 1) / build_time).max(1) as u16;
        let new_progress = (current_progress as u32 + increment as u32).min(0xFFFF) as u16;

        if new_progress >= 0xFFFF {
            entities.set_construction_progress(handle, 0xFFFF);
            if let Some(mut flags) = entities.get_flags(handle) {
                flags.remove(StatusFlags::UNDER_CONSTRUCTION);
                entities.set_flags(handle, flags);
            }
            events.publish(tick, SimEvent::BuildingCompleted { handle, archetype: archetype_id });
            completed += 1;
        } else {
            entities.set_construction_progress(handle, new_progress);
        }
    }

    completed
}

/// Compute how many ticks remain for an entity's construction.
pub fn ticks_remaining(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    handle: EntityHandle,
) -> Option<u32> {
    let progress = entities.get_construction_progress(handle)?;
    if progress >= 0xFFFF { return Some(0); }
    let archetype_id = entities.get_archetype(handle)?;
    let build_time = match registry.get(archetype_id) {
        Some(def) => def.build_time_ticks.max(1),
        None => DEFAULT_BUILD_TIME_TICKS,
    };
    let increment_per_tick = ((0xFFFFu32 + build_time - 1) / build_time).max(1);
    let remaining_progress = 0xFFFFu32 - progress as u32;
    Some((remaining_progress + increment_per_tick - 1) / increment_per_tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::{ArchetypeDefinition, ArchetypeTag};

    fn make_test_arch(id: u16, build_time: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("Test {}", id),
            tags: vec![ArchetypeTag::Residential],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 1, usable_ratio_pct: 80,
            base_cost_cents: 10_000, base_upkeep_cents_per_tick: 1,
            power_demand_kw: 5, power_supply_kw: 0,
            water_demand: 1, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 0, noise: 0,
            build_time_ticks: build_time, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: 40,
            effects: vec![],
        }
    }

    fn setup(build_time: u32) -> (EntityStore, ArchetypeRegistry, EventBus, EntityHandle) {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let events = EventBus::new();
        registry.register(make_test_arch(1, build_time));
        let handle = entities.alloc(1, 5, 5, 0).unwrap();
        (entities, registry, events, handle)
    }

    #[test]
    fn progress_advances_each_tick() {
        let (mut entities, registry, mut events, handle) = setup(100);
        tick_construction(&mut entities, &registry, &mut events, 0);
        assert_eq!(entities.get_construction_progress(handle).unwrap(), 656);
    }

    #[test]
    fn completes_after_build_time_ticks() {
        let (mut entities, registry, mut events, handle) = setup(50);
        for tick in 0..50 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }
        assert_eq!(entities.get_construction_progress(handle).unwrap(), 0xFFFF);
        assert!(!entities.get_flags(handle).unwrap().contains(StatusFlags::UNDER_CONSTRUCTION));
    }

    #[test]
    fn emits_building_completed_event() {
        let (mut entities, registry, mut events, _handle) = setup(10);
        for tick in 0..15 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }
        let drained = events.drain();
        let completed: Vec<_> = drained.iter()
            .filter(|e| matches!(e.event, SimEvent::BuildingCompleted { .. }))
            .collect();
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn multiple_entities_progress_independently() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        registry.register(make_test_arch(1, 100));
        registry.register(make_test_arch(2, 10));
        let h1 = entities.alloc(1, 0, 0, 0).unwrap();
        let h2 = entities.alloc(2, 1, 0, 0).unwrap();

        for tick in 0..10 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }
        assert_eq!(entities.get_construction_progress(h2).unwrap(), 0xFFFF);
        assert!(entities.get_flags(h1).unwrap().contains(StatusFlags::UNDER_CONSTRUCTION));
    }

    #[test]
    fn ticks_remaining_for_new_entity() {
        let (entities, registry, _, handle) = setup(100);
        assert_eq!(ticks_remaining(&entities, &registry, handle).unwrap(), 100);
    }

    #[test]
    fn construction_phase_bands() {
        assert_eq!(construction_phase(0x0000), ConstructionPhase::Foundation);
        assert_eq!(construction_phase(0x4000), ConstructionPhase::Framing);
        assert_eq!(construction_phase(0x8000), ConstructionPhase::Roofed);
        assert_eq!(construction_phase(0xC000), ConstructionPhase::Finishing);
        assert_eq!(construction_phase(0xFFFF), ConstructionPhase::Complete);
    }
}
