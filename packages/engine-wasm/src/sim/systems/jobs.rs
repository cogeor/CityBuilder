//! Jobs and industry system: employment matching and labor market simulation.
//!
//! Each tick, computes total job capacity from active Commercial/Industrial
//! buildings, matches workers from population, sets STAFFED flags, and emits
//! unemployment or labor shortage events.

use crate::core::archetypes::{ArchetypeRegistry, ArchetypeTag};
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent};
use crate::core_types::*;

/// Summary statistics for the job market after a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobStats {
    /// Total job slots available across all active Commercial/Industrial buildings.
    pub total_jobs: u32,
    /// Number of people currently employed (min of population, total_jobs).
    pub employed: u32,
    /// Number of people without jobs (population - employed, if population > total_jobs).
    pub unemployed: u32,
    /// Unmet labor demand (total_jobs - employed, if total_jobs > population).
    pub labor_demand: u32,
}

/// Compute total job capacity from active (non-under-construction) Commercial
/// and Industrial entities.
pub fn compute_job_capacity(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
) -> u32 {
    let mut total: u32 = 0;

    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) {
            Some(f) => f,
            None => continue,
        };

        // Skip entities still under construction.
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
            continue;
        }

        let arch_id = match entities.get_archetype(handle) {
            Some(id) => id,
            None => continue,
        };

        let def = match registry.get(arch_id) {
            Some(d) => d,
            None => continue,
        };

        // Only Commercial and Industrial buildings provide jobs.
        if def.has_tag(ArchetypeTag::Commercial) || def.has_tag(ArchetypeTag::Industrial) {
            total += def.job_capacity();
        }
    }

    total
}

/// Compute unemployment rate as a percentage (0-100).
/// Returns 0 if population is 0.
#[inline]
pub fn unemployment_rate(unemployed: u32, population: u32) -> u8 {
    if population == 0 {
        return 0;
    }
    ((unemployed as u64 * 100) / population as u64) as u8
}

/// Run the jobs system for one tick.
///
/// Computes job capacity, matches workers to jobs, sets STAFFED flags on
/// buildings that have enough workers, and emits events for unemployment
/// or labor shortages.
pub fn tick_jobs(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    population: u32,
) -> JobStats {
    let total_jobs = compute_job_capacity(entities, registry);

    let employed = population.min(total_jobs);
    let unemployed = population.saturating_sub(total_jobs);
    let labor_demand = total_jobs.saturating_sub(population);

    // Set STAFFED flag on active Commercial/Industrial entities.
    // Walk entities in order, marking STAFFED until we run out of available workers.
    let active_job_entities: Vec<(EntityHandle, u32)> = entities
        .iter_alive()
        .filter_map(|handle| {
            let flags = entities.get_flags(handle)?;
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
                return None;
            }
            let arch_id = entities.get_archetype(handle)?;
            let def = registry.get(arch_id)?;
            if def.has_tag(ArchetypeTag::Commercial) || def.has_tag(ArchetypeTag::Industrial) {
                Some((handle, def.job_capacity()))
            } else {
                None
            }
        })
        .collect();

    let mut remaining_workers = population;

    for (handle, capacity) in &active_job_entities {
        if let Some(flags) = entities.get_flags(*handle) {
            if remaining_workers >= *capacity && *capacity > 0 {
                remaining_workers -= *capacity;
                entities.set_flags(*handle, flags.insert(StatusFlags::STAFFED));
            } else {
                entities.set_flags(*handle, flags.remove(StatusFlags::STAFFED));
            }
        }
    }

    // Emit events.
    let rate = unemployment_rate(unemployed, population);
    if rate > 10 {
        events.publish(tick, SimEvent::UnemploymentHigh { rate_pct: rate });
    }

    if labor_demand > 0 {
        events.publish(tick, SimEvent::LaborShortage { deficit: labor_demand });
    }

    JobStats {
        total_jobs,
        employed,
        unemployed,
        labor_demand,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeTag};

    /// Helper: create an archetype with given tags and workspace_per_job_m2.
    fn make_archetype(
        id: ArchetypeId,
        tags: Vec<ArchetypeTag>,
        workspace_per_job_m2: u32,
    ) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Archetype {}", id),
            tags,
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 10_000,
            base_upkeep_cents_per_tick: 1,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 1,
            water_supply: 0,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 0,
            build_time_ticks: 100,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create a residential archetype (no jobs).
    fn make_residential(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Residential {}", id),
            tags: vec![ArchetypeTag::Residential],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 10_000,
            base_upkeep_cents_per_tick: 1,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 1,
            water_supply: 0,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 0,
            build_time_ticks: 100,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
        }
    }

    /// Helper: allocate an entity and clear UNDER_CONSTRUCTION to make it active.
    fn make_active_entity(
        entities: &mut EntityStore,
        arch_id: ArchetypeId,
        x: i16,
    ) -> EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    // ─── Test 1: Empty world returns zero stats ─────────────────────────────

    #[test]
    fn empty_world_returns_zero_stats() {
        let mut entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        let stats = tick_jobs(&mut entities, &registry, &mut events, 0, 0);

        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.employed, 0);
        assert_eq!(stats.unemployed, 0);
        assert_eq!(stats.labor_demand, 0);
        assert!(events.is_empty());
    }

    // ─── Test 2: Job capacity computed correctly ────────────────────────────

    #[test]
    fn job_capacity_computed_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        // Commercial shop: 1x1, 50% coverage, 2 floors = 256 m², /25 = 10 jobs
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));
        // Industrial: 1x1, 50% coverage, 2 floors = 256 m², /50 = 5 jobs
        registry.register(make_archetype(
            2,
            vec![ArchetypeTag::Industrial],
            50,
        ));

        make_active_entity(&mut entities, 1, 0); // 10 jobs
        make_active_entity(&mut entities, 2, 1); // 5 jobs

        let capacity = compute_job_capacity(&entities, &registry);
        assert_eq!(capacity, 15);
    }

    // ─── Test 3: Full employment when jobs <= population ────────────────────

    #[test]
    fn full_employment_when_jobs_lte_population() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs available
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));
        make_active_entity(&mut entities, 1, 0);

        // Population of 100 (more than 10 jobs)
        let stats = tick_jobs(&mut entities, &registry, &mut events, 0, 100);

        assert_eq!(stats.total_jobs, 10);
        assert_eq!(stats.employed, 10);
        assert_eq!(stats.unemployed, 90);
        assert_eq!(stats.labor_demand, 0);
    }

    // ─── Test 4: Unemployment when population > jobs ────────────────────────

    #[test]
    fn unemployment_when_population_exceeds_jobs() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs available
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));
        make_active_entity(&mut entities, 1, 0);

        let stats = tick_jobs(&mut entities, &registry, &mut events, 0, 50);

        assert_eq!(stats.total_jobs, 10);
        assert_eq!(stats.employed, 10);
        assert_eq!(stats.unemployed, 40);
        assert_eq!(stats.labor_demand, 0);
    }

    // ─── Test 5: Labor shortage when jobs > population ──────────────────────

    #[test]
    fn labor_shortage_when_jobs_exceed_population() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs available
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));
        make_active_entity(&mut entities, 1, 0);

        let stats = tick_jobs(&mut entities, &registry, &mut events, 0, 5);

        assert_eq!(stats.total_jobs, 10);
        assert_eq!(stats.employed, 5);
        assert_eq!(stats.unemployed, 0);
        assert_eq!(stats.labor_demand, 5);
    }

    // ─── Test 6: Under-construction excluded from job capacity ──────────────

    #[test]
    fn under_construction_excluded_from_job_capacity() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs per building
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));

        // One active building (10 jobs)
        make_active_entity(&mut entities, 1, 0);
        // One under construction (should NOT count)
        let _uc = entities.alloc(1, 1, 0, 0).unwrap();
        // alloc sets UNDER_CONSTRUCTION by default, don't clear it

        let stats = tick_jobs(&mut entities, &registry, &mut events, 0, 5);

        // Only the active building's jobs should count.
        assert_eq!(stats.total_jobs, 10);
    }

    // ─── Test 7: STAFFED flag set correctly ─────────────────────────────────

    #[test]
    fn staffed_flag_set_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs per building
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));

        let h1 = make_active_entity(&mut entities, 1, 0); // 10 jobs
        let h2 = make_active_entity(&mut entities, 1, 1); // 10 jobs

        // Population of 15: enough for first building (10), not second (10)
        tick_jobs(&mut entities, &registry, &mut events, 0, 15);

        let flags1 = entities.get_flags(h1).unwrap();
        let flags2 = entities.get_flags(h2).unwrap();

        // First building should be staffed, second should not.
        assert!(flags1.contains(StatusFlags::STAFFED));
        assert!(!flags2.contains(StatusFlags::STAFFED));
    }

    // ─── Test 8: Events emitted correctly ───────────────────────────────────

    #[test]
    fn unemployment_high_event_emitted() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));
        make_active_entity(&mut entities, 1, 0);

        // Population of 100, only 10 jobs -> 90% unemployment (> 10%)
        tick_jobs(&mut entities, &registry, &mut events, 42, 100);

        let drained = events.drain();
        let unemployment_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::UnemploymentHigh { .. }))
            .collect();
        assert_eq!(unemployment_events.len(), 1);

        if let SimEvent::UnemploymentHigh { rate_pct } = &unemployment_events[0].event {
            assert_eq!(*rate_pct, 90);
        } else {
            panic!("Expected UnemploymentHigh event");
        }
        assert_eq!(unemployment_events[0].tick, 42);
    }

    #[test]
    fn labor_shortage_event_emitted() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));
        make_active_entity(&mut entities, 1, 0);

        // Population of 3 -> 7 job shortage
        tick_jobs(&mut entities, &registry, &mut events, 10, 3);

        let drained = events.drain();
        let shortage_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::LaborShortage { .. }))
            .collect();
        assert_eq!(shortage_events.len(), 1);

        if let SimEvent::LaborShortage { deficit } = &shortage_events[0].event {
            assert_eq!(*deficit, 7);
        } else {
            panic!("Expected LaborShortage event");
        }
    }

    // ─── Additional tests for edge cases ────────────────────────────────────

    #[test]
    fn residential_buildings_provide_no_jobs() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        registry.register(make_residential(1));
        make_active_entity(&mut entities, 1, 0);

        let capacity = compute_job_capacity(&entities, &registry);
        assert_eq!(capacity, 0);
    }

    #[test]
    fn unemployment_rate_zero_when_population_zero() {
        assert_eq!(unemployment_rate(0, 0), 0);
    }

    #[test]
    fn unemployment_rate_calculation() {
        assert_eq!(unemployment_rate(10, 100), 10);
        assert_eq!(unemployment_rate(50, 100), 50);
        assert_eq!(unemployment_rate(0, 100), 0);
        assert_eq!(unemployment_rate(100, 100), 100);
    }

    #[test]
    fn no_events_when_unemployment_at_or_below_threshold() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 9 jobs
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            // 1x1, 50%, 2 floors = 256 m². 256/28 = 9 jobs
            28,
        ));
        make_active_entity(&mut entities, 1, 0);

        // Population = 10, jobs = 9, unemployment = 1/10 = 10% (not > 10%)
        tick_jobs(&mut entities, &registry, &mut events, 0, 10);

        let drained = events.drain();
        let unemployment_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::UnemploymentHigh { .. }))
            .collect();
        assert_eq!(unemployment_events.len(), 0);
    }

    #[test]
    fn all_buildings_staffed_when_population_is_sufficient() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // 10 jobs per building
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Commercial],
            25,
        ));

        let h1 = make_active_entity(&mut entities, 1, 0);
        let h2 = make_active_entity(&mut entities, 1, 1);
        let h3 = make_active_entity(&mut entities, 1, 2);

        // Population of 100, way more than 30 total jobs
        tick_jobs(&mut entities, &registry, &mut events, 0, 100);

        assert!(entities.get_flags(h1).unwrap().contains(StatusFlags::STAFFED));
        assert!(entities.get_flags(h2).unwrap().contains(StatusFlags::STAFFED));
        assert!(entities.get_flags(h3).unwrap().contains(StatusFlags::STAFFED));
    }
}
