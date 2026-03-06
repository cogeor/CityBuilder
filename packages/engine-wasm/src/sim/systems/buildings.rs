//! Building development system.
//!
//! Converts zoned, empty land into actual structures over time. This is a
//! core city-builder loop: zone first, then let the simulation populate.
//!
//! Development uses a persistent `StripeWalkIter` per `(ZoneType, ZoneDensity)`
//! pair so tiles fill in spatial order rather than random scatter, matching
//! the SimCity scan-line walk pattern.

use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use crate::core::world::WorldState;
use crate::core_types::{EntityHandle, StatusFlags, Tick, ZoneDensity, ZoneType};
use crate::math::rng::Rng;
use crate::sim::systems::stripe_walk::StripeWalkIter;

// ─── Development demand valves ────────────────────────────────────────────────

/// Demand signals that gate zoned development per zone type.
///
/// Mirrors SimCity's RValve/CValve/IValve without census-cycle indirection.
/// Positive = demand, zero/negative = surplus → no new development.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZoneDemand {
    /// Residential demand [-100..+100].
    pub residential: i16,
    /// Commercial demand [-100..+100].
    pub commercial: i16,
    /// Industrial demand [-100..+100].
    pub industrial: i16,
}

impl ZoneDemand {
    /// All zones at maximum demand (used when demand is unconstrained).
    pub const FULL: ZoneDemand = ZoneDemand {
        residential: 100,
        commercial: 100,
        industrial: 100,
    };

    /// Returns true if the given zone has positive demand.
    pub fn has_demand_for(&self, zone: ZoneType) -> bool {
        match zone {
            ZoneType::Residential => self.residential > 0,
            ZoneType::Commercial  => self.commercial > 0,
            ZoneType::Industrial  => self.industrial > 0,
            // Civic always develops regardless of demand
            ZoneType::Civic | ZoneType::Park | ZoneType::Transport | ZoneType::None => true,
        }
    }
}

// ─── Demand computation ───────────────────────────────────────────────────────

/// Compute `ZoneDemand` signals from the current city state.
///
/// Residential demand is driven by housing surplus/deficit. Commercial and
/// industrial demand are approximated from the population/job ratio. Growth
/// modifiers from `CityPolicies` tax rates map to the GROWTH_MODIFIERS table
/// defined in `plugins/base.economy/economy.ts` (0-20% tax range → ±50 pts).
///
/// Returns demand in the range [-100..+100] per zone type.
pub fn compute_zone_demand(world: &WorldState, registry: &ArchetypeRegistry, population: u32) -> ZoneDemand {
    // ── Housing capacity ────────────────────────────────────────────────
    let mut housing_cap: u32 = 0;
    let mut job_cap: u32 = 0;

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
        if def.has_tag(ArchetypeTag::Residential) {
            housing_cap += def.resident_capacity();
        } else if def.has_tag(ArchetypeTag::Commercial) || def.has_tag(ArchetypeTag::Industrial) {
            if def.workspace_per_job_m2 > 0 {
                job_cap += def.job_capacity();
            }
        }
    }

    // ── Residential demand ──────────────────────────────────────────────
    // Positive demand when there is a housing deficit (population > capacity).
    // Clamped to [-100..+100].
    let res_demand: i16 = if housing_cap == 0 {
        // No housing at all → maximum demand
        100
    } else if population > housing_cap {
        // Deficit: more people than houses → build more
        let deficit = (population - housing_cap).min(500) as i16;
        (deficit / 5).min(100)
    } else {
        // Surplus: more houses than people → no demand (or slight negative)
        let surplus = (housing_cap - population).min(500) as i16;
        -(surplus / 5).min(100)
    };

    // ── Commercial / Industrial demand ──────────────────────────────────
    // Proxy: ratio of population to job capacity drives commercial/industrial.
    let pop = population.max(1) as i16;
    let jobs = job_cap as i16;
    let ci_demand: i16 = if jobs == 0 {
        // No jobs yet → moderate demand when there's population
        (pop / 10).min(50)
    } else {
        // Demand proportional to pop/job imbalance
        let ratio_shortfall = pop.saturating_sub(jobs);
        (ratio_shortfall / 10).min(100).max(-100)
    };

    // ── Tax modifier (GROWTH_MODIFIERS: tax_rate 0.5..1.5x) ────────────
    // Residential tax 0% → +25 demand bonus; 20% → -25 demand penalty.
    // Maps tax_pct to [-25..+25] offset: offset = 25 - (tax_pct * 25 / 10)
    let res_tax_offset: i16 = 25 - (world.policies.residential_tax_pct as i16 * 25 / 10);
    let com_tax_offset: i16 = 25 - (world.policies.commercial_tax_pct as i16 * 25 / 10);
    let ind_tax_offset: i16 = 25 - (world.policies.industrial_tax_pct as i16 * 25 / 10);

    ZoneDemand {
        residential: (res_demand + res_tax_offset).clamp(-100, 100),
        commercial:  (ci_demand + com_tax_offset).clamp(-100, 100),
        industrial:  (ci_demand + ind_tax_offset).clamp(-100, 100),
    }
}

// ─── Development configuration ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct DevelopmentConfig {
    pub tick_interval: u32,
    pub max_attempts_per_tick: u16,
    pub max_placements_per_tick: u16,
}

impl Default for DevelopmentConfig {
    fn default() -> Self {
        DevelopmentConfig {
            tick_interval: 20,
            max_attempts_per_tick: 32,
            max_placements_per_tick: 4,
        }
    }
}

// ─── Development state (persistent across ticks) ─────────────────────────────

/// Zone × density index (4 zones × 3 densities = 12 slots).
/// Indexed as `zone_index * 3 + density_index`.
const WALKER_COUNT: usize = 4 * 3;

fn walker_index(zone: ZoneType, density: ZoneDensity) -> usize {
    let zi = match zone {
        ZoneType::Residential => 0,
        ZoneType::Commercial  => 1,
        ZoneType::Industrial  => 2,
        ZoneType::Civic       => 3,
        ZoneType::None | ZoneType::Park | ZoneType::Transport => return usize::MAX,
    };
    let di = density as usize;
    zi * 3 + di
}

/// Persistent iterators for each `(ZoneType, ZoneDensity)` combination.
///
/// Stored in `BuildingsPlugin` so the cursor survives across ticks.
pub struct DevelopmentState {
    walkers: [Option<StripeWalkIter>; WALKER_COUNT],
    map_width: u16,
    map_height: u16,
}

impl std::fmt::Debug for DevelopmentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DevelopmentState")
            .field("map_width", &self.map_width)
            .field("map_height", &self.map_height)
            .finish()
    }
}

impl DevelopmentState {
    pub fn new(map_width: u16, map_height: u16) -> Self {
        Self {
            walkers: std::array::from_fn(|_| None),
            map_width,
            map_height,
        }
    }

    fn walker_for(&mut self, zone: ZoneType, density: ZoneDensity) -> Option<&mut StripeWalkIter> {
        let idx = walker_index(zone, density);
        if idx == usize::MAX {
            return None;
        }
        let (w, h) = (self.map_width, self.map_height);
        Some(self.walkers[idx].get_or_insert_with(|| StripeWalkIter::new(w, h)))
    }
}

// ─── Public entry points ──────────────────────────────────────────────────────

pub fn tick_zoned_development(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    tick: Tick,
    rng: &mut Rng,
) -> u32 {
    let mut state = DevelopmentState::new(world.map_size().width, world.map_size().height);
    tick_zoned_development_with_config(world, registry, tick, rng, DevelopmentConfig::default(), &mut state, ZoneDemand::FULL)
}

pub fn tick_zoned_development_with_config(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    tick: Tick,
    _rng: &mut Rng,
    config: DevelopmentConfig,
    state: &mut DevelopmentState,
    demand: ZoneDemand,
) -> u32 {
    if config.tick_interval == 0 || tick % config.tick_interval as u64 != 0 {
        return 0;
    }

    let map_size = world.map_size();
    let zone_candidates = collect_zone_archetypes(registry);
    let mut occupied = build_occupied_mask(world, registry);
    let mut placements = 0u32;

    // Stripe-walk through each zone/density combination.
    // We iterate zones and densities, advancing each walker until we run out
    // of budget or map passes.
    const ZONE_DENSITY_PAIRS: [(ZoneType, ZoneDensity); 12] = [
        (ZoneType::Residential, ZoneDensity::Low),
        (ZoneType::Residential, ZoneDensity::Medium),
        (ZoneType::Residential, ZoneDensity::High),
        (ZoneType::Commercial,  ZoneDensity::Low),
        (ZoneType::Commercial,  ZoneDensity::Medium),
        (ZoneType::Commercial,  ZoneDensity::High),
        (ZoneType::Industrial,  ZoneDensity::Low),
        (ZoneType::Industrial,  ZoneDensity::Medium),
        (ZoneType::Industrial,  ZoneDensity::High),
        (ZoneType::Civic,       ZoneDensity::Low),
        (ZoneType::Civic,       ZoneDensity::Medium),
        (ZoneType::Civic,       ZoneDensity::High),
    ];

    'outer: for &(zone, density) in &ZONE_DENSITY_PAIRS {
        if placements >= config.max_placements_per_tick as u32 {
            break 'outer;
        }

        // Demand valve: skip zones with no demand
        if !demand.has_demand_for(zone) {
            continue;
        }

        let archetype_ids = match zone_candidates_for(zone, density, &zone_candidates) {
            Some(ids) if !ids.is_empty() => ids,
            _ => continue,
        };

        // Advance stripe walker to find next matching tile
        let Some(walker) = state.walker_for(zone, density) else {
            continue;
        };
        let Some((x, y)) = walker.next_zoned(&world.tiles, zone, density) else {
            continue;
        };

        if is_occupied(&occupied, map_size.width, x, y) {
            continue;
        }

        // Deterministic archetype pick: sorted IDs, first candidate per density tier
        let archetype_id = archetype_ids[0];
        let def = match registry.get(archetype_id) {
            Some(def) => def,
            None => continue,
        };

        if world.treasury < def.cost_at_level(1) {
            continue;
        }

        if !can_place_archetype(world, def, x, y, zone, &occupied) {
            continue;
        }

        if let Some(_handle) = world.place_entity(archetype_id, x, y, 0) {
            world.treasury -= def.cost_at_level(1);
            mark_occupied(&mut occupied, map_size.width, x, y, def.footprint_w, def.footprint_h);
            placements += 1;
        }
    }

    placements
}

// ─── Archetype bucketing ──────────────────────────────────────────────────────

#[derive(Default)]
struct ZoneArchetypes {
    residential: [Vec<u16>; 3], // indexed by ZoneDensity (0=Low, 1=Med, 2=High)
    commercial:  [Vec<u16>; 3],
    industrial:  [Vec<u16>; 3],
    civic:       [Vec<u16>; 3],
}

fn collect_zone_archetypes(registry: &ArchetypeRegistry) -> ZoneArchetypes {
    let mut out = ZoneArchetypes::default();
    for id in registry.list_ids() {
        let Some(def) = registry.get(id) else {
            continue;
        };
        if def.has_tag(ArchetypeTag::Utility) || def.has_tag(ArchetypeTag::Transport) {
            continue;
        }
        let di = density_index(def);
        if def.has_tag(ArchetypeTag::Residential) {
            out.residential[di].push(id);
        } else if def.has_tag(ArchetypeTag::Commercial) {
            out.commercial[di].push(id);
        } else if def.has_tag(ArchetypeTag::Industrial) {
            out.industrial[di].push(id);
        } else if def.has_tag(ArchetypeTag::Civic) {
            out.civic[di].push(id);
        }
    }
    out
}

/// Map archetype density tags to a slot index (0=Low, 1=Med, 2=High).
/// Defaults to Low if no density tag is present.
fn density_index(def: &ArchetypeDefinition) -> usize {
    if def.has_tag(ArchetypeTag::HighDensity) {
        2
    } else if def.has_tag(ArchetypeTag::MediumDensity) {
        1
    } else {
        0 // Low or untagged → Low
    }
}

fn zone_candidates_for<'a>(
    zone: ZoneType,
    density: ZoneDensity,
    candidates: &'a ZoneArchetypes,
) -> Option<&'a [u16]> {
    let di = density as usize;
    let slot = match zone {
        ZoneType::Residential => &candidates.residential[di],
        ZoneType::Commercial  => &candidates.commercial[di],
        ZoneType::Industrial  => &candidates.industrial[di],
        ZoneType::Civic       => &candidates.civic[di],
        ZoneType::None | ZoneType::Park | ZoneType::Transport => return None,
    };
    if slot.is_empty() {
        // Fall back to Low density if no archetypes for requested density tier
        let low_slot = match zone {
            ZoneType::Residential => &candidates.residential[0],
            ZoneType::Commercial  => &candidates.commercial[0],
            ZoneType::Industrial  => &candidates.industrial[0],
            ZoneType::Civic       => &candidates.civic[0],
            _ => return None,
        };
        if low_slot.is_empty() {
            None
        } else {
            Some(low_slot.as_slice())
        }
    } else {
        Some(slot.as_slice())
    }
}

// ─── Placement helpers ────────────────────────────────────────────────────────

fn can_place_archetype(
    world: &WorldState,
    def: &ArchetypeDefinition,
    x: i16,
    y: i16,
    zone: ZoneType,
    occupied: &[bool],
) -> bool {
    for dy in 0..def.footprint_h as i16 {
        for dx in 0..def.footprint_w as i16 {
            let tx = x + dx;
            let ty = y + dy;
            if tx < 0 || ty < 0 || !world.tiles.in_bounds(tx as u32, ty as u32) {
                return false;
            }
            let Some(tile) = world.tiles.get(tx as u32, ty as u32) else {
                return false;
            };
            if tile.zone != zone {
                return false;
            }
            if !world.is_buildable(tx, ty) {
                return false;
            }
            if is_occupied(occupied, world.map_size().width, tx, ty) {
                return false;
            }
        }
    }
    true
}

fn build_occupied_mask(world: &WorldState, registry: &ArchetypeRegistry) -> Vec<bool> {
    let size = world.map_size();
    let mut mask = vec![false; size.area() as usize];
    for handle in world.entities.iter_alive() {
        mark_entity_footprint(&mut mask, size.width, world, registry, handle);
    }
    mask
}

fn mark_entity_footprint(
    mask: &mut [bool],
    map_width: u16,
    world: &WorldState,
    registry: &ArchetypeRegistry,
    handle: EntityHandle,
) {
    let Some(pos) = world.entities.get_pos(handle) else {
        return;
    };
    let (w, h) = world
        .entities
        .get_archetype(handle)
        .and_then(|id| registry.get(id))
        .map(|def| (def.footprint_w, def.footprint_h))
        .unwrap_or((1, 1));
    mark_occupied(mask, map_width, pos.x, pos.y, w, h);
}

fn mark_occupied(mask: &mut [bool], map_width: u16, x: i16, y: i16, w: u8, h: u8) {
    for dy in 0..h as i16 {
        for dx in 0..w as i16 {
            let tx = x + dx;
            let ty = y + dy;
            if tx < 0 || ty < 0 {
                continue;
            }
            let idx = ty as usize * map_width as usize + tx as usize;
            if idx < mask.len() {
                mask[idx] = true;
            }
        }
    }
}

fn is_occupied(mask: &[bool], map_width: u16, x: i16, y: i16) -> bool {
    if x < 0 || y < 0 {
        return true;
    }
    let idx = y as usize * map_width as usize + x as usize;
    mask.get(idx).copied().unwrap_or(true)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::buildings::{register_base_city_builder_archetypes, ARCH_RES_SMALL_HOUSE};
    use crate::core::world::WorldState;
    use crate::core_types::MapSize;

    #[test]
    fn zoned_residential_area_develops_over_time() {
        let mut world = WorldState::new(MapSize::new(16, 16), 42);
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);

        for y in 2..6u32 {
            for x in 2..6u32 {
                world.tiles.set_zone(x, y, ZoneType::Residential);
            }
        }

        let mut rng = Rng::new(42);
        let mut state = DevelopmentState::new(16, 16);
        let placed = tick_zoned_development_with_config(
            &mut world,
            &registry,
            1,
            &mut rng,
            DevelopmentConfig {
                tick_interval: 1,
                max_attempts_per_tick: 128,
                max_placements_per_tick: 8,
            },
            &mut state,
            ZoneDemand::FULL,
        );

        assert!(placed > 0);
        assert!(world.entities.count() > 0);
        let ids: Vec<_> = world
            .entities
            .iter_alive()
            .filter_map(|h| world.entities.get_archetype(h))
            .collect();
        assert!(ids.contains(&ARCH_RES_SMALL_HOUSE));
    }

    #[test]
    fn no_development_when_no_zone_candidates_exist() {
        let mut world = WorldState::new(MapSize::new(8, 8), 7);
        let registry = ArchetypeRegistry::new();
        world.tiles.set_zone(1, 1, ZoneType::Residential);
        let mut rng = Rng::new(7);
        let mut state = DevelopmentState::new(8, 8);

        let placed = tick_zoned_development_with_config(
            &mut world,
            &registry,
            1,
            &mut rng,
            DevelopmentConfig {
                tick_interval: 1,
                max_attempts_per_tick: 16,
                max_placements_per_tick: 4,
            },
            &mut state,
            ZoneDemand::FULL,
        );

        assert_eq!(placed, 0);
        assert_eq!(world.entities.count(), 0);
    }

    #[test]
    fn demand_valve_blocks_residential_when_no_demand() {
        let mut world = WorldState::new(MapSize::new(16, 16), 1);
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);

        for y in 0..8u32 {
            for x in 0..8u32 {
                world.tiles.set_zone(x, y, ZoneType::Residential);
            }
        }

        let mut rng = Rng::new(1);
        let mut state = DevelopmentState::new(16, 16);
        let no_demand = ZoneDemand { residential: 0, commercial: 50, industrial: 50 };

        let placed = tick_zoned_development_with_config(
            &mut world,
            &registry,
            1,
            &mut rng,
            DevelopmentConfig { tick_interval: 1, max_attempts_per_tick: 64, max_placements_per_tick: 8 },
            &mut state,
            no_demand,
        );

        assert_eq!(placed, 0, "residential should not develop when demand is 0");
    }

    #[test]
    fn density_archetype_routing_high_density() {
        // Register an archetype tagged HighDensity + Residential
        use crate::core::archetypes::{ArchetypeDefinition, ArchetypeTag};
        let mut registry = ArchetypeRegistry::new();
        let def = ArchetypeDefinition {
            id: 999,
            name: "High Rise".to_string(),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::HighDensity],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 80,
            floors: 10,
            usable_ratio_pct: 80,
            base_cost_cents: 50_000,
            base_upkeep_cents_per_tick: 10,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 10,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 5,
            build_time_ticks: 100,
            max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 20,
        };
        registry.register(def);

        let candidates = collect_zone_archetypes(&registry);
        // High density slot should contain id 999
        assert!(candidates.residential[2].contains(&999));
        // Low and medium slots should be empty
        assert!(candidates.residential[0].is_empty());
        assert!(candidates.residential[1].is_empty());
    }
}
