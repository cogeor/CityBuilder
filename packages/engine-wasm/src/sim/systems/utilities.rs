//! Utility distribution system: power and water networks.
//!
//! Each tick, computes total supply vs. demand for power and water.
//! When supply >= demand, all buildings are satisfied.
//! When supply < demand, allocates by priority (civic > residential > commercial > industrial).

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent, UtilityType};
use crate::core_types::*;

/// Result of utility distribution for one utility type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtilityBalance {
    pub supply: u32,
    pub demand: u32,
    pub satisfied: u32,
    pub unsatisfied: u32,
}

impl UtilityBalance {
    pub fn has_shortage(&self) -> bool {
        self.demand > self.supply
    }

    pub fn deficit(&self) -> u32 {
        self.demand.saturating_sub(self.supply)
    }
}

/// Priority tier for utility allocation (lower = higher priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum AllocPriority {
    Civic = 0,
    Residential = 1,
    Commercial = 2,
    Industrial = 3,
    Other = 4,
}

/// Determine allocation priority from archetype tags.
fn entity_priority(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    handle: EntityHandle,
) -> AllocPriority {
    let arch_id = match entities.get_archetype(handle) {
        Some(id) => id,
        None => return AllocPriority::Other,
    };
    let def = match registry.get(arch_id) {
        Some(d) => d,
        None => return AllocPriority::Other,
    };

    use crate::core::archetypes::ArchetypeTag;
    if def.has_tag(ArchetypeTag::Civic) || def.has_tag(ArchetypeTag::Service) {
        AllocPriority::Civic
    } else if def.has_tag(ArchetypeTag::Residential) {
        AllocPriority::Residential
    } else if def.has_tag(ArchetypeTag::Commercial) {
        AllocPriority::Commercial
    } else if def.has_tag(ArchetypeTag::Industrial) {
        AllocPriority::Industrial
    } else {
        AllocPriority::Other
    }
}

/// Distribute power to all active (non-under-construction) entities.
/// Updates POWERED status flag on each entity.
pub fn tick_power(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    prev_had_shortage: bool,
) -> UtilityBalance {
    distribute_utility(
        entities,
        registry,
        events,
        tick,
        prev_had_shortage,
        UtilityType::Power,
        |def, level| def.power_supply_kw * level_multiplier(level) / 100,
        |def, level| def.power_demand_at_level(level),
        StatusFlags::POWERED,
    )
}

/// Distribute water to all active entities.
/// Updates HAS_WATER status flag on each entity.
pub fn tick_water(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    prev_had_shortage: bool,
) -> UtilityBalance {
    distribute_utility(
        entities,
        registry,
        events,
        tick,
        prev_had_shortage,
        UtilityType::Water,
        |def, level| def.water_supply * level_multiplier(level) / 100,
        |def, _level| def.water_demand,
        StatusFlags::HAS_WATER,
    )
}

/// Level multiplier: +20% per level above 1.
fn level_multiplier(level: u8) -> u32 {
    100 + (level.saturating_sub(1) as u32) * 20
}

/// Generic utility distribution.
fn distribute_utility<S, D>(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    prev_had_shortage: bool,
    utility_type: UtilityType,
    supply_fn: S,
    demand_fn: D,
    satisfied_flag: StatusFlags,
) -> UtilityBalance
where
    S: Fn(&crate::core::archetypes::ArchetypeDefinition, u8) -> u32,
    D: Fn(&crate::core::archetypes::ArchetypeDefinition, u8) -> u32,
{
    // Collect all active entities (not under construction).
    let active: Vec<EntityHandle> = entities
        .iter_alive()
        .filter(|h| {
            let flags = entities.get_flags(*h).unwrap_or(StatusFlags::NONE);
            !flags.contains(StatusFlags::UNDER_CONSTRUCTION)
        })
        .collect();

    // Calculate total supply and demand.
    let mut total_supply: u32 = 0;
    let mut total_demand: u32 = 0;

    // Entities requesting utility, sorted by priority.
    let mut consumers: Vec<(EntityHandle, u32, AllocPriority)> = Vec::new();

    for &handle in &active {
        let arch_id = match entities.get_archetype(handle) {
            Some(id) => id,
            None => continue,
        };
        let def = match registry.get(arch_id) {
            Some(d) => d,
            None => continue,
        };
        let level = entities.get_level(handle).unwrap_or(1);
        let enabled = entities.get_enabled(handle).unwrap_or(true);

        let supply = if enabled { supply_fn(def, level) } else { 0 };
        let demand = if enabled { demand_fn(def, level) } else { 0 };

        total_supply += supply;

        if demand > 0 {
            let priority = entity_priority(entities, registry, handle);
            consumers.push((handle, demand, priority));
            total_demand += demand;
        }
    }

    // Sort consumers by priority (lower = higher priority).
    consumers.sort_by_key(|&(_, _, p)| p);

    // Allocate supply to consumers.
    let mut remaining_supply = total_supply;
    let mut satisfied_count: u32 = 0;
    let mut unsatisfied_count: u32 = 0;
    let mut satisfied_handles: Vec<EntityHandle> = Vec::new();
    let mut unsatisfied_handles: Vec<EntityHandle> = Vec::new();

    for (handle, demand, _) in &consumers {
        if remaining_supply >= *demand {
            remaining_supply -= *demand;
            satisfied_handles.push(*handle);
            satisfied_count += 1;
        } else {
            unsatisfied_handles.push(*handle);
            unsatisfied_count += 1;
        }
    }

    // Update flags.
    for handle in &satisfied_handles {
        if let Some(flags) = entities.get_flags(*handle) {
            entities.set_flags(*handle, flags.insert(satisfied_flag));
        }
    }
    for handle in &unsatisfied_handles {
        if let Some(flags) = entities.get_flags(*handle) {
            entities.set_flags(*handle, flags.remove(satisfied_flag));
        }
    }

    // Emit events.
    let has_shortage = total_demand > total_supply;
    if has_shortage {
        let deficit = total_demand - total_supply;
        match utility_type {
            UtilityType::Power => {
                events.publish(tick, SimEvent::PowerShortage { deficit_kw: deficit });
            }
            UtilityType::Water => {
                events.publish(tick, SimEvent::WaterShortage { deficit });
            }
        }
    } else if prev_had_shortage {
        events.publish(tick, SimEvent::UtilityRestored { utility_type });
    }

    UtilityBalance {
        supply: total_supply,
        demand: total_demand,
        satisfied: satisfied_count,
        unsatisfied: unsatisfied_count,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeTag};

    fn make_archetype(
        id: ArchetypeId,
        tags: Vec<ArchetypeTag>,
        power_supply: u32,
        power_demand: u32,
        water_supply: u32,
        water_demand: u32,
    ) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Archetype {}", id),
            tags,
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 1,
            usable_ratio_pct: 80,
            base_cost_cents: 10_000,
            base_upkeep_cents_per_tick: 1,
            power_demand_kw: power_demand,
            power_supply_kw: power_supply,
            water_demand,
            water_supply,
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

    fn make_active_entity(
        entities: &mut EntityStore,
        arch_id: ArchetypeId,
        x: i16,
    ) -> EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        // Clear UNDER_CONSTRUCTION to make it active.
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    #[test]
    fn power_surplus_all_satisfied() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // Power plant: supplies 1000 kW
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Utility],
            1000, 0, 0, 0,
        ));
        // House: demands 5 kW
        registry.register(make_archetype(
            2,
            vec![ArchetypeTag::Residential],
            0, 5, 0, 0,
        ));

        make_active_entity(&mut entities, 1, 0); // power plant
        let h1 = make_active_entity(&mut entities, 2, 1); // house 1
        let h2 = make_active_entity(&mut entities, 2, 2); // house 2

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false);

        assert_eq!(balance.supply, 1000);
        assert_eq!(balance.demand, 10);
        assert_eq!(balance.satisfied, 2);
        assert_eq!(balance.unsatisfied, 0);
        assert!(!balance.has_shortage());

        // Both houses should have POWERED flag.
        assert!(entities.get_flags(h1).unwrap().contains(StatusFlags::POWERED));
        assert!(entities.get_flags(h2).unwrap().contains(StatusFlags::POWERED));

        // No shortage events.
        assert!(events.is_empty());
    }

    #[test]
    fn power_shortage_priority_allocation() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // Small power plant: 10 kW
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Utility],
            10, 0, 0, 0,
        ));
        // Civic: 5 kW demand (high priority)
        registry.register(make_archetype(
            2,
            vec![ArchetypeTag::Civic],
            0, 5, 0, 0,
        ));
        // Industrial: 10 kW demand (low priority)
        registry.register(make_archetype(
            3,
            vec![ArchetypeTag::Industrial],
            0, 10, 0, 0,
        ));

        make_active_entity(&mut entities, 1, 0);
        let civic = make_active_entity(&mut entities, 2, 1);
        let industrial = make_active_entity(&mut entities, 3, 2);

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false);

        assert_eq!(balance.supply, 10);
        assert_eq!(balance.demand, 15);
        assert!(balance.has_shortage());
        assert_eq!(balance.deficit(), 5);

        // Civic should get power (higher priority).
        assert!(entities.get_flags(civic).unwrap().contains(StatusFlags::POWERED));
        // Industrial should NOT get power.
        assert!(!entities.get_flags(industrial).unwrap().contains(StatusFlags::POWERED));

        // Shortage event emitted.
        let drained = events.drain();
        assert_eq!(drained.len(), 1);
        assert!(matches!(
            drained[0].event,
            SimEvent::PowerShortage { deficit_kw: 5 }
        ));
    }

    #[test]
    fn water_distribution() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        // Water facility: supplies 100 units
        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Utility],
            0, 0, 100, 0,
        ));
        // House: demands 2 units
        registry.register(make_archetype(
            2,
            vec![ArchetypeTag::Residential],
            0, 0, 0, 2,
        ));

        make_active_entity(&mut entities, 1, 0);
        let h = make_active_entity(&mut entities, 2, 1);

        let balance = tick_water(&mut entities, &registry, &mut events, 0, false);

        assert_eq!(balance.supply, 100);
        assert_eq!(balance.demand, 2);
        assert!(!balance.has_shortage());
        assert!(entities.get_flags(h).unwrap().contains(StatusFlags::HAS_WATER));
    }

    #[test]
    fn under_construction_excluded() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Utility],
            1000, 0, 0, 0,
        ));
        registry.register(make_archetype(
            2,
            vec![ArchetypeTag::Residential],
            0, 5, 0, 0,
        ));

        make_active_entity(&mut entities, 1, 0);
        // This entity is still under construction (default).
        let h = entities.alloc(2, 1, 0, 0).unwrap();

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false);

        // Under-construction entity shouldn't count as demand.
        assert_eq!(balance.demand, 0);
        // And shouldn't get POWERED flag.
        assert!(!entities.get_flags(h).unwrap().contains(StatusFlags::POWERED));
    }

    #[test]
    fn disabled_entity_no_supply_no_demand() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Utility],
            1000, 0, 0, 0,
        ));

        let h = make_active_entity(&mut entities, 1, 0);
        entities.set_enabled(h, false);

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false);

        // Disabled power plant provides no supply.
        assert_eq!(balance.supply, 0);
    }

    #[test]
    fn utility_restored_event() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Utility],
            100, 0, 0, 0,
        ));

        make_active_entity(&mut entities, 1, 0);

        // No consumers, supply > 0, prev had shortage -> emit restored.
        let balance = tick_power(&mut entities, &registry, &mut events, 0, true);

        assert!(!balance.has_shortage());
        let drained = events.drain();
        assert_eq!(drained.len(), 1);
        assert!(matches!(
            drained[0].event,
            SimEvent::UtilityRestored {
                utility_type: UtilityType::Power
            }
        ));
    }

    #[test]
    fn no_events_when_no_change() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Utility],
            100, 0, 0, 0,
        ));

        make_active_entity(&mut entities, 1, 0);

        // No shortage now, no shortage before -> no events.
        tick_power(&mut entities, &registry, &mut events, 0, false);
        assert!(events.is_empty());
    }

    #[test]
    fn balance_with_zero_supply() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(
            1,
            vec![ArchetypeTag::Residential],
            0, 5, 0, 0,
        ));

        make_active_entity(&mut entities, 1, 0);

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false);

        assert_eq!(balance.supply, 0);
        assert_eq!(balance.demand, 5);
        assert!(balance.has_shortage());
        assert_eq!(balance.unsatisfied, 1);
    }

    #[test]
    fn empty_world_no_panic() {
        let mut entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false);

        assert_eq!(balance.supply, 0);
        assert_eq!(balance.demand, 0);
        assert_eq!(balance.satisfied, 0);
        assert_eq!(balance.unsatisfied, 0);
    }
}
