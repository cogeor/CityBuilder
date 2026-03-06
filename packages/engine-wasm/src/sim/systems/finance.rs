//! Finance and budget system: income, expenses, treasury management.
//!
//! Each tick, computes tax income from population and active
//! Commercial/Industrial buildings, sums upkeep expenses from all active
//! entities, updates the city treasury, and emits budget-related events.

use crate::core::archetypes::{ArchetypeRegistry, ArchetypeTag};
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent};
use crate::core::world::CityPolicies;
use crate::core_types::*;

/// Aggregated budget statistics for a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetStats {
    /// Total income per tick in cents.
    pub income_per_tick: MoneyCents,
    /// Total expenses per tick in cents.
    pub expenses_per_tick: MoneyCents,
    /// Net income per tick (income - expenses).
    pub net_per_tick: MoneyCents,
    /// Current treasury balance after this tick.
    pub treasury: MoneyCents,
}

/// Compute total tax income per tick from population and active buildings.
///
/// Residential tax: each resident generates 1 cent/tick base, scaled by
/// residential_tax_pct / 9 (9 is the default rate).
///
/// Commercial tax: for each active commercial entity,
/// job_capacity * commercial_tax_pct / 100 cents/tick.
///
/// Industrial tax: for each active industrial entity,
/// job_capacity * industrial_tax_pct / 100 cents/tick.
pub fn compute_tax_income(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    policies: &CityPolicies,
    population: u32,
) -> MoneyCents {
    // Residential tax: population * residential_tax_pct / 9 (integer division).
    let residential_income =
        population as i64 * policies.residential_tax_pct as i64 / 9;

    // Commercial and industrial tax from active buildings.
    let mut commercial_income: MoneyCents = 0;
    let mut industrial_income: MoneyCents = 0;

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

        if def.has_tag(ArchetypeTag::Commercial) {
            commercial_income +=
                def.job_capacity() as i64 * policies.commercial_tax_pct as i64 / 100;
        }

        if def.has_tag(ArchetypeTag::Industrial) {
            industrial_income +=
                def.job_capacity() as i64 * policies.industrial_tax_pct as i64 / 100;
        }
    }

    residential_income + commercial_income + industrial_income
}

/// Compute total upkeep expenses per tick from all active entities.
///
/// Sums `upkeep_at_level()` for every alive entity that is not under
/// construction.
pub fn compute_expenses(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
) -> MoneyCents {
    let mut total: MoneyCents = 0;

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

        let level = entities.get_level(handle).unwrap_or(1);
        total += def.upkeep_at_level(level);
    }

    total
}

/// Run one tick of the finance system.
///
/// Computes income and expenses, updates the treasury, and emits
/// budget-related events: BudgetDeficit if net < 0, BudgetSurplus
/// once per game day if net > 0, and DebtWarning if treasury < 0.
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

    // Emit BudgetDeficit if running at a loss.
    if net < 0 {
        events.publish(
            tick,
            SimEvent::BudgetDeficit {
                amount_cents: -net,
            },
        );
    }

    // Emit BudgetSurplus once per game day if running at a surplus.
    if net > 0 && tick % TICKS_PER_GAME_DAY == 0 {
        events.publish(
            tick,
            SimEvent::BudgetSurplus {
                amount_cents: net,
            },
        );
    }

    // Emit DebtWarning if treasury is negative.
    if *treasury < 0 {
        events.publish(
            tick,
            SimEvent::DebtWarning {
                treasury_cents: *treasury,
            },
        );
    }

    BudgetStats {
        income_per_tick: income,
        expenses_per_tick: expenses,
        net_per_tick: net,
        treasury: *treasury,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::ArchetypeDefinition;

    /// Helper: create a residential archetype.
    fn make_residential(id: ArchetypeId) -> ArchetypeDefinition {
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
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2,
            desirability_magnitude: 5,
            pollution: 0,
            noise: 1,
            build_time_ticks: 500,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
            effects: vec![],
        }
    }

    /// Helper: create a commercial archetype.
    fn make_commercial(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Shop {}", id),
            tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 80_000,
            base_upkeep_cents_per_tick: 15,
            power_demand_kw: 10,
            power_supply_kw: 0,
            water_demand: 3,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
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
            effects: vec![],
        }
    }

    /// Helper: create an industrial archetype.
    fn make_industrial(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Factory {}", id),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 120_000,
            base_upkeep_cents_per_tick: 20,
            power_demand_kw: 15,
            power_supply_kw: 0,
            water_demand: 5,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 4,
            desirability_magnitude: -10,
            pollution: 5,
            noise: 4,
            build_time_ticks: 600,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 50,
            living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    /// Helper: allocate an entity and clear UNDER_CONSTRUCTION to make it active.
    fn make_active(entities: &mut EntityStore, arch_id: ArchetypeId, x: i16) -> EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    // ─── Test 1: Empty world zero income zero expenses ──────────────────────

    #[test]
    fn empty_world_zero_income_zero_expenses() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let policies = CityPolicies::default();

        let income = compute_tax_income(&entities, &registry, &policies, 0);
        let expenses = compute_expenses(&entities, &registry);

        assert_eq!(income, 0);
        assert_eq!(expenses, 0);
    }

    // ─── Test 2: Tax income scales with population ──────────────────────────

    #[test]
    fn tax_income_scales_with_population() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let policies = CityPolicies::default(); // residential_tax_pct = 9

        // With default tax rate 9, each resident generates 9/9 = 1 cent/tick.
        let income_100 = compute_tax_income(&entities, &registry, &policies, 100);
        let income_200 = compute_tax_income(&entities, &registry, &policies, 200);

        // income should scale linearly with population.
        assert_eq!(income_100, 100);
        assert_eq!(income_200, 200);
        assert_eq!(income_200, income_100 * 2);
    }

    // ─── Test 3: Expenses computed from active entities ─────────────────────

    #[test]
    fn expenses_computed_from_active_entities() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        // Residential upkeep = 10 cents/tick at level 1
        registry.register(make_residential(1));
        // Commercial upkeep = 15 cents/tick at level 1
        registry.register(make_commercial(2));

        make_active(&mut entities, 1, 0); // 10 upkeep
        make_active(&mut entities, 2, 1); // 15 upkeep

        let expenses = compute_expenses(&entities, &registry);
        assert_eq!(expenses, 25);
    }

    // ─── Test 4: Treasury increases with surplus ────────────────────────────

    #[test]
    fn treasury_increases_with_surplus() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let policies = CityPolicies::default();
        let mut treasury: MoneyCents = 1000;

        // 100 population, no buildings = 100 income, 0 expenses.
        let stats = tick_finance(
            &entities, &registry, &mut events, 0,
            &policies, &mut treasury, 100,
        );

        assert_eq!(stats.income_per_tick, 100);
        assert_eq!(stats.expenses_per_tick, 0);
        assert_eq!(stats.net_per_tick, 100);
        assert_eq!(treasury, 1100);
        assert_eq!(stats.treasury, 1100);
    }

    // ─── Test 5: Treasury decreases with deficit ────────────────────────────

    #[test]
    fn treasury_decreases_with_deficit() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let policies = CityPolicies::default();
        let mut treasury: MoneyCents = 1000;

        // Expensive building with no population = 0 income, 20 expenses.
        registry.register(make_industrial(1));
        make_active(&mut entities, 1, 0);

        let stats = tick_finance(
            &entities, &registry, &mut events, 1,
            &policies, &mut treasury, 0,
        );

        assert_eq!(stats.income_per_tick, 0);
        assert_eq!(stats.expenses_per_tick, 20);
        assert_eq!(stats.net_per_tick, -20);
        assert_eq!(treasury, 980);
    }

    // ─── Test 6: Under-construction excluded from expenses ──────────────────

    #[test]
    fn under_construction_excluded_from_expenses() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        registry.register(make_residential(1));

        // One active building (upkeep = 10).
        make_active(&mut entities, 1, 0);
        // One under construction (should NOT count).
        let _h_uc = entities.alloc(1, 1, 0, 0).unwrap();

        let expenses = compute_expenses(&entities, &registry);
        assert_eq!(expenses, 10);
    }

    // ─── Test 7: Deficit event emitted ──────────────────────────────────────

    #[test]
    fn deficit_event_emitted() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let policies = CityPolicies::default();
        let mut treasury: MoneyCents = 1000;

        // Building with upkeep but no population income.
        registry.register(make_industrial(1));
        make_active(&mut entities, 1, 0);

        tick_finance(
            &entities, &registry, &mut events, 1,
            &policies, &mut treasury, 0,
        );

        let drained = events.drain();
        let deficit_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::BudgetDeficit { .. }))
            .collect();
        assert_eq!(deficit_events.len(), 1);

        if let SimEvent::BudgetDeficit { amount_cents } = &deficit_events[0].event {
            assert_eq!(*amount_cents, 20);
        } else {
            panic!("Expected BudgetDeficit event");
        }
    }

    // ─── Test 8: Debt warning emitted when treasury negative ────────────────

    #[test]
    fn debt_warning_emitted_when_treasury_negative() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let policies = CityPolicies::default();
        let mut treasury: MoneyCents = 10; // Small treasury

        // Building with 20 upkeep, no income -> treasury goes to -10.
        registry.register(make_industrial(1));
        make_active(&mut entities, 1, 0);

        tick_finance(
            &entities, &registry, &mut events, 1,
            &policies, &mut treasury, 0,
        );

        assert_eq!(treasury, -10);

        let drained = events.drain();
        let debt_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::DebtWarning { .. }))
            .collect();
        assert_eq!(debt_events.len(), 1);

        if let SimEvent::DebtWarning { treasury_cents } = &debt_events[0].event {
            assert_eq!(*treasury_cents, -10);
        } else {
            panic!("Expected DebtWarning event");
        }
    }

    // ─── Test 9: Commercial tax income computed correctly ───────────────────

    #[test]
    fn commercial_tax_income_computed_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        // Commercial: 1x1, 50% coverage, 2 floors = 256 m2 / 25 = 10 jobs.
        registry.register(make_commercial(1));
        make_active(&mut entities, 1, 0);

        let policies = CityPolicies::default(); // commercial_tax_pct = 9

        // Commercial income = 10 jobs * 9 / 100 = 0 (integer division)
        // With 0 population, residential income = 0.
        let income = compute_tax_income(&entities, &registry, &policies, 0);
        assert_eq!(income, 0); // 10 * 9 / 100 = 0 (integer floor)

        // With higher tax rate.
        let mut policies_high = CityPolicies::default();
        policies_high.commercial_tax_pct = 50;

        // 10 * 50 / 100 = 5
        let income_high = compute_tax_income(&entities, &registry, &policies_high, 0);
        assert_eq!(income_high, 5);
    }

    // ─── Test 10: Industrial tax income computed correctly ──────────────────

    #[test]
    fn industrial_tax_income_computed_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        // Industrial: 1x1, 50% coverage, 2 floors = 256 m2 / 50 = 5 jobs.
        registry.register(make_industrial(1));
        make_active(&mut entities, 1, 0);

        let mut policies = CityPolicies::default();
        policies.industrial_tax_pct = 100;

        // 5 jobs * 100 / 100 = 5
        let income = compute_tax_income(&entities, &registry, &policies, 0);
        assert_eq!(income, 5);
    }

    // ─── Test 11: Surplus event emitted only on game day boundary ───────────

    #[test]
    fn surplus_event_emitted_on_game_day_boundary() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let policies = CityPolicies::default();

        // Tick NOT on day boundary: no surplus event.
        let mut events = EventBus::new();
        let mut treasury: MoneyCents = 1000;
        tick_finance(
            &entities, &registry, &mut events, 1,
            &policies, &mut treasury, 100,
        );
        let drained = events.drain();
        let surplus_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::BudgetSurplus { .. }))
            .collect();
        assert_eq!(surplus_events.len(), 0);

        // Tick ON day boundary: surplus event emitted.
        let mut events = EventBus::new();
        let mut treasury2: MoneyCents = 1000;
        tick_finance(
            &entities, &registry, &mut events, TICKS_PER_GAME_DAY,
            &policies, &mut treasury2, 100,
        );
        let drained = events.drain();
        let surplus_events: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::BudgetSurplus { .. }))
            .collect();
        assert_eq!(surplus_events.len(), 1);
    }

    // ─── Test 12: Expenses scale with entity level ──────────────────────────

    #[test]
    fn expenses_scale_with_entity_level() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();

        // base_upkeep = 10 cents/tick
        registry.register(make_residential(1));

        let h = make_active(&mut entities, 1, 0);
        // Level 1: upkeep = 10 * 100 / 100 = 10
        assert_eq!(compute_expenses(&entities, &registry), 10);

        // Level 2: upkeep = 10 * 130 / 100 = 13
        entities.set_level(h, 2);
        assert_eq!(compute_expenses(&entities, &registry), 13);

        // Level 3: upkeep = 10 * 160 / 100 = 16
        entities.set_level(h, 3);
        assert_eq!(compute_expenses(&entities, &registry), 16);
    }

    // ─── Test 13: Tax rate affects residential income ────────────────────────

    #[test]
    fn tax_rate_affects_residential_income() {
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();

        // Default rate (9): 100 * 9 / 9 = 100
        let default_policies = CityPolicies::default();
        let income_default = compute_tax_income(&entities, &registry, &default_policies, 100);
        assert_eq!(income_default, 100);

        // Higher rate (18): 100 * 18 / 9 = 200
        let mut high_policies = CityPolicies::default();
        high_policies.residential_tax_pct = 18;
        let income_high = compute_tax_income(&entities, &registry, &high_policies, 100);
        assert_eq!(income_high, 200);

        // Lower rate (0): 100 * 0 / 9 = 0
        let mut zero_policies = CityPolicies::default();
        zero_policies.residential_tax_pct = 0;
        let income_zero = compute_tax_income(&entities, &registry, &zero_policies, 100);
        assert_eq!(income_zero, 0);
    }
}
