//! `UtilitySystem` trait and built-in implementations.

use city_core::{StatusFlags, TileCoord, Tick};
use crate::archetype::{ArchetypeRegistry, ArchetypeTag};
use city_engine::entity::EntityStore;

use crate::events::{EventBus, SimEvent, UtilityType};
use crate::world::WorldState;
pub use crate::systems::utilities::{UtilityBalance, UtilityDistributeScratch};

/// One composable utility simulation (electricity, water, healthcare, …).
pub trait UtilitySystem: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;

    fn update(
        &mut self,
        world: &mut WorldState,
        registry: &ArchetypeRegistry,
        events: &mut EventBus,
        tick: Tick,
        prev_shortage: bool,
    ) -> UtilityBalance;

    fn tile_served(&self, _pos: TileCoord) -> bool { false }
    fn capacity(&self) -> u32;
    fn demand(&self) -> u32;

    fn shortage_ratio(&self) -> u32 {
        let cap = self.capacity();
        let dem = self.demand();
        if dem == 0 { return 0; }
        dem.saturating_sub(cap) * 10_000 / dem
    }

    fn has_shortage(&self) -> bool { self.demand() > self.capacity() }

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

// ─── ElectricitySystem ──────────────────────────────────────────────────────

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
        use crate::systems::electricity::propagate_power;
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

// ─── WaterSystem ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct WaterSystem {
    last_balance: UtilityBalance,
    prev_shortage: bool,
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
        use crate::systems::utilities::{compute_water_coverage, tick_water};
        let state = compute_water_coverage(world, registry);
        let balance = tick_water(&mut world.entities, registry, events, tick, prev_shortage, &mut self.scratch);
        self.last_balance = UtilityBalance {
            supply: state.total_supply,
            demand: state.total_demand,
            satisfied: state.total_supply.min(state.total_demand),
            unsatisfied: state.deficit,
        };
        self.prev_shortage = self.last_balance.has_shortage();
        balance
    }

    fn capacity(&self) -> u32 { self.last_balance.supply }
    fn demand(&self)    -> u32 { self.last_balance.demand }
}

// ─── HealthCareSystem ───────────────────────────────────────────────────────

/// 1 hospital bed per 200 residents.
const BED_RATIO: u32 = 200;

#[derive(Debug)]
pub struct HealthCareSystem {
    hospital_coverage: Vec<(TileCoord, u8)>,
    total_beds: u32,
    total_demand: u32,
}

impl HealthCareSystem {
    pub fn new() -> Self {
        HealthCareSystem {
            hospital_coverage: Vec::new(),
            total_beds: 0,
            total_demand: 0,
        }
    }

    fn rebuild_coverage_in_place(
        coverage: &mut Vec<(TileCoord, u8)>,
        entities: &EntityStore,
        registry: &ArchetypeRegistry,
    ) {
        coverage.clear();
        for handle in entities.iter_alive() {
            let flags = match entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
            let arch_id = match entities.get_archetype(handle) {
                Some(id) => id,
                None => continue,
            };
            let def = match registry.get(arch_id) {
                Some(d) => d,
                None => continue,
            };
            if def.service_radius > 0
                && (def.has_tag(ArchetypeTag::Service) || def.has_tag(ArchetypeTag::Civic))
            {
                if let Some(pos) = entities.get_pos(handle) {
                    coverage.push((pos, def.service_radius));
                }
            }
        }
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
        Self::rebuild_coverage_in_place(&mut self.hospital_coverage, &world.entities, registry);

        let mut beds: u32 = 0;
        let mut resident_pop: u32 = 0;

        for handle in world.entities.iter_alive() {
            let flags = match world.entities.get_flags(handle) {
                Some(f) => f,
                None => continue,
            };
            if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
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

        if has_shortage && !prev_shortage {
            events.publish(tick, SimEvent::HealthCareShortage {
                deficit: self.total_demand.saturating_sub(self.total_beds),
            });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthcare_default_methods_work() {
        let system = HealthCareSystem::new();
        assert_eq!(system.capacity(), 0);
        assert_eq!(system.demand(), 0);
        assert!(!system.has_shortage());
        assert_eq!(system.shortage_ratio(), 0);
    }

    #[test]
    fn electricity_default_metrics() {
        let system = ElectricitySystem::new();
        assert_eq!(system.capacity(), 0);
        assert_eq!(system.demand(), 0);
        let snap = system.metrics_snapshot();
        assert_eq!(snap.supply, 0);
        assert_eq!(snap.demand, 0);
    }
}
