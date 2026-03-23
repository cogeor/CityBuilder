//! Finance and budget system: income, expenses, treasury management.

use city_core::{StatusFlags, Tick};
use crate::types::MoneyCents;
use crate::archetype::{ArchetypeRegistry, ArchetypeTag};
use city_engine::entity::EntityStore;

use crate::events::{EventBus, SimEvent};
use crate::types::TICKS_PER_GAME_DAY;
use crate::world::CityPolicies;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetStats {
    pub income_per_tick: MoneyCents,
    pub expenses_per_tick: MoneyCents,
    pub net_per_tick: MoneyCents,
    pub treasury: MoneyCents,
}

/// Compute total tax income per tick.
pub fn compute_tax_income(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    policies: &CityPolicies,
    population: u32,
) -> MoneyCents {
    let residential_income = population as i64 * policies.residential_tax_pct as i64 / 9;

    let mut commercial_income: MoneyCents = 0;
    let mut industrial_income: MoneyCents = 0;

    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) { Some(f) => f, None => continue };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
        let arch_id = match entities.get_archetype(handle) { Some(id) => id, None => continue };
        let def = match registry.get(arch_id) { Some(d) => d, None => continue };

        if def.has_tag(ArchetypeTag::Commercial) {
            commercial_income += def.job_capacity() as i64 * policies.commercial_tax_pct as i64 / 100;
        }
        if def.has_tag(ArchetypeTag::Industrial) {
            industrial_income += def.job_capacity() as i64 * policies.industrial_tax_pct as i64 / 100;
        }
    }

    residential_income + commercial_income + industrial_income
}

/// Compute total upkeep expenses per tick.
pub fn compute_expenses(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
) -> MoneyCents {
    let mut total: MoneyCents = 0;
    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) { Some(f) => f, None => continue };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
        let arch_id = match entities.get_archetype(handle) { Some(id) => id, None => continue };
        let def = match registry.get(arch_id) { Some(d) => d, None => continue };
        let level = entities.get_level(handle).unwrap_or(1);
        total += def.upkeep_at_level(level);
    }
    total
}

/// Run one tick of the finance system.
pub fn tick_finance(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    policies: &CityPolicies,
    treasury: &mut MoneyCents,
    population: u32,
) -> BudgetStats {
    let income = compute_tax_income(entities, registry, policies, population);
    let expenses = compute_expenses(entities, registry);
    let net = income - expenses;

    *treasury += net;

    if net < 0 {
        events.publish(tick, SimEvent::BudgetDeficit { amount_cents: -net });
    }
    if net > 0 && tick % TICKS_PER_GAME_DAY == 0 {
        events.publish(tick, SimEvent::BudgetSurplus { amount_cents: net });
    }
    if *treasury < 0 {
        events.publish(tick, SimEvent::DebtWarning { treasury_cents: *treasury });
    }

    BudgetStats { income_per_tick: income, expenses_per_tick: expenses, net_per_tick: net, treasury: *treasury }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::ArchetypeDefinition;

    fn make_residential(id: u16) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("House {}", id),
            tags: vec![ArchetypeTag::Residential],
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
            workspace_per_job_m2: 0, living_space_per_person_m2: 40,
            effects: vec![],
        }
    }

    fn make_industrial(id: u16) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("Factory {}", id),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
            base_cost_cents: 120_000, base_upkeep_cents_per_tick: 20,
            power_demand_kw: 15, power_supply_kw: 0,
            water_demand: 5, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 4, desirability_magnitude: -10,
            pollution: 5, noise: 4,
            build_time_ticks: 600, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 50, living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    fn make_active(entities: &mut EntityStore, arch_id: u16, x: i16) -> city_core::EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    #[test]
    fn empty_world_zero_income_zero_expenses() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let policies = CityPolicies::default();
        assert_eq!(compute_tax_income(&entities, &registry, &policies, 0), 0);
        assert_eq!(compute_expenses(&entities, &registry), 0);
    }

    #[test]
    fn tax_income_scales_with_population() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let policies = CityPolicies::default();
        assert_eq!(compute_tax_income(&entities, &registry, &policies, 100), 100);
        assert_eq!(compute_tax_income(&entities, &registry, &policies, 200), 200);
    }

    #[test]
    fn treasury_increases_with_surplus() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let policies = CityPolicies::default();
        let mut treasury: MoneyCents = 1000;
        let stats = tick_finance(&entities, &registry, &mut events, 0, &policies, &mut treasury, 100);
        assert_eq!(stats.net_per_tick, 100);
        assert_eq!(treasury, 1100);
    }

    #[test]
    fn treasury_decreases_with_deficit() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let policies = CityPolicies::default();
        let mut treasury: MoneyCents = 1000;
        registry.register(make_industrial(1));
        make_active(&mut entities, 1, 0);
        let stats = tick_finance(&entities, &registry, &mut events, 1, &policies, &mut treasury, 0);
        assert_eq!(stats.expenses_per_tick, 20);
        assert_eq!(treasury, 980);
    }

    #[test]
    fn expenses_scale_with_entity_level() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_residential(1));
        let h = make_active(&mut entities, 1, 0);
        assert_eq!(compute_expenses(&entities, &registry), 10);
        entities.set_level(h, 2);
        assert_eq!(compute_expenses(&entities, &registry), 13);
    }

    #[test]
    fn debt_warning_emitted() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let policies = CityPolicies::default();
        let mut treasury: MoneyCents = 10;
        registry.register(make_industrial(1));
        make_active(&mut entities, 1, 0);
        tick_finance(&entities, &registry, &mut events, 1, &policies, &mut treasury, 0);
        assert_eq!(treasury, -10);
        let drained = events.drain();
        assert!(drained.iter().any(|e| matches!(e.event, SimEvent::DebtWarning { .. })));
    }
}
