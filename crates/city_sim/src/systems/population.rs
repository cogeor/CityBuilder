//! Population and housing system.

use city_core::{StatusFlags, Tick};
use city_engine::archetype::{ArchetypeRegistry, ArchetypeTag};
use city_engine::entity::EntityStore;

use crate::events::{EventBus, SimEvent};
use crate::math::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationStats {
    pub total_population: u32,
    pub total_housing_capacity: u32,
    pub occupied_housing: u32,
}

const MAX_INCOMING_PER_TICK: i32 = 5;
const MAX_OUTGOING_PER_TICK: i32 = 3;

/// Compute total housing capacity from active residential entities.
pub fn compute_housing_capacity(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
) -> u32 {
    let mut total: u32 = 0;
    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) { Some(f) => f, None => continue };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
        let arch_id = match entities.get_archetype(handle) { Some(id) => id, None => continue };
        let def = match registry.get(arch_id) { Some(d) => d, None => continue };
        if def.has_tag(ArchetypeTag::Residential) {
            total += def.resident_capacity();
        }
    }
    total
}

/// Compute a simple desirability score for a single entity.
pub fn compute_desirability_score(entities: &EntityStore, handle: city_core::EntityHandle) -> u8 {
    let flags = match entities.get_flags(handle) { Some(f) => f, None => return 0 };
    let mut score: u8 = 50;
    if flags.contains(StatusFlags::POWERED) { score += 10; }
    if flags.contains(StatusFlags::WATER_CONNECTED) { score += 10; }
    score
}

/// Run one tick of the population system.
pub fn tick_population(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    rng: &mut Rng,
    current_population: u32,
) -> PopulationStats {
    let capacity = compute_housing_capacity(entities, registry);

    let base_migration: i32 = if capacity > current_population {
        let surplus = (capacity - current_population) as i32;
        (surplus / 10).min(MAX_INCOMING_PER_TICK)
    } else if current_population > capacity {
        let deficit = (current_population - capacity) as i32;
        -(deficit / 10).min(MAX_OUTGOING_PER_TICK)
    } else {
        0
    };

    let jitter = rng.range_inclusive(-1, 1);
    let net_migration = base_migration + jitter;

    let new_population = if net_migration < 0 {
        current_population.saturating_sub(net_migration.unsigned_abs())
    } else {
        current_population + net_migration as u32
    };

    if new_population != current_population {
        events.publish(tick, SimEvent::PopulationChanged { old: current_population, new: new_population });
    }
    if new_population > capacity {
        events.publish(tick, SimEvent::HousingShortage { deficit: new_population - capacity });
    }
    if net_migration.abs() > 2 {
        events.publish(tick, SimEvent::MigrationWave { incoming: net_migration });
    }

    let occupied = new_population.min(capacity);
    PopulationStats { total_population: new_population, total_housing_capacity: capacity, occupied_housing: occupied }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_engine::archetype::ArchetypeDefinition;

    fn make_residential(id: u16, living_space_m2: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("House {}", id),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
            base_cost_cents: 100_000, base_upkeep_cents_per_tick: 10,
            power_demand_kw: 5, power_supply_kw: 0,
            water_demand: 2, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2, desirability_magnitude: 5,
            pollution: 0, noise: 1,
            build_time_ticks: 500, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: living_space_m2,
            effects: vec![],
        }
    }

    fn make_active(entities: &mut EntityStore, arch_id: u16, x: i16) -> city_core::EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    #[test]
    fn empty_world_returns_zero_stats() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let mut rng = Rng::new(42);
        let stats = tick_population(&entities, &registry, &mut events, 0, &mut rng, 0);
        assert_eq!(stats.total_population, 0);
        assert_eq!(stats.total_housing_capacity, 0);
    }

    #[test]
    fn housing_capacity_computed_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_residential(1, 40));
        make_active(&mut entities, 1, 0);
        make_active(&mut entities, 1, 1);
        let capacity = compute_housing_capacity(&entities, &registry);
        assert_eq!(capacity, 12);
    }

    #[test]
    fn population_grows_with_surplus_housing() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        registry.register(make_residential(1, 40));
        for x in 0..10 { make_active(&mut entities, 1, x); }
        let mut pop = 0u32;
        for tick in 0..20 {
            let mut rng = Rng::new(100 + tick);
            let stats = tick_population(&entities, &registry, &mut events, tick, &mut rng, pop);
            pop = stats.total_population;
        }
        assert!(pop > 0);
    }

    #[test]
    fn population_cannot_go_below_zero() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut pop = 1u32;
        for tick in 0..50 {
            let mut events = EventBus::new();
            let mut rng = Rng::new(300 + tick);
            let stats = tick_population(&entities, &registry, &mut events, tick, &mut rng, pop);
            pop = stats.total_population;
        }
        assert!(pop <= 1);
    }

    #[test]
    fn desirability_score_base() {
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 0, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        assert_eq!(compute_desirability_score(&entities, h), 50);
    }

    #[test]
    fn desirability_score_with_power_and_water() {
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 0, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::POWERED | StatusFlags::WATER_CONNECTED);
        assert_eq!(compute_desirability_score(&entities, h), 70);
    }
}
