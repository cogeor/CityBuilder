//! Building development system.
//!
//! Converts zoned, empty land into actual structures over time.
//! Uses a persistent `StripeWalkIter` per `(ZoneType, ZoneDensity)`
//! pair so tiles fill in spatial order.

use city_engine::archetype::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};

use crate::caches::analysis_maps::AnalysisMaps;
use crate::math::rng::Rng;
use crate::systems::stripe_walk::StripeWalkIter;
use crate::systems::suitability::{tile_suitability, GROWTH_THRESHOLD};
use crate::types::{ZoneDensity, ZoneType};
use crate::world::WorldState;

use city_core::{EntityHandle, StatusFlags, Tick};

// ─── Demand valves ───────────────────────────────────────────────────────────

/// Demand signals that gate zoned development per zone type.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZoneDemand {
    pub residential: i16,
    pub commercial: i16,
    pub industrial: i16,
}

impl ZoneDemand {
    pub const FULL: ZoneDemand = ZoneDemand {
        residential: 100,
        commercial: 100,
        industrial: 100,
    };

    pub fn has_demand_for(&self, zone: ZoneType) -> bool {
        match zone {
            ZoneType::Residential => self.residential > 0,
            ZoneType::Commercial  => self.commercial > 0,
            ZoneType::Industrial  => self.industrial > 0,
            ZoneType::Civic | ZoneType::Park | ZoneType::Transport | ZoneType::None => true,
        }
    }
}

// ─── Demand computation ──────────────────────────────────────────────────────

/// Compute `ZoneDemand` signals from the current city state.
pub fn compute_zone_demand(
    world: &WorldState,
    registry: &ArchetypeRegistry,
    population: u32,
) -> ZoneDemand {
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

    let res_demand: i16 = if housing_cap == 0 {
        100
    } else if population > housing_cap {
        let deficit = (population - housing_cap).min(500) as i16;
        (deficit / 5).min(100)
    } else {
        let surplus = (housing_cap - population).min(500) as i16;
        -(surplus / 5).min(100)
    };

    let pop = population.max(1) as i16;
    let jobs = job_cap as i16;
    let ci_demand: i16 = if jobs == 0 {
        (pop / 10).min(50)
    } else {
        let ratio_shortfall = pop.saturating_sub(jobs);
        (ratio_shortfall / 10).min(100).max(-100)
    };

    let res_tax_offset: i16 = 25 - (world.policies.residential_tax_pct as i16 * 25 / 10);
    let com_tax_offset: i16 = 25 - (world.policies.commercial_tax_pct as i16 * 25 / 10);
    let ind_tax_offset: i16 = 25 - (world.policies.industrial_tax_pct as i16 * 25 / 10);

    ZoneDemand {
        residential: (res_demand + res_tax_offset).clamp(-100, 100),
        commercial:  (ci_demand + com_tax_offset).clamp(-100, 100),
        industrial:  (ci_demand + ind_tax_offset).clamp(-100, 100),
    }
}

// ─── Development configuration ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct DevelopmentConfig {
    pub tick_interval: u32,
    pub max_attempts_per_tick: u16,
    pub max_placements_per_tick: u16,
    pub suitability_threshold: i32,
}

impl Default for DevelopmentConfig {
    fn default() -> Self {
        DevelopmentConfig {
            tick_interval: 20,
            max_attempts_per_tick: 32,
            max_placements_per_tick: 4,
            suitability_threshold: GROWTH_THRESHOLD,
        }
    }
}

// ─── Development state ──────────────────────────────────────────────────────

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

// ─── Public entry points ─────────────────────────────────────────────────────

pub fn tick_zoned_development(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    tick: Tick,
    rng: &mut Rng,
) -> u32 {
    let mut state = DevelopmentState::new(world.map_size().width, world.map_size().height);
    tick_zoned_development_with_config(
        world, registry, tick, rng,
        DevelopmentConfig::default(), &mut state,
        ZoneDemand::FULL, None,
    )
}

pub fn tick_zoned_development_with_config(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    tick: Tick,
    _rng: &mut Rng,
    config: DevelopmentConfig,
    state: &mut DevelopmentState,
    demand: ZoneDemand,
    suitability_ctx: Option<(&AnalysisMaps, (i16, i16))>,
) -> u32 {
    if config.tick_interval == 0 || tick % config.tick_interval as u64 != 0 {
        return 0;
    }

    let map_size = world.map_size();
    let zone_candidates = collect_zone_archetypes(registry);
    let mut occupied = build_occupied_mask(world, registry);
    let mut placements = 0u32;

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

        if !demand.has_demand_for(zone) {
            continue;
        }

        let archetype_ids = match zone_candidates_for(zone, density, &zone_candidates) {
            Some(ids) if !ids.is_empty() => ids,
            _ => continue,
        };

        let Some(walker) = state.walker_for(zone, density) else {
            continue;
        };
        let Some((x, y)) = walker.next_zoned(&world.tiles, zone, density) else {
            continue;
        };

        if let Some((maps, city_center)) = suitability_ctx {
            let Some(tile) = world.tiles.get(x as u32, y as u32) else {
                continue;
            };
            match tile_suitability(&tile, x, y, maps, city_center, zone) {
                None => continue,
                Some(score) if score.total() <= config.suitability_threshold => continue,
                _ => {}
            }
        }

        if is_occupied(&occupied, map_size.width, x, y) {
            continue;
        }

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
            // Mark footprint tiles as Building so renderer shows building pattern
            for dy in 0..def.footprint_h as i16 {
                for dx in 0..def.footprint_w as i16 {
                    world.tiles.set_kind((x + dx) as u32, (y + dy) as u32, crate::tilemap::TileKind::Building);
                }
            }
            placements += 1;
        }
    }

    placements
}

// ─── Archetype bucketing ────────────────────────────────────────────────────

#[derive(Default)]
struct ZoneArchetypes {
    residential: [Vec<u16>; 3],
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

fn density_index(def: &ArchetypeDefinition) -> usize {
    if def.has_tag(ArchetypeTag::HighDensity) {
        2
    } else if def.has_tag(ArchetypeTag::MediumDensity) {
        1
    } else {
        0
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
        let low_slot = match zone {
            ZoneType::Residential => &candidates.residential[0],
            ZoneType::Commercial  => &candidates.commercial[0],
            ZoneType::Industrial  => &candidates.industrial[0],
            ZoneType::Civic       => &candidates.civic[0],
            _ => return None,
        };
        if low_slot.is_empty() { None } else { Some(low_slot.as_slice()) }
    } else {
        Some(slot.as_slice())
    }
}

// ─── Placement helpers ──────────────────────────────────────────────────────

fn can_place_archetype(
    world: &WorldState,
    def: &ArchetypeDefinition,
    x: i16,
    y: i16,
    zone: ZoneType,
    occupied: &[u64],
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

fn build_occupied_mask(world: &WorldState, registry: &ArchetypeRegistry) -> Vec<u64> {
    let size = world.map_size();
    let tiles = size.tile_count() as usize;
    let word_count = (tiles + 63) / 64;
    let mut mask = vec![0u64; word_count];
    for handle in world.entities.iter_alive() {
        mark_entity_footprint(&mut mask, size.width, world, registry, handle);
    }
    mask
}

fn mark_entity_footprint(
    mask: &mut [u64],
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

fn mark_occupied(mask: &mut [u64], map_width: u16, x: i16, y: i16, w: u8, h: u8) {
    for dy in 0..h as i16 {
        for dx in 0..w as i16 {
            let tx = x + dx;
            let ty = y + dy;
            if tx < 0 || ty < 0 {
                continue;
            }
            let idx = ty as usize * map_width as usize + tx as usize;
            let word = idx / 64;
            if word < mask.len() {
                mask[word] |= 1u64 << (idx % 64);
            }
        }
    }
}

#[inline]
fn is_occupied(mask: &[u64], map_width: u16, x: i16, y: i16) -> bool {
    if x < 0 || y < 0 {
        return true;
    }
    let idx = y as usize * map_width as usize + x as usize;
    let word = idx / 64;
    match mask.get(word) {
        Some(&w) => (w >> (idx % 64)) & 1 != 0,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_engine::archetype::ArchetypeDefinition;
    use city_core::MapSize;

    fn make_residential_arch(id: u16) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("House {}", id),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
            base_cost_cents: 10_000, base_upkeep_cents_per_tick: 10,
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

    fn make_commercial_arch(id: u16) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Shop {}", id),
            tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 1, usable_ratio_pct: 80,
            base_cost_cents: 15_000, base_upkeep_cents_per_tick: 5,
            power_demand_kw: 3, power_supply_kw: 0,
            water_demand: 1, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 1, desirability_magnitude: 2,
            pollution: 0, noise: 2,
            build_time_ticks: 300, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 25, living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    #[test]
    fn no_development_when_no_zone_candidates_exist() {
        let mut world = WorldState::new(MapSize::new(8, 8), 7);
        let registry = ArchetypeRegistry::new();
        world.tiles.set_zone(1, 1, ZoneType::Residential);
        let mut rng = Rng::new(7);
        let mut state = DevelopmentState::new(8, 8);

        let placed = tick_zoned_development_with_config(
            &mut world, &registry, 1, &mut rng,
            DevelopmentConfig { tick_interval: 1, max_attempts_per_tick: 16, max_placements_per_tick: 4, suitability_threshold: GROWTH_THRESHOLD },
            &mut state, ZoneDemand::FULL, None,
        );

        assert_eq!(placed, 0);
        assert_eq!(world.entities.count(), 0);
    }

    #[test]
    fn demand_valve_blocks_residential_when_no_demand() {
        let mut world = WorldState::new(MapSize::new(16, 16), 1);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_residential_arch(1));

        for y in 0..8u32 {
            for x in 0..8u32 {
                world.tiles.set_zone(x, y, ZoneType::Residential);
            }
        }

        let mut rng = Rng::new(1);
        let mut state = DevelopmentState::new(16, 16);
        let no_demand = ZoneDemand { residential: 0, commercial: 50, industrial: 50 };

        let placed = tick_zoned_development_with_config(
            &mut world, &registry, 1, &mut rng,
            DevelopmentConfig { tick_interval: 1, max_attempts_per_tick: 64, max_placements_per_tick: 8, suitability_threshold: GROWTH_THRESHOLD },
            &mut state, no_demand, None,
        );

        assert_eq!(placed, 0, "residential should not develop when demand is 0");
    }

    #[test]
    fn zoned_residential_area_develops() {
        let mut world = WorldState::new(MapSize::new(16, 16), 42);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_residential_arch(1));

        for y in 2..6u32 {
            for x in 2..6u32 {
                world.tiles.set_zone(x, y, ZoneType::Residential);
            }
        }

        let mut rng = Rng::new(42);
        let mut state = DevelopmentState::new(16, 16);
        let placed = tick_zoned_development_with_config(
            &mut world, &registry, 1, &mut rng,
            DevelopmentConfig { tick_interval: 1, max_attempts_per_tick: 128, max_placements_per_tick: 8, suitability_threshold: GROWTH_THRESHOLD },
            &mut state, ZoneDemand::FULL, None,
        );

        assert!(placed > 0);
        assert!(world.entities.count() > 0);
    }

    #[test]
    fn development_gated_by_tick_interval() {
        let mut world = WorldState::new(MapSize::new(16, 16), 5);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_residential_arch(1));

        for y in 0..8u32 {
            for x in 0..8u32 {
                world.tiles.set_zone(x, y, ZoneType::Residential);
            }
        }

        let mut rng = Rng::new(5);
        let mut state = DevelopmentState::new(16, 16);

        let placed = tick_zoned_development_with_config(
            &mut world, &registry, 1, &mut rng,
            DevelopmentConfig { tick_interval: 20, max_attempts_per_tick: 64, max_placements_per_tick: 8, suitability_threshold: GROWTH_THRESHOLD },
            &mut state, ZoneDemand::FULL, None,
        );
        assert_eq!(placed, 0, "should not develop on non-interval ticks");

        let placed20 = tick_zoned_development_with_config(
            &mut world, &registry, 20, &mut rng,
            DevelopmentConfig { tick_interval: 20, max_attempts_per_tick: 64, max_placements_per_tick: 8, suitability_threshold: GROWTH_THRESHOLD },
            &mut state, ZoneDemand::FULL, None,
        );
        assert!(placed20 > 0, "should develop on interval tick");
    }

    #[test]
    fn development_blocked_when_treasury_insufficient() {
        let mut world = WorldState::new(MapSize::new(16, 16), 3);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_residential_arch(1));

        for y in 0..8u32 {
            for x in 0..8u32 {
                world.tiles.set_zone(x, y, ZoneType::Residential);
            }
        }
        world.treasury = 0;

        let mut rng = Rng::new(3);
        let mut state = DevelopmentState::new(16, 16);
        let placed = tick_zoned_development_with_config(
            &mut world, &registry, 1, &mut rng,
            DevelopmentConfig { tick_interval: 1, max_attempts_per_tick: 64, max_placements_per_tick: 8, suitability_threshold: GROWTH_THRESHOLD },
            &mut state, ZoneDemand::FULL, None,
        );
        assert_eq!(placed, 0, "development blocked when treasury is empty");
    }

    #[test]
    fn density_archetype_routing_high_density() {
        let mut registry = ArchetypeRegistry::new();
        let def = ArchetypeDefinition {
            id: 999,
            name: "High Rise".to_string(),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::HighDensity],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 80, floors: 10, usable_ratio_pct: 80,
            base_cost_cents: 50_000, base_upkeep_cents_per_tick: 10,
            power_demand_kw: 50, power_supply_kw: 0,
            water_demand: 10, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 0, noise: 5,
            build_time_ticks: 100, max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: 20,
            effects: vec![],
        };
        registry.register(def);

        let candidates = collect_zone_archetypes(&registry);
        assert!(candidates.residential[2].contains(&999));
        assert!(candidates.residential[0].is_empty());
        assert!(candidates.residential[1].is_empty());
    }
}
