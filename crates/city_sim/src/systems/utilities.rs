//! Utility distribution system: power and water networks.
//!
//! Computes total supply vs. demand for power and water.
//! When supply < demand, allocates by priority (civic > residential > commercial > industrial).
//! Also provides `compute_water_coverage`, a BFS-based spatial system.

use std::collections::VecDeque;

use city_core::{EntityHandle, StatusFlags, Tick};
use crate::archetype::{ArchetypeRegistry, ArchetypeTag};
use city_engine::entity::EntityStore;

use crate::events::{EventBus, SimEvent, UtilityType};
use crate::tilemap::TileFlags;
use crate::world::WorldState;

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

const PRIORITY_BUCKETS: usize = 5;

/// Pre-allocated scratch buffers for zero-allocation utility distribution.
#[derive(Debug, Default)]
pub struct UtilityDistributeScratch {
    buckets: [Vec<(EntityHandle, u32)>; PRIORITY_BUCKETS],
}

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

/// Distribute power to all active entities. Updates POWERED status flag.
pub fn tick_power(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    prev_had_shortage: bool,
    scratch: &mut UtilityDistributeScratch,
) -> UtilityBalance {
    distribute_utility(
        entities, registry, events, tick, prev_had_shortage,
        UtilityType::Power,
        |def, level| def.power_supply_kw * level_multiplier(level) / 100,
        |def, level| def.power_demand_at_level(level),
        StatusFlags::POWERED,
        scratch,
    )
}

/// Distribute water to all active entities. Updates WATER_CONNECTED status flag.
pub fn tick_water(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    prev_had_shortage: bool,
    scratch: &mut UtilityDistributeScratch,
) -> UtilityBalance {
    distribute_utility(
        entities, registry, events, tick, prev_had_shortage,
        UtilityType::Water,
        |def, level| def.water_supply * level_multiplier(level) / 100,
        |def, _level| def.water_demand,
        StatusFlags::WATER_CONNECTED,
        scratch,
    )
}

fn level_multiplier(level: u8) -> u32 {
    100 + (level.saturating_sub(1) as u32) * 20
}

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
    scratch: &mut UtilityDistributeScratch,
) -> UtilityBalance
where
    S: Fn(&crate::archetype::ArchetypeDefinition, u8) -> u32,
    D: Fn(&crate::archetype::ArchetypeDefinition, u8) -> u32,
{
    for bucket in &mut scratch.buckets {
        bucket.clear();
    }

    let mut total_supply: u32 = 0;
    let mut total_demand: u32 = 0;

    for handle in entities.iter_alive() {
        let flags = entities.get_flags(handle).unwrap_or(StatusFlags::NONE);
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
        let enabled = entities.get_enabled(handle).unwrap_or(true);

        let supply = if enabled { supply_fn(def, level) } else { 0 };
        let demand = if enabled { demand_fn(def, level) } else { 0 };

        total_supply += supply;

        if demand > 0 {
            let priority = entity_priority(entities, registry, handle);
            scratch.buckets[priority as usize].push((handle, demand));
            total_demand += demand;
        }
    }

    let mut remaining_supply = total_supply;
    let mut satisfied_count: u32 = 0;
    let mut unsatisfied_count: u32 = 0;

    for bucket in &scratch.buckets {
        for &(handle, demand) in bucket {
            let got_supply = remaining_supply >= demand;
            if got_supply {
                remaining_supply -= demand;
                satisfied_count += 1;
            } else {
                unsatisfied_count += 1;
            }
            if let Some(mut flags) = entities.get_flags(handle) {
                if got_supply {
                    flags.insert(satisfied_flag);
                } else {
                    flags.remove(satisfied_flag);
                }
                entities.set_flags(handle, flags);
            }
        }
    }

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
            UtilityType::HealthCare => {
                events.publish(tick, SimEvent::HealthCareShortage { deficit });
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

// ─── Water Coverage (BFS spatial) ───────────────────────────────────────────

/// Aggregate water accounting for one simulation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterState {
    pub total_supply: u32,
    pub total_demand: u32,
    pub deficit: u32,
}

/// Run one full water-coverage tick using BFS flood from each water pump.
pub fn compute_water_coverage(world: &mut WorldState, registry: &ArchetypeRegistry) -> WaterState {
    // Phase 1: clear all WATERED flags
    let coords: Vec<(u32, u32)> = world.tiles.iter().map(|(x, y, _)| (x, y)).collect();
    for (x, y) in coords {
        world.tiles.clear_flags(x, y, TileFlags::WATERED);
    }

    // Phase 2: scan entities
    let mut total_supply: u32 = 0;
    let mut total_demand: u32 = 0;
    let mut pumps: Vec<(u32, u32, u8)> = Vec::new();

    let handles: Vec<_> = world.entities.iter_alive().collect();
    for handle in handles {
        let flags = world.entities.get_flags(handle).unwrap_or(StatusFlags::NONE);
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
            continue;
        }
        let arch_id = match world.entities.get_archetype(handle) {
            Some(id) => id,
            None => continue,
        };
        let def = match registry.get(arch_id) {
            Some(d) => d,
            None => continue,
        };
        let enabled = world.entities.get_enabled(handle).unwrap_or(true);
        if !enabled { continue; }

        total_supply += def.water_supply;
        total_demand += def.water_demand;

        if def.water_supply > 0 && def.water_coverage_radius > 0 {
            if let Some(coord) = world.entities.get_pos(handle) {
                if coord.x >= 0 && coord.y >= 0 {
                    pumps.push((coord.x as u32, coord.y as u32, def.water_coverage_radius));
                }
            }
        }
    }

    // Phase 3: BFS flood from each pump within its coverage radius
    let mut frontier: VecDeque<(u32, u32, u8, u8)> = VecDeque::new();

    for (sx, sy, radius) in pumps {
        if !world.tiles.in_bounds(sx, sy) { continue; }
        let already = world.tiles.get(sx, sy)
            .map(|t| t.flags.contains(TileFlags::WATERED))
            .unwrap_or(false);
        if !already {
            world.tiles.set_flags(sx, sy, TileFlags::WATERED);
        }
        frontier.push_back((sx, sy, 0, radius));
    }

    while let Some((x, y, dist, radius)) = frontier.pop_front() {
        if dist >= radius { continue; }
        for neighbour in world.tiles.tile_neighbors(x, y).into_iter().flatten() {
            let (nx, ny) = neighbour;
            let already_watered = world.tiles.get(nx, ny)
                .map(|t| t.flags.contains(TileFlags::WATERED))
                .unwrap_or(true);
            if !already_watered {
                world.tiles.set_flags(nx, ny, TileFlags::WATERED);
                frontier.push_back((nx, ny, dist + 1, radius));
            }
        }
    }

    let deficit = total_demand.saturating_sub(total_supply);
    WaterState { total_supply, total_demand, deficit }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::MapSize;
    use crate::archetype::{ArchetypeDefinition, ArchetypeTag, Prerequisite};
    use city_engine::entity::EntityStore;

    use crate::tilemap::TileMap;
    use crate::world::{CityPolicies, WorldSeeds, WorldState};

    fn make_archetype(
        id: u16, tags: Vec<ArchetypeTag>,
        power_supply: u32, power_demand: u32,
        water_supply: u32, water_demand: u32,
    ) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("Archetype {}", id), tags,
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 1, usable_ratio_pct: 80,
            base_cost_cents: 10_000, base_upkeep_cents_per_tick: 1,
            power_demand_kw: power_demand, power_supply_kw: power_supply,
            water_demand, water_supply,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 0, noise: 0,
            build_time_ticks: 100, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: 40,
            effects: vec![],
        }
    }

    fn make_active(entities: &mut EntityStore, arch_id: u16, x: i16) -> EntityHandle {
        let h = entities.alloc(arch_id, x, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    #[test]
    fn power_surplus_all_satisfied() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(1, vec![ArchetypeTag::Utility], 1000, 0, 0, 0));
        registry.register(make_archetype(2, vec![ArchetypeTag::Residential], 0, 5, 0, 0));

        make_active(&mut entities, 1, 0);
        let h1 = make_active(&mut entities, 2, 1);
        let h2 = make_active(&mut entities, 2, 2);

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

        assert_eq!(balance.supply, 1000);
        assert_eq!(balance.demand, 10);
        assert_eq!(balance.satisfied, 2);
        assert!(!balance.has_shortage());
        assert!(entities.get_flags(h1).unwrap().contains(StatusFlags::POWERED));
        assert!(entities.get_flags(h2).unwrap().contains(StatusFlags::POWERED));
        assert!(events.is_empty());
    }

    #[test]
    fn power_shortage_priority_allocation() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(1, vec![ArchetypeTag::Utility], 10, 0, 0, 0));
        registry.register(make_archetype(2, vec![ArchetypeTag::Civic], 0, 5, 0, 0));
        registry.register(make_archetype(3, vec![ArchetypeTag::Industrial], 0, 10, 0, 0));

        make_active(&mut entities, 1, 0);
        let civic = make_active(&mut entities, 2, 1);
        let industrial = make_active(&mut entities, 3, 2);

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

        assert!(balance.has_shortage());
        assert!(entities.get_flags(civic).unwrap().contains(StatusFlags::POWERED));
        assert!(!entities.get_flags(industrial).unwrap().contains(StatusFlags::POWERED));
    }

    #[test]
    fn water_distribution() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(1, vec![ArchetypeTag::Utility], 0, 0, 100, 0));
        registry.register(make_archetype(2, vec![ArchetypeTag::Residential], 0, 0, 0, 2));

        make_active(&mut entities, 1, 0);
        let h = make_active(&mut entities, 2, 1);

        let balance = tick_water(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

        assert!(!balance.has_shortage());
        assert!(entities.get_flags(h).unwrap().contains(StatusFlags::WATER_CONNECTED));
    }

    #[test]
    fn under_construction_excluded() {
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        registry.register(make_archetype(1, vec![ArchetypeTag::Utility], 1000, 0, 0, 0));
        registry.register(make_archetype(2, vec![ArchetypeTag::Residential], 0, 5, 0, 0));

        make_active(&mut entities, 1, 0);
        let h = entities.alloc(2, 1, 0, 0).unwrap(); // still under construction

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());
        assert_eq!(balance.demand, 0);
        assert!(!entities.get_flags(h).unwrap().contains(StatusFlags::POWERED));
    }

    #[test]
    fn empty_world_no_panic() {
        let mut entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let mut events = EventBus::new();

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());
        assert_eq!(balance.supply, 0);
        assert_eq!(balance.demand, 0);
    }

    // ─── Water Coverage BFS tests ───────────────────────────────────────────

    fn make_pump_arch(id: u16, water_supply: u32, radius: u8) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("Pump {}", id),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 1, usable_ratio_pct: 80,
            base_cost_cents: 80_000, base_upkeep_cents_per_tick: 5,
            power_demand_kw: 0, power_supply_kw: 0,
            water_demand: 0, water_supply,
            water_coverage_radius: radius, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 0, noise: 0,
            build_time_ticks: 100, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    fn make_world_state(tiles: TileMap, entities: EntityStore) -> WorldState {
        WorldState {
            tiles, entities,
            policies: CityPolicies::default(),
            seeds: WorldSeeds::new(0),
            tick: 0, treasury: 0,
            city_name: String::from("Test"),
        }
    }

    fn make_active_2d(entities: &mut EntityStore, arch_id: u16, x: i16, y: i16) -> EntityHandle {
        let h = entities.alloc(arch_id, x, y, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    #[test]
    fn pump_waters_tiles_within_radius() {
        let tiles = TileMap::new(7, 1);
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();

        registry.register(make_pump_arch(1, 200, 2));
        make_active_2d(&mut entities, 1, 3, 0);

        let mut world = make_world_state(tiles, entities);
        let state = compute_water_coverage(&mut world, &registry);

        assert_eq!(state.total_supply, 200);
        for x in 1_u32..=5 {
            assert!(world.tiles.get(x, 0).unwrap().flags.contains(TileFlags::WATERED));
        }
        assert!(!world.tiles.get(0, 0).unwrap().flags.contains(TileFlags::WATERED));
        assert!(!world.tiles.get(6, 0).unwrap().flags.contains(TileFlags::WATERED));
    }

    #[test]
    fn no_pump_no_watered() {
        let tiles = TileMap::new(4, 4);
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();

        let mut world = make_world_state(tiles, entities);
        let state = compute_water_coverage(&mut world, &registry);

        assert_eq!(state.total_supply, 0);
        for y in 0..4_u32 {
            for x in 0..4_u32 {
                assert!(!world.tiles.get(x, y).unwrap().flags.contains(TileFlags::WATERED));
            }
        }
    }
}
