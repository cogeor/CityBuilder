//! Jobs and industry system: employment matching and labor market.

use city_core::{EntityHandle, StatusFlags, Tick};
use crate::archetype::{ArchetypeRegistry, ArchetypeTag};
use city_engine::entity::EntityStore;

use crate::events::{EventBus, SimEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobStats {
    pub total_jobs: u32,
    pub employed: u32,
    pub unemployed: u32,
    pub labor_demand: u32,
}

/// Compute total job capacity from active Commercial/Industrial entities.
pub fn compute_job_capacity(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
) -> u32 {
    let mut total: u32 = 0;
    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) { Some(f) => f, None => continue };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
        let arch_id = match entities.get_archetype(handle) { Some(id) => id, None => continue };
        let def = match registry.get(arch_id) { Some(d) => d, None => continue };
        if def.has_tag(ArchetypeTag::Commercial) || def.has_tag(ArchetypeTag::Industrial) {
            total += def.job_capacity();
        }
    }
    total
}

#[inline]
pub fn unemployment_rate(unemployed: u32, population: u32) -> u8 {
    if population == 0 { return 0; }
    ((unemployed as u64 * 100) / population as u64) as u8
}

/// Run the jobs system for one tick.
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

    let active_job_entities: Vec<(EntityHandle, u32)> = entities
        .iter_alive()
        .filter_map(|handle| {
            let flags = entities.get_flags(handle)?;
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { return None; }
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
        if let Some(mut flags) = entities.get_flags(*handle) {
            if remaining_workers >= *capacity && *capacity > 0 {
                remaining_workers -= *capacity;
                flags.insert(StatusFlags::STAFFED);
            } else {
                flags.remove(StatusFlags::STAFFED);
            }
            entities.set_flags(*handle, flags);
        }
    }

    let rate = unemployment_rate(unemployed, population);
    if rate > 10 {
        events.publish(tick, SimEvent::UnemploymentHigh { rate_pct: rate });
    }
    if labor_demand > 0 {
        events.publish(tick, SimEvent::LaborShortage { deficit: labor_demand });
    }

    JobStats { total_jobs, employed, unemployed, labor_demand }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::ArchetypeDefinition;

    fn make_arch(id: u16, tags: Vec<ArchetypeTag>, workspace: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("Arch {}", id), tags,
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
            base_cost_cents: 10_000, base_upkeep_cents_per_tick: 1,
            power_demand_kw: 5, power_supply_kw: 0,
            water_demand: 1, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 0, noise: 0,
            build_time_ticks: 100, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: workspace, living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    fn make_active(entities: &mut EntityStore, arch_id: u16, x: i16) -> EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    #[test]
    fn empty_world_returns_zero_stats() {
        let mut entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        let stats = tick_jobs(&mut entities, &registry, &mut events, 0, 0);
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.employed, 0);
    }

    #[test]
    fn job_capacity_computed_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_arch(1, vec![ArchetypeTag::Commercial], 25)); // 10 jobs
        registry.register(make_arch(2, vec![ArchetypeTag::Industrial], 50));  // 5 jobs
        make_active(&mut entities, 1, 0);
        make_active(&mut entities, 2, 1);
        assert_eq!(compute_job_capacity(&entities, &registry), 15);
    }

    #[test]
    fn staffed_flag_set_correctly() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();
        registry.register(make_arch(1, vec![ArchetypeTag::Commercial], 25)); // 10 jobs
        let h1 = make_active(&mut entities, 1, 0);
        let h2 = make_active(&mut entities, 1, 1);

        tick_jobs(&mut entities, &registry, &mut events, 0, 15);
        assert!(entities.get_flags(h1).unwrap().contains(StatusFlags::STAFFED));
        assert!(!entities.get_flags(h2).unwrap().contains(StatusFlags::STAFFED));
    }

    #[test]
    fn unemployment_rate_calculation() {
        assert_eq!(unemployment_rate(10, 100), 10);
        assert_eq!(unemployment_rate(50, 100), 50);
        assert_eq!(unemployment_rate(0, 0), 0);
    }
}
