//! `UtilitySystem` trait and built-in implementations.
//!
//! Replaces the free-function pair (`tick_power`, `tick_water`) with a
//! composable trait that `UtilityRegistry` drives uniformly.  Adding a new
//! utility type requires implementing the trait and registering it — no
//! changes to `SimulationEngine` are needed.
//!
//! ## Design
//! - Thin wrappers: `ElectricitySystem` and `WaterSystem` delegate to the
//!   existing `propagate_power`/`compute_water_coverage` free functions so no
//!   logic is duplicated.
//! - `HealthCareSystem` caches active hospital positions and radii and answers
//!   `tile_served` queries via Chebyshev-distance checks.

use crate::core::archetypes::{ArchetypeRegistry, ArchetypeTag};
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent, UtilityType};
use crate::core::world::WorldState;
use crate::core_types::*;
pub use crate::sim::systems::utilities::{UtilityBalance, UtilityDistributeScratch};

// ─── UtilitySystem trait ──────────────────────────────────────────────────────

/// One composable utility simulation (electricity, water, healthcare, …).
pub trait UtilitySystem: std::fmt::Debug {
    /// Stable name used as a key in `UtilityRegistry`.
    fn name(&self) -> &'static str;

    /// Run one tick of distribution / spatial propagation.
    ///
    /// Reads and writes world state as needed.  Returns the aggregate balance
    /// for this tick so the registry can carry shortage state forward.
    ///
    /// `prev_shortage` — whether the previous tick ended with a shortage
    /// (used to detect restoration).
    fn update(
        &mut self,
        world: &mut WorldState,
        registry: &ArchetypeRegistry,
        events: &mut EventBus,
        tick: Tick,
        prev_shortage: bool,
    ) -> UtilityBalance;

    /// Returns `true` if the tile at `pos` is currently served by this utility.
    ///
    /// For electricity and water the authoritative answer lives in
    /// `TileFlags::POWERED` / `TileFlags::WATERED` on the tile.  The default
    /// implementation returns `false`; concrete types override as needed.
    fn tile_served(&self, _pos: TileCoord) -> bool { false }

    /// Total generation / supply capacity after the last `update`.
    fn capacity(&self) -> u32;

    /// Total demand after the last `update`.
    fn demand(&self) -> u32;

    // ── Default derived methods ──────────────────────────────────────────

    /// Fraction of demand that is unmet, expressed in basis points (10000 = 100%).
    fn shortage_ratio(&self) -> u32 {
        let cap = self.capacity();
        let dem = self.demand();
        if dem == 0 { return 0; }
        dem.saturating_sub(cap) * 10_000 / dem
    }

    /// `true` when demand exceeds supply.
    fn has_shortage(&self) -> bool { self.demand() > self.capacity() }

    /// Point-in-time `UtilityBalance` snapshot derived from `capacity`/`demand`.
    fn metrics_snapshot(&self) -> UtilityBalance {
        let cap = self.capacity();
        let dem = self.demand();
        UtilityBalance {
            supply: cap,
            demand: dem,
            satisfied: cap.min(dem),
            unsatisfied: dem.saturating_sub(cap),
        }
    }
}

// ─── ElectricitySystem ────────────────────────────────────────────────────────

/// Thin wrapper around `propagate_power` BFS.
#[derive(Debug)]
pub struct ElectricitySystem {
    last_balance: UtilityBalance,
}

impl ElectricitySystem {
    pub fn new() -> Self {
        ElectricitySystem {
            last_balance: UtilityBalance { supply: 0, demand: 0, satisfied: 0, unsatisfied: 0 },
        }
    }
}

impl UtilitySystem for ElectricitySystem {
    fn name(&self) -> &'static str { "electricity" }

    fn update(
        &mut self,
        world: &mut WorldState,
        registry: &ArchetypeRegistry,
        _events: &mut EventBus,
        _tick: Tick,
        _prev_shortage: bool,
    ) -> UtilityBalance {
        use crate::sim::systems::electricity::propagate_power;
        let state = propagate_power(world, registry);
        self.last_balance = UtilityBalance {
            supply: state.total_capacity_kw,
            demand: state.total_demand_kw,
            satisfied: state.total_capacity_kw.min(state.total_demand_kw),
            unsatisfied: state.deficit_kw,
        };
        self.last_balance
    }

    fn capacity(&self) -> u32 { self.last_balance.supply }
    fn demand(&self)    -> u32 { self.last_balance.demand }
}

// ─── WaterSystem ──────────────────────────────────────────────────────────────

/// Thin wrapper around `compute_water_coverage` + `tick_water`.
#[derive(Debug)]
pub struct WaterSystem {
    last_balance: UtilityBalance,
    prev_shortage: bool,
    /// Pre-allocated scratch buffers so `tick_water` makes zero heap allocations
    /// per call after the first tick.
    scratch: UtilityDistributeScratch,
}

impl WaterSystem {
    pub fn new() -> Self {
        WaterSystem {
            last_balance: UtilityBalance { supply: 0, demand: 0, satisfied: 0, unsatisfied: 0 },
            prev_shortage: false,
            scratch: UtilityDistributeScratch::default(),
        }
    }
}

impl UtilitySystem for WaterSystem {
    fn name(&self) -> &'static str { "water" }

    fn update(
        &mut self,
        world: &mut WorldState,
        registry: &ArchetypeRegistry,
        events: &mut EventBus,
        tick: Tick,
        prev_shortage: bool,
    ) -> UtilityBalance {
        use crate::sim::systems::utilities::{compute_water_coverage, tick_water};
        let state = compute_water_coverage(world, registry);
        let balance = tick_water(&mut world.entities, registry, events, tick, prev_shortage, &mut self.scratch);
        self.last_balance = UtilityBalance {
            supply: state.total_supply,
            demand: state.total_demand,
            satisfied: state.total_supply.min(state.total_demand),
            unsatisfied: state.deficit,
        };
        self.prev_shortage = self.last_balance.has_shortage();
        // Return the tick_water balance for entity flag accuracy
        balance
    }

    fn capacity(&self) -> u32 { self.last_balance.supply }
    fn demand(&self)    -> u32 { self.last_balance.demand }
}

// ─── HealthCareSystem ─────────────────────────────────────────────────────────

/// Bed-count capacity, resident demand, and Chebyshev spatial coverage.
///
/// After each `update`, caches hospital tile positions and service radii so
/// `tile_served` can answer coverage queries without world access.
#[derive(Debug)]
pub struct HealthCareSystem {
    /// Cached `(tile_position, service_radius)` for active, completed hospitals.
    hospital_coverage: Vec<(TileCoord, u8)>,
    /// Total bed count (proxy: sum of job_capacity across active hospitals).
    total_beds: u32,
    /// Total demand: one bed per BED_RATIO residents.
    total_demand: u32,
}

/// 1 hospital bed per 200 residents.
const BED_RATIO: u32 = 200;

impl HealthCareSystem {
    pub fn new() -> Self {
        HealthCareSystem {
            hospital_coverage: Vec::new(),
            total_beds: 0,
            total_demand: 0,
        }
    }

    fn rebuild_coverage(
        entities: &EntityStore,
        registry: &ArchetypeRegistry,
    ) -> Vec<(TileCoord, u8)> {
        let mut out = Vec::new();
        for handle in entities.iter_alive() {
            let flags = match entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
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
            // Only buildings with a service_radius contribute to healthcare coverage.
            // Using the Civic+Service tag as proxy for healthcare providers.
            if def.service_radius > 0
                && (def.has_tag(ArchetypeTag::Service) || def.tags.iter().any(|t| format!("{t:?}") == "Civic"))
            {
                if let Some(pos) = entities.get_pos(handle) {
                    out.push((pos, def.service_radius));
                }
            }
        }
        out
    }
}

impl UtilitySystem for HealthCareSystem {
    fn name(&self) -> &'static str { "healthcare" }

    fn update(
        &mut self,
        world: &mut WorldState,
        registry: &ArchetypeRegistry,
        events: &mut EventBus,
        tick: Tick,
        prev_shortage: bool,
    ) -> UtilityBalance {
        self.hospital_coverage = Self::rebuild_coverage(&world.entities, registry);

        // Compute total bed capacity (job_capacity proxies hospital beds).
        let mut beds: u32 = 0;
        let mut resident_pop: u32 = 0;

        for handle in world.entities.iter_alive() {
            let flags = match world.entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
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
            if def.service_radius > 0 && def.workspace_per_job_m2 > 0
                && (def.has_tag(ArchetypeTag::Service) || def.has_tag(ArchetypeTag::Civic))
            {
                beds += def.job_capacity();
            }
            if def.has_tag(ArchetypeTag::Residential) {
                resident_pop += def.resident_capacity();
            }
        }

        self.total_beds = beds;
        self.total_demand = resident_pop / BED_RATIO;

        let has_shortage = self.total_demand > self.total_beds;

        // Emit shortage / restored events.
        if has_shortage && !prev_shortage {
            events.publish(
                tick,
                SimEvent::HealthCareShortage {
                    deficit: self.total_demand.saturating_sub(self.total_beds),
                },
            );
        } else if !has_shortage && prev_shortage {
            events.publish(tick, SimEvent::UtilityRestored { utility_type: UtilityType::HealthCare });
        }

        UtilityBalance {
            supply: self.total_beds,
            demand: self.total_demand,
            satisfied: self.total_beds.min(self.total_demand),
            unsatisfied: self.total_demand.saturating_sub(self.total_beds),
        }
    }

    /// Returns `true` if `pos` is within `service_radius` Chebyshev distance
    /// of any active, non-under-construction hospital.
    fn tile_served(&self, pos: TileCoord) -> bool {
        self.hospital_coverage.iter().any(|(hospital_pos, radius)| {
            let dx = (pos.x - hospital_pos.x).unsigned_abs() as u32;
            let dy = (pos.y - hospital_pos.y).unsigned_abs() as u32;
            dx.max(dy) <= *radius as u32
        })
    }

    fn capacity(&self) -> u32 { self.total_beds }
    fn demand(&self)    -> u32 { self.total_demand }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeTag};
    use crate::core::world::WorldState;
    use crate::core_types::MapSize;

    fn make_hospital_arch(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: "Hospital".to_string(),
            tags: vec![ArchetypeTag::Civic, ArchetypeTag::Service],
            footprint_w: 3,
            footprint_h: 3,
            coverage_ratio_pct: 60,
            floors: 4,
            usable_ratio_pct: 75,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 25,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 20,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 10,
            desirability_radius: 5,
            desirability_magnitude: 3,
            pollution: 0,
            noise: 5,
            build_time_ticks: 100,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 15,
            living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    #[test]
    fn healthcare_tile_served_within_radius() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_hospital_arch(1));

        let mut world = WorldState::new(MapSize::new(64, 64), 1);
        let handle = world.entities.alloc(1, 10, 10, 0).unwrap();
        // Mark as not under construction
        world.entities.set_flags(handle, StatusFlags::NONE);

        let mut system = HealthCareSystem::new();
        let mut events = EventBus::new();
        system.update(&mut world, &registry, &mut events, 1, false);

        // Tile at (10, 10) is the hospital — served
        assert!(system.tile_served(TileCoord::new(10, 10)));
        // Tile 10 chebyshev away — exactly at radius boundary — served
        assert!(system.tile_served(TileCoord::new(20, 10)));
        assert!(system.tile_served(TileCoord::new(10, 20)));
        // Tile 11 chebyshev away — outside radius — not served
        assert!(!system.tile_served(TileCoord::new(21, 10)));
    }

    #[test]
    fn healthcare_tile_not_served_outside_radius() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_hospital_arch(1));

        let mut world = WorldState::new(MapSize::new(64, 64), 1);
        let handle = world.entities.alloc(1, 5, 5, 0).unwrap();
        world.entities.set_flags(handle, StatusFlags::NONE);

        let mut system = HealthCareSystem::new();
        let mut events = EventBus::new();
        system.update(&mut world, &registry, &mut events, 1, false);

        // Tile 20 chebyshev away — beyond radius=10 — not served
        assert!(!system.tile_served(TileCoord::new(25, 25)));
    }

    #[test]
    fn healthcare_default_methods_work() {
        let system = HealthCareSystem::new();
        assert_eq!(system.capacity(), 0);
        assert_eq!(system.demand(), 0);
        assert!(!system.has_shortage());
        assert_eq!(system.shortage_ratio(), 0);
    }
}
