//! Generic network propagation kernel — BFS flood-fill for utility networks.

use std::collections::VecDeque;

use city_core::StatusFlags;
use crate::archetype::{ArchetypeDefinition, ArchetypeRegistry};
use crate::tilemap::{TileFlags, TileValue};
use crate::world::WorldState;

/// Configuration for a utility network propagation.
pub struct NetworkCfg {
    /// Name of this network (for debugging).
    pub name: &'static str,
    /// TileFlags bit to set on powered/connected tiles.
    pub flag_bit: TileFlags,
    /// Extract supply from an archetype definition.
    pub supply_fn: fn(&ArchetypeDefinition) -> u32,
    /// Extract demand from an archetype definition (at given level).
    pub demand_fn: fn(&ArchetypeDefinition, u8) -> u32,
    /// Check if a tile conducts this network.
    pub conductor_fn: fn(TileValue) -> bool,
}

/// Result of a network propagation pass.
#[derive(Debug, Clone, Copy)]
pub struct NetworkState {
    pub total_supply: u32,
    pub total_demand: u32,
    pub deficit: u32,
}

/// Run one BFS propagation pass for a utility network.
pub fn propagate_network(world: &mut WorldState, registry: &ArchetypeRegistry, cfg: &NetworkCfg) -> NetworkState {
    // Phase 1: clear flags
    let coords: Vec<(u32, u32)> = world.tiles.iter().map(|(x, y, _)| (x, y)).collect();
    for (x, y) in &coords {
        world.tiles.clear_flags(*x, *y, cfg.flag_bit);
    }

    // Phase 2: scan entities for supply/demand
    let mut total_supply: u32 = 0;
    let mut total_demand: u32 = 0;
    let mut sources: Vec<(u32, u32)> = Vec::new();

    let handles: Vec<_> = world.entities.iter_alive().collect();
    for handle in handles {
        let flags = world.entities.get_flags(handle).unwrap_or(StatusFlags::NONE);
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
        let arch_id = match world.entities.get_archetype(handle) { Some(id) => id, None => continue };
        let def = match registry.get(arch_id) { Some(d) => d, None => continue };
        let level = world.entities.get_level(handle).unwrap_or(1);
        let enabled = world.entities.get_enabled(handle).unwrap_or(true);

        if enabled {
            let supply = (cfg.supply_fn)(def);
            let demand = (cfg.demand_fn)(def, level);
            total_supply += supply;
            total_demand += demand;

            if supply > 0 {
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
        let tile = match world.tiles.get(sx, sy) { Some(t) => t, None => continue };
        if (cfg.conductor_fn)(tile) && !tile.flags.contains(cfg.flag_bit) {
            world.tiles.set_flags(sx, sy, cfg.flag_bit);
            frontier.push_back((sx, sy));
        }
    }

    while let Some((x, y)) = frontier.pop_front() {
        for (nx, ny) in world.tiles.tile_neighbors(x, y).into_iter().flatten() {
            let ntile = match world.tiles.get(nx, ny) { Some(t) => t, None => continue };
            if (cfg.conductor_fn)(ntile) && !ntile.flags.contains(cfg.flag_bit) {
                world.tiles.set_flags(nx, ny, cfg.flag_bit);
                frontier.push_back((nx, ny));
            }
        }
    }

    let deficit = total_demand.saturating_sub(total_supply);
    NetworkState { total_supply, total_demand, deficit }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::MapSize;
    use crate::archetype::{ArchetypeDefinition, ArchetypeTag};
    use city_engine::entity::EntityStore;
    use crate::tilemap::{TileKind, TileMap, TileFlags};
    use crate::world::{CityPolicies, WorldSeeds, WorldState};

    fn make_supply_arch(id: u16, supply: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("Source {}", id),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 1, usable_ratio_pct: 80,
            base_cost_cents: 500_000, base_upkeep_cents_per_tick: 10,
            power_demand_kw: 0, power_supply_kw: supply,
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

    fn make_world(tiles: TileMap, entities: EntityStore) -> WorldState {
        WorldState {
            tiles, entities,
            policies: CityPolicies::default(),
            seeds: WorldSeeds::new(0),
            tick: 0, treasury: 0,
            city_name: String::from("Test"),
        }
    }

    fn gas_cfg() -> NetworkCfg {
        NetworkCfg {
            name: "gas",
            flag_bit: TileFlags::POWERED, // reuse POWERED for test (no GAS_CONNECTED bit yet)
            supply_fn: |def| def.power_supply_kw,
            demand_fn: |def, _level| def.power_demand_kw,
            conductor_fn: |tile| matches!(tile.kind, TileKind::Road | TileKind::Building | TileKind::Zone),
        }
    }

    #[test]
    fn gas_network_propagates_from_source() {
        let mut tiles = TileMap::new(3, 1);
        for x in 0..3_u32 {
            if let Some(t) = tiles.get_mut(x, 0) { t.kind = TileKind::Road; }
        }
        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_supply_arch(1, 500));
        let h = entities.alloc(1, 0, 0, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);

        let mut world = make_world(tiles, entities);
        let cfg = gas_cfg();
        let state = propagate_network(&mut world, &registry, &cfg);

        assert_eq!(state.total_supply, 500);
        assert_eq!(state.deficit, 0);
        // Source tile and neighbours should be flagged
        assert!(world.tiles.get(0, 0).unwrap().flags.contains(TileFlags::POWERED));
        assert!(world.tiles.get(1, 0).unwrap().flags.contains(TileFlags::POWERED));
        assert!(world.tiles.get(2, 0).unwrap().flags.contains(TileFlags::POWERED));
    }

    #[test]
    fn gas_network_empty_world_no_panic() {
        let tiles = TileMap::new(4, 4);
        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();
        let mut world = make_world(tiles, entities);
        let cfg = gas_cfg();
        let state = propagate_network(&mut world, &registry, &cfg);
        assert_eq!(state.total_supply, 0);
        assert_eq!(state.total_demand, 0);
        assert_eq!(state.deficit, 0);
    }
}
