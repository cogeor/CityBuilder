//! Building development system.
//!
//! Converts zoned, empty land into actual structures over time. This is a
//! core city-builder loop: zone first, then let the simulation populate.

use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use crate::core::world::WorldState;
use crate::core_types::{EntityHandle, Tick, ZoneType};
use crate::math::rng::Rng;

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

pub fn tick_zoned_development(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    tick: Tick,
    rng: &mut Rng,
) -> u32 {
    tick_zoned_development_with_config(world, registry, tick, rng, DevelopmentConfig::default())
}

pub fn tick_zoned_development_with_config(
    world: &mut WorldState,
    registry: &ArchetypeRegistry,
    tick: Tick,
    rng: &mut Rng,
    config: DevelopmentConfig,
) -> u32 {
    if config.tick_interval == 0 || tick % config.tick_interval as u64 != 0 {
        return 0;
    }

    let map = world.map_size();
    let zone_candidates = collect_zone_archetypes(registry);
    let mut occupied = build_occupied_mask(world, registry);
    let mut placements = 0u32;

    for _ in 0..config.max_attempts_per_tick {
        if placements >= config.max_placements_per_tick as u32 {
            break;
        }

        let x = rng.next_bounded(map.width as u32) as i16;
        let y = rng.next_bounded(map.height as u32) as i16;
        let zone = match world.tiles.get(x, y) {
            Some(tile) => tile.zone,
            None => continue,
        };

        if zone == ZoneType::None || is_occupied(&occupied, map.width, x, y) {
            continue;
        }

        let archetype_ids = match zone_candidates_for(zone, &zone_candidates) {
            Some(ids) if !ids.is_empty() => ids,
            _ => continue,
        };

        let pick = rng.next_bounded(archetype_ids.len() as u32) as usize;
        let archetype_id = archetype_ids[pick];
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

        if let Some(handle) = world.place_entity(archetype_id, x, y, 0) {
            world.treasury -= def.cost_at_level(1);
            mark_occupied(&mut occupied, map.width, x, y, def.footprint_w, def.footprint_h);
            if !world.entities.is_valid(handle) {
                continue;
            }
            placements += 1;
        }
    }

    placements
}

#[derive(Default)]
struct ZoneArchetypes {
    residential: Vec<u16>,
    commercial: Vec<u16>,
    industrial: Vec<u16>,
    civic: Vec<u16>,
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
        if def.has_tag(ArchetypeTag::Residential) {
            out.residential.push(id);
        } else if def.has_tag(ArchetypeTag::Commercial) {
            out.commercial.push(id);
        } else if def.has_tag(ArchetypeTag::Industrial) {
            out.industrial.push(id);
        } else if def.has_tag(ArchetypeTag::Civic) {
            out.civic.push(id);
        }
    }
    out
}

fn zone_candidates_for(zone: ZoneType, candidates: &ZoneArchetypes) -> Option<&[u16]> {
    match zone {
        ZoneType::Residential => Some(&candidates.residential),
        ZoneType::Commercial => Some(&candidates.commercial),
        ZoneType::Industrial => Some(&candidates.industrial),
        ZoneType::Civic => Some(&candidates.civic),
        ZoneType::None => None,
    }
}

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
            if !world.tiles.in_bounds(tx, ty) {
                return false;
            }
            let Some(tile) = world.tiles.get(tx, ty) else {
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

        for y in 2..6 {
            for x in 2..6 {
                world.tiles.set_zone(x, y, ZoneType::Residential);
            }
        }

        let mut rng = Rng::new(42);
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
        );

        assert_eq!(placed, 0);
        assert_eq!(world.entities.count(), 0);
    }
}
