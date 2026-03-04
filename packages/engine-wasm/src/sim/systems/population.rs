//! Population and housing system.
//!
//! Tracks total population, housing capacity, and migration.
//! Each tick, computes available housing from residential archetypes,
//! then applies migration logic: surplus housing attracts migrants,
//! deficit causes population loss. Emits events for UI notifications.

use crate::core::archetypes::{ArchetypeRegistry, ArchetypeTag};
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent};
use crate::core_types::*;
use crate::math::rng::Rng;

/// Aggregated population statistics for a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationStats {
    pub total_population: u32,
    pub total_housing_capacity: u32,
    pub occupied_housing: u32,
}

/// Maximum migrants attracted per tick.
const MAX_INCOMING_PER_TICK: i32 = 5;

/// Maximum emigrants lost per tick.
const MAX_OUTGOING_PER_TICK: i32 = 3;

/// Compute total housing capacity from all active residential entities.
///
/// Only entities that are alive, not under construction, and tagged
/// Residential contribute to capacity.
pub fn compute_housing_capacity(
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

        if def.has_tag(ArchetypeTag::Residential) {
            total += def.resident_capacity();
        }
    }

    total
}

/// Compute a simple desirability score for a single entity.
///
/// Base score is 50. +10 if the entity has power, +10 if it has water.
/// Returns a u8 in the range 0..=100.
pub fn compute_desirability_score(
    entities: &EntityStore,
    handle: EntityHandle,
) -> u8 {
    let flags = match entities.get_flags(handle) {
        Some(f) => f,
        None => return 0,
    };

    let mut score: u8 = 50;

    if flags.contains(StatusFlags::POWERED) {
        score += 10;
    }
    if flags.contains(StatusFlags::HAS_WATER) {
        score += 10;
    }

    score
}

/// Run one tick of the population system.
///
/// Computes housing capacity, applies migration logic with random jitter,
/// emits events, and returns aggregated stats.
pub fn tick_population(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    rng: &mut Rng,
    current_population: u32,
) -> PopulationStats {
    let capacity = compute_housing_capacity(entities, registry);

    // Compute base migration.
    let base_migration: i32 = if capacity > current_population {
        let surplus = (capacity - current_population) as i32;
        (surplus / 10).min(MAX_INCOMING_PER_TICK)
    } else if current_population > capacity {
        let deficit = (current_population - capacity) as i32;
        -(deficit / 10).min(MAX_OUTGOING_PER_TICK)
    } else {
        0
    };

    // Add small random jitter.
    let jitter = rng.range_inclusive(-1, 1);
    let net_migration = base_migration + jitter;

    // Compute new population, clamped to zero.
    let new_population = if net_migration < 0 {
        current_population.saturating_sub(net_migration.unsigned_abs())
    } else {
        current_population + net_migration as u32
    };

    // Emit PopulationChanged if the population actually changed.
    if new_population != current_population {
        events.publish(
            tick,
            SimEvent::PopulationChanged {
                old: current_population,
                new: new_population,
            },
        );
    }

    // Emit HousingShortage if population exceeds capacity.
    if new_population > capacity {
        events.publish(
            tick,
            SimEvent::HousingShortage {
                deficit: new_population - capacity,
            },
        );
    }

    // Emit MigrationWave if |net_migration| > 2.
    if net_migration.abs() > 2 {
        events.publish(
            tick,
            SimEvent::MigrationWave {
                incoming: net_migration,
            },
        );
    }

    let occupied = new_population.min(capacity);

    PopulationStats {
        total_population: new_population,
        total_housing_capacity: capacity,
        occupied_housing: occupied,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::ArchetypeDefinition;

    /// Helper: create a residential archetype with a given resident capacity.
    fn make_residential(id: ArchetypeId, living_space_m2: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("House {}", id),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 100_000,
            base_upkeep_cents_per_tick: 10,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 2,
            water_supply: 0,
            service_radius: 0,
            desirability_radius: 2,
            desirability_magnitude: 5,
            pollution: 0,
            noise: 1,
            build_time_ticks: 500,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: living_space_m2,
        }
    }

    /// Helper: create a non-residential (commercial) archetype.
    fn make_commercial(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Shop {}", id),
            tags: vec![ArchetypeTag::Commercial],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 1,
            usable_ratio_pct: 80,
            base_cost_cents: 80_000,
            base_upkeep_cents_per_tick: 15,
            power_demand_kw: 10,
            power_supply_kw: 0,
            water_demand: 3,
            water_supply: 0,
            service_radius: 0,
            desirability_radius: 3,
            desirability_magnitude: 3,
            pollution: 0,
            noise: 2,
            build_time_ticks: 300,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 25,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create an active (not under construction) entity.
    fn make_active(entities: &mut EntityStore, arch_id: ArchetypeId, x: i16) -> EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    // ─── Test 1: Empty world returns zero stats ─────────────────────────────

    #[test]
    fn empty_world_returns_zero_stats() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let mut rng = Rng::new(42);

        let stats = tick_population(&entities, &registry, &mut events, 0, &mut rng, 0);

        assert_eq!(stats.total_population, 0);
        assert_eq!(stats.total_housing_capacity, 0);
        assert_eq!(stats.occupied_housing, 0);
    }

    // ─── Test 2: Housing capacity computed correctly ────────────────────────

    #[test]
    fn housing_capacity_computed_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        // living_space_per_person_m2 = 40
        // gross_floor_area = 1*1*256 * 50/100 * 2 = 256 m2
        // resident_capacity = 256 / 40 = 6
        registry.register(make_residential(1, 40));
        registry.register(make_commercial(2));

        // Two active residential buildings.
        make_active(&mut entities, 1, 0);
        make_active(&mut entities, 1, 1);
        // One commercial building (should not contribute).
        make_active(&mut entities, 2, 2);

        let capacity = compute_housing_capacity(&entities, &registry);
        assert_eq!(capacity, 12); // 6 * 2
    }

    // ─── Test 3: Population grows with surplus housing ──────────────────────

    #[test]
    fn population_grows_with_surplus_housing() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // Each house has capacity 6, we create 10 houses = 60 capacity.
        registry.register(make_residential(1, 40));
        for x in 0..10 {
            make_active(&mut entities, 1, x);
        }

        // Run multiple ticks with low population to see growth.
        let mut pop = 0u32;
        for tick in 0..20 {
            let mut rng = Rng::new(100 + tick); // deterministic per-tick
            let stats = tick_population(&entities, &registry, &mut events, tick, &mut rng, pop);
            pop = stats.total_population;
        }

        // Population should have grown from 0.
        assert!(pop > 0, "Population should grow with surplus housing, got {}", pop);
    }

    // ─── Test 4: Population declines with housing deficit ───────────────────

    #[test]
    fn population_declines_with_housing_deficit() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // No housing, but start with population of 100.
        let mut pop = 100u32;
        for tick in 0..20 {
            let mut rng = Rng::new(200 + tick);
            let stats = tick_population(&entities, &registry, &mut events, tick, &mut rng, pop);
            pop = stats.total_population;
        }

        // Population should have declined.
        assert!(pop < 100, "Population should decline with no housing, got {}", pop);
    }

    // ─── Test 5: Under-construction buildings excluded from capacity ────────

    #[test]
    fn under_construction_excluded_from_capacity() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        registry.register(make_residential(1, 40));

        // One active building.
        make_active(&mut entities, 1, 0);
        // One under construction (default state from alloc).
        let _h_uc = entities.alloc(1, 1, 0, 0).unwrap();

        let capacity = compute_housing_capacity(&entities, &registry);
        // Only the active building contributes (capacity = 6).
        assert_eq!(capacity, 6);
    }

    // ─── Test 6: Events emitted correctly ───────────────────────────────────

    #[test]
    fn events_emitted_correctly() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let mut rng = Rng::new(42);

        // No housing, population of 50.
        // Base migration = -(50/10).min(3) = -3
        // With jitter, net_migration could be -4, -3, or -2
        let stats = tick_population(&entities, &registry, &mut events, 1, &mut rng, 50);

        let drained = events.drain();

        // Should have PopulationChanged event since pop changed.
        let pop_changed: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::PopulationChanged { .. }))
            .collect();
        assert!(!pop_changed.is_empty(), "Should emit PopulationChanged");

        // Should have HousingShortage since population > 0 capacity.
        let shortage: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::HousingShortage { .. }))
            .collect();
        assert!(!shortage.is_empty(), "Should emit HousingShortage");

        // Population should be less than 50.
        assert!(stats.total_population < 50);
    }

    // ─── Test 7: Migration capped at limits ─────────────────────────────────

    #[test]
    fn migration_capped_at_limits() {
        let mut entities = EntityStore::new(128);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // Create massive housing surplus: 100 houses * 6 capacity = 600.
        registry.register(make_residential(1, 40));
        for x in 0..100 {
            make_active(&mut entities, 1, x as i16);
        }

        // Start with 0 population. (600 - 0) / 10 = 60, capped to 5.
        // With jitter [-1, 1], max is 6 per tick.
        let mut rng = Rng::new(999);
        let stats = tick_population(&entities, &registry, &mut events, 0, &mut rng, 0);

        // New population should be at most 6 (5 + 1 jitter).
        assert!(
            stats.total_population <= 6,
            "Migration should be capped, got {}",
            stats.total_population
        );
    }

    // ─── Test 8: Zero capacity zero growth ──────────────────────────────────

    #[test]
    fn zero_capacity_zero_population_minimal_change() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let mut rng = Rng::new(42);

        // No housing, no population. Base migration = 0, only jitter applies.
        let stats = tick_population(&entities, &registry, &mut events, 0, &mut rng, 0);

        // With 0 pop and 0 capacity, base migration is 0.
        // Jitter can be -1, 0, or 1. Since population can't go below 0,
        // result should be 0 or 1.
        assert!(
            stats.total_population <= 1,
            "Zero capacity should yield minimal growth, got {}",
            stats.total_population
        );
    }

    // ─── Test 9: Desirability score computation ─────────────────────────────

    #[test]
    fn desirability_score_base() {
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 0, 0, 0).unwrap();
        // Clear all flags (no power, no water, not under construction).
        entities.set_flags(h, StatusFlags::NONE);

        let score = compute_desirability_score(&entities, h);
        assert_eq!(score, 50);
    }

    #[test]
    fn desirability_score_with_power_and_water() {
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 0, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED | StatusFlags::HAS_WATER);

        let score = compute_desirability_score(&entities, h);
        assert_eq!(score, 70);
    }

    #[test]
    fn desirability_score_with_power_only() {
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 0, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED);

        let score = compute_desirability_score(&entities, h);
        assert_eq!(score, 60);
    }

    // ─── Test 10: Migration wave event emitted ──────────────────────────────

    #[test]
    fn migration_wave_emitted_for_large_migration() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        // Create large housing surplus.
        registry.register(make_residential(1, 40));
        for x in 0..50 {
            make_active(&mut entities, 1, x as i16);
        }

        // Try multiple seeds to find one where |net_migration| > 2.
        let mut found_wave = false;
        for seed in 0..100u64 {
            let mut events = EventBus::new();
            let mut rng = Rng::new(seed);
            tick_population(&entities, &registry, &mut events, 0, &mut rng, 0);

            let drained = events.drain();
            if drained.iter().any(|e| matches!(e.event, SimEvent::MigrationWave { .. })) {
                found_wave = true;
                break;
            }
        }

        assert!(found_wave, "MigrationWave should be emitted when |net_migration| > 2");
    }

    // ─── Test 11: Population cannot go below zero ───────────────────────────

    #[test]
    fn population_cannot_go_below_zero() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();

        // Start with very small population and no housing.
        // Over many ticks, population should never go negative (u32 underflow).
        let mut pop = 1u32;
        for tick in 0..50 {
            let mut events = EventBus::new();
            let mut rng = Rng::new(300 + tick);
            let stats = tick_population(&entities, &registry, &mut events, tick, &mut rng, pop);
            pop = stats.total_population;
            // If population hit 0, it should stay there.
        }
        // pop is u32, so it cannot be negative. Just verify the test ran
        // without panicking and pop is small.
        assert!(pop <= 1, "Population should converge to 0, got {}", pop);
    }

    // ─── Test 12: Occupied housing capped at capacity ───────────────────────

    #[test]
    fn occupied_housing_capped_at_capacity() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let mut rng = Rng::new(42);

        // One house with capacity 6.
        registry.register(make_residential(1, 40));
        make_active(&mut entities, 1, 0);

        // Population of 100 exceeds capacity of 6.
        let stats = tick_population(&entities, &registry, &mut events, 0, &mut rng, 100);

        assert_eq!(stats.total_housing_capacity, 6);
        assert_eq!(stats.occupied_housing, 6);
        assert!(stats.total_population > 6);
    }
}
