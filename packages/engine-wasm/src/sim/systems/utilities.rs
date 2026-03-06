//! Utility distribution system: power and water networks.
//!
//! Each tick, computes total supply vs. demand for power and water.
//! When supply >= demand, all buildings are satisfied.
//! When supply < demand, allocates by priority (civic > residential > commercial > industrial).
//!
//! Also provides `compute_water_coverage`, a BFS-based spatial system that marks
//! tiles within each water pump's `water_coverage_radius` as `TileFlags::WATERED`.

use std::collections::VecDeque;

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent, UtilityType};
use crate::core::tilemap::TileFlags;
use crate::core::world::WorldState;
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

/// Number of priority tiers — matches `AllocPriority` variant count.
const PRIORITY_BUCKETS: usize = 5;

/// Pre-allocated scratch buffers for zero-allocation utility distribution.
///
/// Store one instance per utility system (e.g. as a field on `ElectricitySystem`
/// or `WaterSystem`) and pass a mutable reference to `tick_power`/`tick_water`
/// each tick. Bucket Vecs grow on the first call and remain allocated thereafter.
#[derive(Debug, Default)]
pub struct UtilityDistributeScratch {
    /// One bucket per `AllocPriority`, ordered Civic → Residential → Commercial →
    /// Industrial → Other. Cleared at the start of every `distribute_utility` call.
    buckets: [Vec<(EntityHandle, u32)>; PRIORITY_BUCKETS],
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
///
/// `scratch` holds pre-allocated priority buckets so no heap allocation occurs
/// after the first call. Pass a persistent `UtilityDistributeScratch` stored on
/// the calling utility system to achieve zero per-tick allocations.
pub fn tick_power(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    prev_had_shortage: bool,
    scratch: &mut UtilityDistributeScratch,
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
        scratch,
    )
}

/// Distribute water to all active entities.
/// Updates HAS_WATER status flag on each entity.
///
/// `scratch` holds pre-allocated priority buckets. See `tick_power` docs.
pub fn tick_water(
    entities: &mut EntityStore,
    registry: &ArchetypeRegistry,
    events: &mut EventBus,
    tick: Tick,
    prev_had_shortage: bool,
    scratch: &mut UtilityDistributeScratch,
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
        scratch,
    )
}

/// Level multiplier: +20% per level above 1.
fn level_multiplier(level: u8) -> u32 {
    100 + (level.saturating_sub(1) as u32) * 20
}

/// Generic utility distribution — zero heap allocation per call after the first.
///
/// # Algorithm
/// **Pass 1** (supply scan): iterate all alive entities, accumulate `total_supply`,
/// bucket-sort consumers into `scratch.buckets[priority]` (no Vec sort).
/// **Pass 2** (allocation): drain buckets Civic→Other, set `satisfied_flag` /
/// clear it inline — no separate `satisfied_handles`/`unsatisfied_handles` Vecs.
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
    S: Fn(&crate::core::archetypes::ArchetypeDefinition, u8) -> u32,
    D: Fn(&crate::core::archetypes::ArchetypeDefinition, u8) -> u32,
{
    // Clear buckets (O(1) per bucket, no allocation).
    for bucket in &mut scratch.buckets {
        bucket.clear();
    }

    // Pass 1: compute total supply and fill priority buckets.
    // No `active: Vec` collect — iterate iter_alive() directly.
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

    // Pass 2: allocate supply in priority order, set flags inline.
    // No `satisfied_handles`/`unsatisfied_handles` Vecs needed.
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

    // Emit shortage / restoration events.
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

/// Aggregate water accounting for one simulation tick (spatial BFS model).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterState {
    /// Total water supply across all active, enabled water pumps.
    pub total_supply: u32,
    /// Total water demand across all active, enabled consumers.
    pub total_demand: u32,
    /// Unmet demand (`total_demand.saturating_sub(total_supply)`).
    pub deficit: u32,
}

/// Run one full water-coverage tick using BFS flood from each water pump.
///
/// 1. **Clear** all `TileFlags::WATERED` flags from every tile.
/// 2. **Scan** all alive, non-under-construction entities.  Collect the tile
///    positions of every enabled water pump (`water_supply > 0`) together with
///    its `water_coverage_radius`.  Accumulate `total_supply` and
///    `total_demand`.
/// 3. **BFS flood** from each pump position, propagating up to
///    `water_coverage_radius` steps away.  Every visited tile is marked
///    `TileFlags::WATERED`.  Distance is tracked in the queue as `(x, y, dist)`;
///    a tile is enqueued only when `dist < radius` so that the frontier
///    never expands beyond the radius.
/// 4. Compute `deficit` and return the [`WaterState`] summary.
pub fn compute_water_coverage(world: &mut WorldState, registry: &ArchetypeRegistry) -> WaterState {
    // ── Phase 1: clear all WATERED flags ─────────────────────────────────
    let coords: Vec<(u32, u32)> = world.tiles.iter().map(|(x, y, _)| (x, y)).collect();
    for (x, y) in coords {
        world.tiles.clear_flags(x, y, TileFlags::WATERED);
    }

    // ── Phase 2: scan entities ────────────────────────────────────────────
    let mut total_supply: u32 = 0;
    let mut total_demand: u32 = 0;
    // Each pump: (tile_x, tile_y, coverage_radius)
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
        if !enabled {
            continue;
        }

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

    // ── Phase 3: BFS flood from each pump within its coverage radius ──────
    // Queue entries: (x, y, current_dist, max_radius)
    // We run separate BFS waves per pump but share a single VecDeque for
    // efficiency.  The WATERED flag acts as the "visited" marker so tiles
    // already covered by one pump are skipped when reached by another.
    let mut frontier: VecDeque<(u32, u32, u8, u8)> = VecDeque::new();

    for (sx, sy, radius) in pumps {
        if !world.tiles.in_bounds(sx, sy) {
            continue;
        }
        // Mark source tile as WATERED if not already done.
        let already = world
            .tiles
            .get(sx, sy)
            .map(|t| t.flags.contains(TileFlags::WATERED))
            .unwrap_or(false);
        if !already {
            world.tiles.set_flags(sx, sy, TileFlags::WATERED);
        }
        // Enqueue at distance 0 with this pump's radius.
        frontier.push_back((sx, sy, 0, radius));
    }

    while let Some((x, y, dist, radius)) = frontier.pop_front() {
        // Only expand neighbours when we haven't reached the radius limit.
        if dist >= radius {
            continue;
        }
        for neighbour in world.tiles.tile_neighbors(x, y).into_iter().flatten() {
            let (nx, ny) = neighbour;
            let already_watered = world
                .tiles
                .get(nx, ny)
                .map(|t| t.flags.contains(TileFlags::WATERED))
                .unwrap_or(true);
            if !already_watered {
                world.tiles.set_flags(nx, ny, TileFlags::WATERED);
                frontier.push_back((nx, ny, dist + 1, radius));
            }
        }
    }

    // ── Phase 4: compute deficit ──────────────────────────────────────────
    let deficit = total_demand.saturating_sub(total_supply);

    WaterState {
        total_supply,
        total_demand,
        deficit,
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
            water_coverage_radius: 0,
            is_water_pipe: false,
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
            effects: vec![],
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

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

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

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

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

        let balance = tick_water(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

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

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

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

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

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
        let balance = tick_power(&mut entities, &registry, &mut events, 0, true, &mut UtilityDistributeScratch::default());

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
        tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());
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

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

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

        let balance = tick_power(&mut entities, &registry, &mut events, 0, false, &mut UtilityDistributeScratch::default());

        assert_eq!(balance.supply, 0);
        assert_eq!(balance.demand, 0);
        assert_eq!(balance.satisfied, 0);
        assert_eq!(balance.unsatisfied, 0);
    }

    // ─── Water Coverage (BFS) Tests ─────────────────────────────────────────

    use crate::core::tilemap::TileMap;
    use crate::core::world::{CityPolicies, WorldSeeds, WorldState};
    use crate::core_types::MapSize;

    /// Build a minimal pump `ArchetypeDefinition` with given coverage radius.
    fn make_pump_archetype(id: u16, water_supply: u32, water_coverage_radius: u8) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Pump {}", id),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 1,
            usable_ratio_pct: 80,
            base_cost_cents: 80_000,
            base_upkeep_cents_per_tick: 5,
            power_demand_kw: 0,
            power_supply_kw: 0,
            water_demand: 0,
            water_supply,
            water_coverage_radius,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 0,
            build_time_ticks: 100,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    fn make_world_state(tiles: TileMap, entities: EntityStore) -> WorldState {
        let size = MapSize::new(tiles.width() as u16, tiles.height() as u16);
        WorldState {
            tiles,
            entities,
            policies: CityPolicies::default(),
            seeds: WorldSeeds::new(0),
            tick: 0,
            treasury: 0,
            city_name: String::from("Test"),
        }
    }

    fn make_active_entity_2d(
        entities: &mut EntityStore,
        arch_id: u16,
        x: i16,
        y: i16,
    ) -> EntityHandle {
        let h = entities.alloc(arch_id, x, y, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    /// A pump with radius 2 should water the source tile and all tiles within
    /// 2 steps (Manhattan distance).
    #[test]
    fn pump_waters_tiles_within_radius() {
        // 7x1 map: pump at (3,0), radius 2 => tiles 1,2,3,4,5 watered; 0 and 6 not.
        let tiles = TileMap::new(7, 1);
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();

        registry.register(make_pump_archetype(1, 200, 2));
        make_active_entity_2d(&mut entities, 1, 3, 0);

        let mut world = make_world_state(tiles, entities);
        let state = compute_water_coverage(&mut world, &registry);

        assert_eq!(state.total_supply, 200);
        assert_eq!(state.total_demand, 0);
        assert_eq!(state.deficit, 0);

        // Tiles within radius 2 of (3,0): x = 1..=5
        for x in 1_u32..=5 {
            assert!(
                world.tiles.get(x, 0).unwrap().flags.contains(TileFlags::WATERED),
                "tile ({x},0) should be WATERED"
            );
        }
        // Tiles at distance 3: x=0 and x=6 must NOT be watered.
        assert!(
            !world.tiles.get(0, 0).unwrap().flags.contains(TileFlags::WATERED),
            "tile (0,0) must NOT be WATERED"
        );
        assert!(
            !world.tiles.get(6, 0).unwrap().flags.contains(TileFlags::WATERED),
            "tile (6,0) must NOT be WATERED"
        );
    }

    /// Stale WATERED flags from a previous tick must be cleared before BFS.
    #[test]
    fn stale_watered_flags_are_cleared() {
        let mut tiles = TileMap::new(5, 1);
        // Pre-seed a stale WATERED flag far from any pump.
        tiles.set_flags(4, 0, TileFlags::WATERED);

        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();

        let mut world = make_world_state(tiles, entities);
        compute_water_coverage(&mut world, &registry);

        assert!(
            !world.tiles.get(4, 0).unwrap().flags.contains(TileFlags::WATERED),
            "stale WATERED flag on tile (4,0) must have been cleared"
        );
    }

    /// With no pump entities, no tile should be WATERED.
    #[test]
    fn no_pump_no_watered_tiles() {
        let tiles = TileMap::new(4, 4);
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();

        let mut world = make_world_state(tiles, entities);
        let state = compute_water_coverage(&mut world, &registry);

        assert_eq!(state.total_supply, 0);
        for y in 0..4_u32 {
            for x in 0..4_u32 {
                assert!(
                    !world.tiles.get(x, y).unwrap().flags.contains(TileFlags::WATERED),
                    "tile ({x},{y}) must NOT be WATERED"
                );
            }
        }
    }

    /// Two pumps at opposite corners of a 5x1 map each with radius 1 should
    /// water their own adjacent tiles without crossing into the other's zone.
    #[test]
    fn two_pumps_cover_non_overlapping_zones() {
        // Layout: [Pump0] [.] [.] [.] [Pump1]  (5x1)
        // Pump at x=0 radius 1 => waters x=0,1
        // Pump at x=4 radius 1 => waters x=3,4
        // x=2 is in the middle — neither pump reaches it.
        let tiles = TileMap::new(5, 1);
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();

        registry.register(make_pump_archetype(1, 100, 1));
        make_active_entity_2d(&mut entities, 1, 0, 0);
        make_active_entity_2d(&mut entities, 1, 4, 0);

        let mut world = make_world_state(tiles, entities);
        let state = compute_water_coverage(&mut world, &registry);

        assert_eq!(state.total_supply, 200); // two pumps × 100

        assert!(world.tiles.get(0, 0).unwrap().flags.contains(TileFlags::WATERED), "tile (0,0)");
        assert!(world.tiles.get(1, 0).unwrap().flags.contains(TileFlags::WATERED), "tile (1,0)");
        assert!(!world.tiles.get(2, 0).unwrap().flags.contains(TileFlags::WATERED), "tile (2,0) must NOT be WATERED");
        assert!(world.tiles.get(3, 0).unwrap().flags.contains(TileFlags::WATERED), "tile (3,0)");
        assert!(world.tiles.get(4, 0).unwrap().flags.contains(TileFlags::WATERED), "tile (4,0)");
    }

    /// Under-construction pump should not contribute to coverage.
    #[test]
    fn under_construction_pump_not_counted() {
        let tiles = TileMap::new(5, 1);
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();

        registry.register(make_pump_archetype(1, 200, 3));
        // Allocate but do NOT clear UNDER_CONSTRUCTION.
        let _ = entities.alloc(1, 2, 0, 0).unwrap();

        let mut world = make_world_state(tiles, entities);
        let state = compute_water_coverage(&mut world, &registry);

        assert_eq!(state.total_supply, 0, "under-construction pump supplies nothing");
        for x in 0..5_u32 {
            assert!(
                !world.tiles.get(x, 0).unwrap().flags.contains(TileFlags::WATERED),
                "tile ({x},0) must NOT be WATERED"
            );
        }
    }
}
