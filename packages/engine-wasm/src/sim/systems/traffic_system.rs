//! Traffic density system.
//!
//! `traffic_system` computes `SimMap::TrafficDensity` using anchor-based trip
//! demand from `WorldVars`. Each road tile aggregates the trip demand of
//! residential and industrial entities in its 3×3 Chebyshev neighbourhood,
//! normalised against `ROAD_CAPACITY`.

use crate::core::archetypes::{ArchetypeRegistry, ArchetypeTag};
use crate::core::entity::EntityStore;
use crate::core::tilemap::{TileKind, TileMap};
use crate::core::world_vars::WorldVars;
use crate::core_types::StatusFlags;
use crate::sim::sim_map::{SimMap, SimMapRegistry};

/// Road tiles can handle this many normalised trip-units per tile per tick.
const ROAD_CAPACITY: f32 = 1000.0;

// ─── traffic_system ──────────────────────────────────────────────────────────

/// Compute `SimMap::TrafficDensity` for one tick.
///
/// ## Algorithm
///
/// 1. Build a `trip_demand` buffer (one `f32` per tile) from alive, completed
///    entities:
///    * **Residential** entities contribute
///      `resident_capacity × spending_mobility / needs_mobility_min`.
///    * **Industrial** entities contribute
///      `job_capacity × freight_trips_per_1000_per_day / 1000`.
///
/// 2. For each **Road** tile sum the trip demand in the 3×3 Chebyshev
///    neighbourhood (radius 1) and write
///    `(neighbourhood_sum / ROAD_CAPACITY).clamp(0, 1)` to the output map.
///    Non-road tiles always receive 0.
pub fn traffic_system(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    tile_map: &TileMap,
    world_vars: &WorldVars,
    maps: &mut SimMapRegistry,
) {
    maps.clear_next(SimMap::TrafficDensity);

    let width  = maps.width();
    let height = maps.height();

    // ── Step 1: accumulate trip demand per tile ───────────────────────────────

    let mut trip_demand: Vec<f32> = vec![0.0_f32; width * height];

    for handle in entities.iter_alive() {
        // Skip entities still under construction.
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

        let pos = match entities.get_pos(handle) {
            Some(p) => p,
            None => continue,
        };

        // Validate position is inside the map.
        let px = pos.x as i32;
        let py = pos.y as i32;
        if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
            continue;
        }

        let tile_idx = py as usize * width + px as usize;

        if def.has_tag(ArchetypeTag::Residential) {
            let demand = def.resident_capacity() as f32
                * world_vars.spending_mobility
                / world_vars.needs_mobility_min.max(f32::EPSILON);
            trip_demand[tile_idx] += demand;
        }

        if def.has_tag(ArchetypeTag::Industrial) {
            let freight = def.job_capacity() as f32
                * world_vars.freight_trips_per_1000_per_day
                / 1000.0;
            trip_demand[tile_idx] += freight;
        }
    }

    // ── Step 2: write density to road tiles ──────────────────────────────────

    let density_buf = maps.next_mut(SimMap::TrafficDensity);

    for ty in 0..height {
        for tx in 0..width {
            let tile_idx = ty * width + tx;

            // Only road tiles get a non-zero density value.
            let tile = match tile_map.get(tx as u32, ty as u32) {
                Some(t) => t,
                None => continue,
            };

            if tile.kind != TileKind::Road {
                density_buf[tile_idx] = 0.0;
                continue;
            }

            // Sum trip_demand in the 3×3 Chebyshev neighbourhood.
            let mut neighbourhood_demand = 0.0_f32;

            let x0 = (tx as i32 - 1).max(0) as usize;
            let x1 = (tx as i32 + 1).min(width as i32 - 1) as usize;
            let y0 = (ty as i32 - 1).max(0) as usize;
            let y1 = (ty as i32 + 1).min(height as i32 - 1) as usize;

            for ny in y0..=y1 {
                for nx in x0..=x1 {
                    neighbourhood_demand += trip_demand[ny * width + nx];
                }
            }

            density_buf[tile_idx] = (neighbourhood_demand / ROAD_CAPACITY).clamp(0.0, 1.0);
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
    use crate::core::entity::EntityStore;
    use crate::core::tilemap::{TileKind, TileMap, TileValue};
    use crate::core::world_vars::WorldVars;
    use crate::core_types::StatusFlags;
    use crate::sim::sim_map::{SimMap, SimMapRegistry};

    /// Build a minimal residential archetype definition.
    fn make_residential(id: crate::core_types::ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: "House".to_string(),
            tags: vec![ArchetypeTag::Residential],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 60,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 50_000,
            base_upkeep_cents_per_tick: 10,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 5,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 0,
            build_time_ticks: 1,
            max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 30, // gives non-zero resident_capacity
            effects: vec![],
        }
    }

    /// A road tile adjacent to a residential entity should have density > 0.
    #[test]
    fn road_adjacent_to_residential_has_nonzero_density() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_residential(1));

        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 5, 5, 0).unwrap();
        // Mark entity as completed (clear UNDER_CONSTRUCTION flag).
        entities.set_flags(h, StatusFlags::NONE);

        // 10×10 map: place a Road tile at (5, 6) — adjacent (south) to entity.
        let mut tile_map = TileMap::new(10, 10);
        let mut road_tile = TileValue::DEFAULT;
        road_tile.kind = TileKind::Road;
        tile_map.set(5, 6, road_tile);

        let world_vars = WorldVars::default();
        let mut maps = SimMapRegistry::new(10, 10);

        traffic_system(&entities, &registry, &tile_map, &world_vars, &mut maps);

        // Swap so we can read the freshly-written values.
        maps.swap();

        let idx = 6 * 10 + 5; // (5, 6)
        let density = maps.current(SimMap::TrafficDensity)[idx];
        assert!(
            density > 0.0,
            "expected TrafficDensity > 0.0 at road tile adjacent to residential, got {density}"
        );
    }

    /// With no entities all TrafficDensity values must be 0.
    #[test]
    fn empty_map_has_zero_density() {
        let registry = ArchetypeRegistry::new();
        let entities = EntityStore::new(64);

        // All-empty tile map (no roads, no buildings).
        let tile_map = TileMap::new(8, 8);
        let world_vars = WorldVars::default();
        let mut maps = SimMapRegistry::new(8, 8);

        traffic_system(&entities, &registry, &tile_map, &world_vars, &mut maps);
        maps.swap();

        let traffic = maps.current(SimMap::TrafficDensity);
        assert!(
            traffic.iter().all(|&v| v == 0.0_f32),
            "expected all TrafficDensity == 0.0 for empty map"
        );
    }
}
