//! Construction system: advances building construction progress.
//!
//! Entities start with `UNDER_CONSTRUCTION` flag and `construction_progress = 0`.
//! Each tick, progress advances by `0xFFFF / build_time_ticks`.
//! When progress reaches `0xFFFF`, the entity transitions to active.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent};
use crate::core_types::*;

/// Default build time in ticks when archetype is not found in the registry.
const DEFAULT_BUILD_TIME_TICKS: u32 = 1000;

// ─── ConstructionPhase ────────────────────────────────────────────────────────

/// Visual milestone bands for a building under construction.
///
/// Divides the u16 progress range (0x0000..0xFFFF) into five equal bands.
/// Used by the renderer to select the appropriate sprite per construction stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionPhase {
    /// 0x0000..0x3FFF — Foundation poured.
    Foundation,
    /// 0x4000..0x7FFF — Frame erected.
    Framing,
    /// 0x8000..0xBFFF — Roof installed.
    Roofed,
    /// 0xC000..0xFFFE — Interior finishing.
    Finishing,
    /// 0xFFFF — Construction complete; entity is active.
    Complete,
}

/// Map a u16 construction progress value to the corresponding `ConstructionPhase`.
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

    // Collect handles to avoid aliasing borrow issues.
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

        // Look up build time from archetype definition.
        let build_time = match registry.get(archetype_id) {
            Some(def) => def.build_time_ticks.max(1),
            None => DEFAULT_BUILD_TIME_TICKS,
        };

        // Progress increment per tick (ceiling division ensures completion
        // in exactly build_time ticks rather than build_time + 1).
        let increment = ((0xFFFFu32 + build_time - 1) / build_time).max(1) as u16;

        let new_progress = (current_progress as u32 + increment as u32).min(0xFFFF) as u16;

        if new_progress >= 0xFFFF {
            // Construction complete.
            entities.set_construction_progress(handle, 0xFFFF);

            // Remove UNDER_CONSTRUCTION flag.
            if let Some(mut flags) = entities.get_flags(handle) {
                flags.remove(StatusFlags::UNDER_CONSTRUCTION);
                entities.set_flags(handle, flags);
            }

            events.publish(
                tick,
                SimEvent::BuildingCompleted {
                    handle,
                    archetype: archetype_id,
                },
            );

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
    if progress >= 0xFFFF {
        return Some(0);
    }
    let archetype_id = entities.get_archetype(handle)?;
    let build_time = match registry.get(archetype_id) {
        Some(def) => def.build_time_ticks.max(1),
        None => DEFAULT_BUILD_TIME_TICKS,
    };
    let increment_per_tick = ((0xFFFFu32 + build_time - 1) / build_time).max(1);
    let remaining_progress = 0xFFFFu32 - progress as u32;
    Some((remaining_progress + increment_per_tick - 1) / increment_per_tick)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeTag};

    fn make_test_archetype(id: ArchetypeId, build_time: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Test Building {}", id),
            tags: vec![ArchetypeTag::Residential],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 1,
            usable_ratio_pct: 80,
            base_cost_cents: 10_000,
            base_upkeep_cents_per_tick: 1,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 1,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 0,
            build_time_ticks: build_time,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
            effects: vec![],
        }
    }

    fn setup(
        build_time: u32,
    ) -> (EntityStore, ArchetypeRegistry, EventBus, EntityHandle) {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let events = EventBus::new();

        registry.register(make_test_archetype(1, build_time));

        let handle = entities.alloc(1, 5, 5, 0).unwrap();
        // Entity starts with UNDER_CONSTRUCTION flag and progress=0

        (entities, registry, events, handle)
    }

    #[test]
    fn progress_advances_each_tick() {
        let (mut entities, registry, mut events, handle) = setup(100);

        tick_construction(&mut entities, &registry, &mut events, 0);

        let progress = entities.get_construction_progress(handle).unwrap();
        // increment = ceil(0xFFFF / 100) = 656
        assert_eq!(progress, 656);
    }

    #[test]
    fn progress_accumulates_over_ticks() {
        let (mut entities, registry, mut events, handle) = setup(100);

        for tick in 0..10 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }

        let progress = entities.get_construction_progress(handle).unwrap();
        // 10 ticks * 656 = 6560
        assert_eq!(progress, 6560);
    }

    #[test]
    fn completes_after_build_time_ticks() {
        let build_time = 50;
        let (mut entities, registry, mut events, handle) = setup(build_time);

        for tick in 0..build_time as u64 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }

        let progress = entities.get_construction_progress(handle).unwrap();
        assert_eq!(progress, 0xFFFF);

        // UNDER_CONSTRUCTION flag should be removed
        let flags = entities.get_flags(handle).unwrap();
        assert!(!flags.contains(StatusFlags::UNDER_CONSTRUCTION));
    }

    #[test]
    fn emits_building_completed_event() {
        let build_time = 10;
        let (mut entities, registry, mut events, handle) = setup(build_time);

        // Run until completion
        for tick in 0..build_time as u64 + 5 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }

        let drained = events.drain();
        let completed_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::BuildingCompleted { .. }))
            .collect();
        assert_eq!(completed_events.len(), 1);

        if let SimEvent::BuildingCompleted {
            handle: h,
            archetype,
        } = &completed_events[0].event
        {
            assert_eq!(*h, handle);
            assert_eq!(*archetype, 1);
        } else {
            panic!("Expected BuildingCompleted event");
        }
    }

    #[test]
    fn already_complete_entity_not_affected() {
        let (mut entities, registry, mut events, handle) = setup(100);

        // Mark as already complete
        entities.set_construction_progress(handle, 0xFFFF);
        entities.set_flags(handle, StatusFlags::NONE);

        let completed = tick_construction(&mut entities, &registry, &mut events, 0);
        assert_eq!(completed, 0);
        assert!(events.is_empty());
    }

    #[test]
    fn multiple_entities_progress_independently() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_test_archetype(1, 100)); // slow build
        registry.register(make_test_archetype(2, 10)); // fast build

        let h1 = entities.alloc(1, 0, 0, 0).unwrap(); // slow
        let h2 = entities.alloc(2, 1, 0, 0).unwrap(); // fast

        // Run 10 ticks — h2 should complete, h1 should not
        for tick in 0..10 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }

        // h2 should be complete
        assert_eq!(
            entities.get_construction_progress(h2).unwrap(),
            0xFFFF
        );
        assert!(
            !entities
                .get_flags(h2)
                .unwrap()
                .contains(StatusFlags::UNDER_CONSTRUCTION)
        );

        // h1 should still be under construction
        assert!(entities
            .get_flags(h1)
            .unwrap()
            .contains(StatusFlags::UNDER_CONSTRUCTION));
        assert!(entities.get_construction_progress(h1).unwrap() < 0xFFFF);
    }

    #[test]
    fn returns_completed_count() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_test_archetype(1, 1)); // completes in 1 tick

        entities.alloc(1, 0, 0, 0).unwrap();
        entities.alloc(1, 1, 0, 0).unwrap();
        entities.alloc(1, 2, 0, 0).unwrap();

        let completed = tick_construction(&mut entities, &registry, &mut events, 0);
        assert_eq!(completed, 3);
    }

    #[test]
    fn unknown_archetype_uses_default_build_time() {
        let mut entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new(); // empty registry
        let mut events = EventBus::new();

        let handle = entities.alloc(999, 0, 0, 0).unwrap();

        tick_construction(&mut entities, &registry, &mut events, 0);

        // Default build time = 1000, increment = ceil(0xFFFF / 1000) = 66
        let progress = entities.get_construction_progress(handle).unwrap();
        assert_eq!(progress, 66);
    }

    #[test]
    fn build_time_zero_treated_as_one() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_test_archetype(1, 0)); // zero build time

        let handle = entities.alloc(1, 0, 0, 0).unwrap();

        let completed = tick_construction(&mut entities, &registry, &mut events, 0);
        assert_eq!(completed, 1);
        assert_eq!(
            entities.get_construction_progress(handle).unwrap(),
            0xFFFF
        );
    }

    #[test]
    fn ticks_remaining_for_new_entity() {
        let (entities, registry, _, handle) = setup(100);

        let remaining = ticks_remaining(&entities, &registry, handle).unwrap();
        // 0xFFFF / (0xFFFF / 100) = 0xFFFF / 655 = 100 (ceiling division)
        assert_eq!(remaining, 100);
    }

    #[test]
    fn ticks_remaining_for_completed_entity() {
        let (mut entities, registry, _, handle) = setup(100);
        entities.set_construction_progress(handle, 0xFFFF);

        let remaining = ticks_remaining(&entities, &registry, handle).unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn ticks_remaining_after_partial_progress() {
        let (mut entities, registry, mut events, handle) = setup(100);

        // Run 50 ticks
        for tick in 0..50 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }

        let remaining = ticks_remaining(&entities, &registry, handle).unwrap();
        assert_eq!(remaining, 50);
    }

    #[test]
    fn ticks_remaining_invalid_handle() {
        let (entities, registry, _, _) = setup(100);
        assert!(ticks_remaining(&entities, &registry, EntityHandle::INVALID).is_none());
    }

    // ── ConstructionPhase boundaries ─────────────────────────────────────

    #[test]
    fn construction_phase_foundation_band() {
        assert_eq!(construction_phase(0x0000), ConstructionPhase::Foundation);
        assert_eq!(construction_phase(0x1000), ConstructionPhase::Foundation);
        assert_eq!(construction_phase(0x3FFF), ConstructionPhase::Foundation);
    }

    #[test]
    fn construction_phase_framing_band() {
        assert_eq!(construction_phase(0x4000), ConstructionPhase::Framing);
        assert_eq!(construction_phase(0x5000), ConstructionPhase::Framing);
        assert_eq!(construction_phase(0x7FFF), ConstructionPhase::Framing);
    }

    #[test]
    fn construction_phase_roofed_band() {
        assert_eq!(construction_phase(0x8000), ConstructionPhase::Roofed);
        assert_eq!(construction_phase(0xAFFF), ConstructionPhase::Roofed);
        assert_eq!(construction_phase(0xBFFF), ConstructionPhase::Roofed);
    }

    #[test]
    fn construction_phase_finishing_band() {
        assert_eq!(construction_phase(0xC000), ConstructionPhase::Finishing);
        assert_eq!(construction_phase(0xEFFF), ConstructionPhase::Finishing);
        assert_eq!(construction_phase(0xFFFE), ConstructionPhase::Finishing);
    }

    #[test]
    fn construction_phase_complete() {
        assert_eq!(construction_phase(0xFFFF), ConstructionPhase::Complete);
    }

    #[test]
    fn disabled_entity_still_constructs() {
        let (mut entities, registry, mut events, handle) = setup(10);
        entities.set_enabled(handle, false);

        for tick in 0..10 {
            tick_construction(&mut entities, &registry, &mut events, tick);
        }

        // Construction still completes (disabled doesn't pause construction)
        assert_eq!(
            entities.get_construction_progress(handle).unwrap(),
            0xFFFF
        );
    }
}
