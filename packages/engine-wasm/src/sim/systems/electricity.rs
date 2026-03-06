//! Topology-aware electricity distribution via BFS flood-fill.
//!
//! ## Design
//!
//! Rather than flat supply/demand accounting, `propagate_power` performs a
//! breadth-first flood from every active power-source entity outward through
//! connected conductor tiles. A tile is a conductor when it carries
//! `TileFlags::CONDUCTOR` or its `TileKind` is `PowerLine`, `Road`, `Zone`,
//! or `Building` — mirroring the original SimCity rule that any occupied or
//! networked tile can carry current.
//!
//! `TileFlags::POWERED` serves a dual role:
//! - **Persistent tile state** — read by other systems to know whether a tile
//!   has power.
//! - **BFS visited marker** — a tile already flagged `POWERED` is never
//!   re-enqueued, so no `HashSet` is needed.
//!
//! The function first clears all `POWERED` flags (resetting both the state
//! and the visited set), then floods from source positions, and finally
//! computes aggregate kW totals across all alive, non-under-construction
//! entities.

use std::collections::VecDeque;

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::tilemap::{TileFlags, TileKind, TileValue};
use crate::core::world::WorldState;
use crate::core_types::StatusFlags;

// ─── PowerState ──────────────────────────────────────────────────────────────

/// Aggregate power accounting for one simulation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerState {
    /// Total generation capacity across all active, enabled power plants (kW).
    pub total_capacity_kw: u32,
    /// Total demand across all active, enabled consumers (kW).
    pub total_demand_kw: u32,
    /// How many kW of demand go unmet (`demand.saturating_sub(capacity)`).
    pub deficit_kw: u32,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Returns `true` when `tile` can carry electrical current.
///
/// A tile conducts when it explicitly carries `TileFlags::CONDUCTOR` **or**
/// its kind is one of `PowerLine`, `Road`, `Zone`, or `Building`.
#[inline]
fn tile_is_conductor(tile: TileValue) -> bool {
    tile.flags.contains(TileFlags::CONDUCTOR)
        || matches!(
            tile.kind,
            TileKind::PowerLine | TileKind::Road | TileKind::Zone | TileKind::Building
        )
}

// ─── propagate_power ─────────────────────────────────────────────────────────

/// Run one full power-propagation tick.
///
/// 1. **Clear** all `TileFlags::POWERED` flags from every tile (resets both
///    the power state and the BFS visited markers).
/// 2. **Accumulate** capacity/demand from all alive, non-under-construction
///    entities. Collect the tile positions of every enabled power source
///    (`power_supply_kw > 0`).
/// 3. **BFS flood** from each source position through adjacent conductor
///    tiles, setting `POWERED` as tiles are enqueued (prevents re-enqueue).
/// 4. Compute `deficit_kw` and return the [`PowerState`] summary.
pub fn propagate_power(world: &mut WorldState, registry: &ArchetypeRegistry) -> PowerState {
    // ── Phase 1: clear all POWERED flags ─────────────────────────────────
    let coords: Vec<(u32, u32)> = world.tiles.iter().map(|(x, y, _)| (x, y)).collect();
    for (x, y) in coords {
        world.tiles.clear_flags(x, y, TileFlags::POWERED);
    }

    // ── Phase 2: scan entities ────────────────────────────────────────────
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
                // Record the tile position of this source.
                let pos = world.entities.get_pos(handle);
                if let Some(coord) = pos {
                    if coord.x >= 0 && coord.y >= 0 {
                        sources.push((coord.x as u32, coord.y as u32));
                    }
                }
            }
        }
    }

    // ── Phase 3: BFS flood from sources ──────────────────────────────────
    let mut frontier: VecDeque<(u32, u32)> = VecDeque::new();

    for (sx, sy) in sources {
        if !world.tiles.in_bounds(sx, sy) {
            continue;
        }
        let tile = match world.tiles.get(sx, sy) {
            Some(t) => t,
            None => continue,
        };
        // Only start BFS from this source if the tile is a conductor and not
        // already POWERED (i.e. not already visited).
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

    // ── Phase 4: compute deficit ──────────────────────────────────────────
    let deficit_kw = total_demand_kw.saturating_sub(total_capacity_kw);

    PowerState {
        total_capacity_kw,
        total_demand_kw,
        deficit_kw,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
    use crate::core::entity::EntityStore;
    use crate::core::tilemap::{TileFlags, TileKind, TileMap, TileValue};
    use crate::core::world::{CityPolicies, WorldSeeds, WorldState};
    use crate::core_types::{MapSize, StatusFlags};

    /// Build a minimal `ArchetypeDefinition` for a power plant with the given supply.
    fn make_plant_archetype(id: u16, supply_kw: u32) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Plant {}", id),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 1,
            usable_ratio_pct: 80,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 10,
            power_demand_kw: 0,
            power_supply_kw: supply_kw,
            water_demand: 0,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 0,
            build_time_ticks: 100,
            max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 0,
        }
    }

    /// Create an active (non-under-construction) entity at the given tile position.
    fn make_active_entity(
        entities: &mut EntityStore,
        arch_id: u16,
        x: i16,
        y: i16,
    ) -> crate::core_types::EntityHandle {
        let h = entities.alloc(arch_id, x, y, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    /// Build a `WorldState` from pre-constructed `TileMap` and `EntityStore`.
    fn make_world(tiles: TileMap, entities: EntityStore) -> WorldState {
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

    // ── Test 1: source powers immediately adjacent conductor tiles ────────

    #[test]
    fn isolated_plant_powers_adjacent_tiles() {
        // 5x1 map.  Layout: [Empty] [Zone] [Plant/Building] [Zone] [Empty]
        // Only the three central tiles are conductors; the two endpoints are
        // plain Empty tiles with no CONDUCTOR flag, so power cannot reach them.
        let mut tiles = TileMap::new(5, 1);

        // Plant tile at (2,0)
        if let Some(t) = tiles.get_mut(2, 0) {
            t.kind = TileKind::Building;
            t.flags.insert(TileFlags::CONDUCTOR);
        }
        // Left neighbour at (1,0)
        if let Some(t) = tiles.get_mut(1, 0) {
            t.kind = TileKind::Zone;
            t.flags.insert(TileFlags::CONDUCTOR);
        }
        // Right neighbour at (3,0)
        if let Some(t) = tiles.get_mut(3, 0) {
            t.kind = TileKind::Zone;
            t.flags.insert(TileFlags::CONDUCTOR);
        }
        // Tiles (0,0) and (4,0) remain Empty with no CONDUCTOR — gap stops BFS.

        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_plant_archetype(1, 1000));
        make_active_entity(&mut entities, 1, 2, 0); // power plant at (2,0)

        let mut world = make_world(tiles, entities);
        propagate_power(&mut world, &registry);

        // Centre and immediate neighbours powered.
        assert!(
            world.tiles.get(2, 0).unwrap().flags.contains(TileFlags::POWERED),
            "tile (2,0) should be POWERED"
        );
        assert!(
            world.tiles.get(1, 0).unwrap().flags.contains(TileFlags::POWERED),
            "tile (1,0) should be POWERED"
        );
        assert!(
            world.tiles.get(3, 0).unwrap().flags.contains(TileFlags::POWERED),
            "tile (3,0) should be POWERED"
        );
        // Endpoints not powered — gap breaks propagation.
        assert!(
            !world.tiles.get(0, 0).unwrap().flags.contains(TileFlags::POWERED),
            "tile (0,0) must NOT be POWERED"
        );
        assert!(
            !world.tiles.get(4, 0).unwrap().flags.contains(TileFlags::POWERED),
            "tile (4,0) must NOT be POWERED"
        );
    }

    // ── Test 2: power lines bridge a gap between source and consumer ──────

    #[test]
    fn power_line_bridges_gap() {
        // 5x1 map: [Plant] [PwrLine] [PwrLine] [PwrLine] [Consumer]
        let mut tiles = TileMap::new(5, 1);

        // Power plant at (0,0)
        if let Some(t) = tiles.get_mut(0, 0) {
            t.kind = TileKind::Building;
            t.flags.insert(TileFlags::CONDUCTOR);
        }
        // Consumer building at (4,0)
        if let Some(t) = tiles.get_mut(4, 0) {
            t.kind = TileKind::Building;
            t.flags.insert(TileFlags::CONDUCTOR);
        }
        // Bridge tiles (1,0), (2,0), (3,0) as PowerLine + CONDUCTOR
        for x in 1..=3_u32 {
            if let Some(t) = tiles.get_mut(x, 0) {
                t.kind = TileKind::PowerLine;
                t.flags.insert(TileFlags::CONDUCTOR);
            }
        }

        let mut entities = EntityStore::new(16);
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_plant_archetype(1, 1000));
        make_active_entity(&mut entities, 1, 0, 0); // power plant at (0,0)

        let mut world = make_world(tiles, entities);
        propagate_power(&mut world, &registry);

        for x in 0..5_u32 {
            assert!(
                world.tiles.get(x, 0).unwrap().flags.contains(TileFlags::POWERED),
                "tile ({x},0) should be POWERED via power line bridge"
            );
        }
    }

    // ── Test 3: no power when there is no plant ───────────────────────────

    #[test]
    fn no_power_without_plant() {
        // 3x1 map, all Zone + CONDUCTOR, but zero source entities.
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

        assert_eq!(state.total_capacity_kw, 0, "no plants -> no capacity");
        for x in 0..3_u32 {
            assert!(
                !world.tiles.get(x, 0).unwrap().flags.contains(TileFlags::POWERED),
                "tile ({x},0) must NOT be POWERED"
            );
        }
    }

    // ── Test 4: stale POWERED flags are cleared before BFS ───────────────

    #[test]
    fn cleared_before_propagation() {
        // 3x1 map. Tile (2,0) gets POWERED pre-seeded but has no conductor
        // kind and no source. The function must clear it before BFS.
        let mut tiles = TileMap::new(3, 1);
        // Pre-seed a stale flag on a plain Empty tile.
        tiles.set_flags(2, 0, TileFlags::POWERED);

        let entities = EntityStore::new(16);
        let registry = ArchetypeRegistry::new();

        let mut world = make_world(tiles, entities);
        propagate_power(&mut world, &registry);

        assert!(
            !world.tiles.get(2, 0).unwrap().flags.contains(TileFlags::POWERED),
            "stale POWERED flag on tile (2,0) must have been cleared"
        );
    }
}
