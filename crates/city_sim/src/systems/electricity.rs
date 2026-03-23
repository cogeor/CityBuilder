//! Topology-aware electricity distribution via BFS flood-fill.

use std::collections::VecDeque;

use city_core::StatusFlags;
use crate::archetype::ArchetypeRegistry;

use crate::tilemap::{TileFlags, TileKind};
use crate::world::WorldState;

/// Aggregate power accounting for one simulation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerState {
    pub total_capacity_kw: u32,
    pub total_demand_kw: u32,
    pub deficit_kw: u32,
}

#[inline]
fn tile_is_conductor(tile: crate::tilemap::TileValue) -> bool {
    tile.flags.contains(TileFlags::CONDUCTOR)
        || matches!(
            tile.kind,
            TileKind::PowerLine | TileKind::Road | TileKind::Zone | TileKind::Building
        )
}

/// Run one full power-propagation tick via BFS flood-fill.
pub fn propagate_power(world: &mut WorldState, registry: &ArchetypeRegistry) -> PowerState {
    // Phase 1: clear all POWERED flags
    let coords: Vec<(u32, u32)> = world.tiles.iter().map(|(x, y, _)| (x, y)).collect();
    for (x, y) in coords {
        world.tiles.clear_flags(x, y, TileFlags::POWERED);
    }

    // Phase 2: scan entities
    let mut total_capacity_kw: u32 = 0;
    let mut total_demand_kw: u32 = 0;
    let mut sources: Vec<(u32, u32)> = Vec::new();

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
        let level = world.entities.get_level(handle).unwrap_or(1);
        let enabled = world.entities.get_enabled(handle).unwrap_or(true);

        if enabled {
            total_capacity_kw += def.power_supply_kw;
            total_demand_kw += def.power_demand_at_level(level);

            if def.power_supply_kw > 0 {
                if let Some(coord) = world.entities.get_pos(handle) {
                    if coord.x >= 0 && coord.y >= 0 {
                        sources.push((coord.x as u32, coord.y as u32));
                    }
                }
            }
        }
    }

    // Phase 3: BFS flood from sources
    let mut frontier: VecDeque<(u32, u32)> = VecDeque::new();

    for (sx, sy) in sources {
        if !world.tiles.in_bounds(sx, sy) { continue; }
        let tile = match world.tiles.get(sx, sy) {
            Some(t) => t,
            None => continue,
        };
        if tile_is_conductor(tile) && !tile.flags.contains(TileFlags::POWERED) {
            world.tiles.set_flags(sx, sy, TileFlags::POWERED);
            frontier.push_back((sx, sy));
        }
    }

    while let Some((x, y)) = frontier.pop_front() {
        for neighbour in world.tiles.tile_neighbors(x, y).into_iter().flatten() {
            let (nx, ny) = neighbour;
            let ntile = match world.tiles.get(nx, ny) {
                Some(t) => t,
                None => continue,
            };
            if tile_is_conductor(ntile) && !ntile.flags.contains(TileFlags::POWERED) {
                world.tiles.set_flags(nx, ny, TileFlags::POWERED);
                frontier.push_back((nx, ny));
            }
        }
    }

    let deficit_kw = total_demand_kw.saturating_sub(total_capacity_kw);
    PowerState { total_capacity_kw, total_demand_kw, deficit_kw }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::MapSize;
    use crate::archetype::{ArchetypeDefinition, ArchetypeTag};
    use city_engine::entity::EntityStore;

    use crate::tilemap::{TileKind, TileMap, TileFlags};
    use crate::world::{CityPolicies, WorldSeeds, WorldState};

    fn make_plant_arch(id: u16, supply_kw: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("Plant {}", id),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 1, usable_ratio_pct: 80,
            base_cost_cents: 500_000, base_upkeep_cents_per_tick: 10,
            power_demand_kw: 0, power_supply_kw: supply_kw,
            water_demand: 0, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 0, noise: 0,
            build_time_ticks: 100, max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: 0,
            effects: vec![],
            sprite_id: 0,
        }
    }

    fn make_active(entities: &mut EntityStore, arch_id: u16, x: i16, y: i16) -> city_core::EntityHandle {
        let h = entities.alloc(arch_id, x, y, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    fn make_world(tiles: TileMap, entities: EntityStore) -> WorldState {
        WorldState {
            tiles, entities,
            policies: CityPolicies::default(),
            seeds: WorldSeeds::new(0),
            tick: 0, treasury: 0,
            city_name: String::from("Test"),
        }
    }

    #[test]
    fn isolated_plant_powers_adjacent_tiles() {
        let mut tiles = TileMap::new(5, 1);
        if let Some(t) = tiles.get_mut(2, 0) { t.kind = TileKind::Building; t.flags.insert(TileFlags::CONDUCTOR); }
        if let Some(t) = tiles.get_mut(1, 0) { t.kind = TileKind::Zone; t.flags.insert(TileFlags::CONDUCTOR); }
        if let Some(t) = tiles.get_mut(3, 0) { t.kind = TileKind::Zone; t.flags.insert(TileFlags::CONDUCTOR); }

        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_plant_arch(1, 1000));
        make_active(&mut entities, 1, 2, 0);

        let mut world = make_world(tiles, entities);
        propagate_power(&mut world, &registry);

        assert!(world.tiles.get(2, 0).unwrap().flags.contains(TileFlags::POWERED));
        assert!(world.tiles.get(1, 0).unwrap().flags.contains(TileFlags::POWERED));
        assert!(world.tiles.get(3, 0).unwrap().flags.contains(TileFlags::POWERED));
        assert!(!world.tiles.get(0, 0).unwrap().flags.contains(TileFlags::POWERED));
        assert!(!world.tiles.get(4, 0).unwrap().flags.contains(TileFlags::POWERED));
    }

    #[test]
    fn no_power_without_plant() {
        let mut tiles = TileMap::new(3, 1);
        for x in 0..3_u32 {
            if let Some(t) = tiles.get_mut(x, 0) {
                t.kind = TileKind::Zone;
                t.flags.insert(TileFlags::CONDUCTOR);
            }
        }
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();
        let mut world = make_world(tiles, entities);
        let state = propagate_power(&mut world, &registry);

        assert_eq!(state.total_capacity_kw, 0);
        for x in 0..3_u32 {
            assert!(!world.tiles.get(x, 0).unwrap().flags.contains(TileFlags::POWERED));
        }
    }

    #[test]
    fn cleared_before_propagation() {
        let mut tiles = TileMap::new(3, 1);
        tiles.set_flags(2, 0, TileFlags::POWERED);
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();
        let mut world = make_world(tiles, entities);
        propagate_power(&mut world, &registry);
        assert!(!world.tiles.get(2, 0).unwrap().flags.contains(TileFlags::POWERED));
    }
}
