//! Transport / traffic approximation system.
//!
//! Approximates commuter traffic by pairing residential entities with nearby
//! commercial/industrial entities. Density is tracked per-tile on a u16 grid.
//! Congestion is detected when density exceeds road capacity, and TrafficJam
//! events are emitted for severely congested tiles.

use crate::core::archetypes::{ArchetypeRegistry, ArchetypeTag};
use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent};
use crate::core::network::RoadGraph;
use crate::core::tilemap::{TileFlags, TileMap};
use crate::core_types::*;

/// Default road capacity used for tiles without a road.
const DEFAULT_CAPACITY: u16 = 200;

/// Multiplier for severe congestion threshold (density > capacity * this).
const TRAFFIC_JAM_MULTIPLIER: u16 = 2;

/// Aggregated traffic statistics for a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrafficStats {
    /// Total commuter trips generated this tick.
    pub total_trips: u32,
    /// Number of road segments where density exceeds capacity.
    pub congested_segments: u32,
    /// Average density across all tiles that have any traffic.
    pub avg_density: u16,
}

/// Per-tile traffic density grid. Each tile stores a u16 density value.
#[derive(Debug)]
pub struct TrafficGrid {
    data: Vec<u16>,
    width: u16,
    height: u16,
}

impl TrafficGrid {
    /// Create a new traffic grid initialized to zero.
    pub fn new(width: u16, height: u16) -> Self {
        TrafficGrid {
            data: vec![0u16; width as usize * height as usize],
            width,
            height,
        }
    }

    /// Get the density at a tile coordinate. Returns 0 for out-of-bounds.
    #[inline]
    pub fn get(&self, x: i16, y: i16) -> u16 {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return 0;
        }
        self.data[y as usize * self.width as usize + x as usize]
    }

    /// Set the density at a tile coordinate. No-op for out-of-bounds.
    #[inline]
    pub fn set(&mut self, x: i16, y: i16, value: u16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        self.data[y as usize * self.width as usize + x as usize] = value;
    }

    /// Add to the density at a tile coordinate (saturating). No-op for out-of-bounds.
    #[inline]
    pub fn add(&mut self, x: i16, y: i16, amount: u16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        let idx = y as usize * self.width as usize + x as usize;
        self.data[idx] = self.data[idx].saturating_add(amount);
    }

    /// Clear all density values to zero.
    pub fn clear(&mut self) {
        for v in self.data.iter_mut() {
            *v = 0;
        }
    }

    /// Grid width.
    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Grid height.
    #[inline]
    pub fn height(&self) -> u16 {
        self.height
    }
}

/// Get the road capacity for a tile. If the tile has a road, returns the
/// highest-capacity road type at that position. Otherwise returns the default.
fn road_capacity_at(road_graph: &RoadGraph, pos: TileCoord) -> u16 {
    if !road_graph.has_road_at(pos) {
        return DEFAULT_CAPACITY;
    }
    let neighbors = road_graph.neighbors(pos);
    let mut max_cap: u32 = 0;
    for &(_neighbor, road_type) in neighbors {
        let cap = road_type.capacity();
        if cap > max_cap {
            max_cap = cap;
        }
    }
    if max_cap == 0 {
        DEFAULT_CAPACITY
    } else {
        max_cap.min(u16::MAX as u32) as u16
    }
}

/// Run one tick of the transport system.
///
/// Only performs heavy recomputation on Transport phase ticks (tick % 4 == 0).
/// On other ticks, returns zeroed stats without modifying the grid.
///
/// For each residential entity (not under construction), finds the nearest
/// commercial or industrial entity by manhattan distance, and increments
/// density on both the home tile and the work tile. Each resident generates
/// one trip.
///
/// Congested segments are tiles where density exceeds the road capacity.
/// TrafficJam events are emitted for tiles where density exceeds 2x capacity.
pub fn tick_transport(
    traffic_grid: &mut TrafficGrid,
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    road_graph: &RoadGraph,
    tile_map: &TileMap,
    events: &mut EventBus,
    tick: Tick,
    map_size: MapSize,
) -> TrafficStats {
    // Only do heavy recomputation on Transport phase ticks.
    if tick % 4 != 0 {
        return TrafficStats {
            total_trips: 0,
            congested_segments: 0,
            avg_density: 0,
        };
    }

    // Clear grid for fresh computation.
    traffic_grid.clear();

    // Collect residential and work (commercial/industrial) positions.
    let mut residential_positions: Vec<TileCoord> = Vec::new();
    let mut work_positions: Vec<TileCoord> = Vec::new();

    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) {
            Some(f) => f,
            None => continue,
        };

        // Skip entities under construction.
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

        if def.has_tag(ArchetypeTag::Residential) {
            // Only count trips from residential entities whose tile has road access.
            let has_road = tile_map
                .get(pos.x as u32, pos.y as u32)
                .map(|t| t.flags.contains(TileFlags::ROAD_ACCESS))
                .unwrap_or(false);
            if has_road {
                let cap = def.resident_capacity();
                for _ in 0..cap {
                    residential_positions.push(pos);
                }
            }
        }

        if def.has_tag(ArchetypeTag::Commercial) || def.has_tag(ArchetypeTag::Industrial) {
            work_positions.push(pos);
        }
    }

    let mut total_trips: u32 = 0;

    // For each residential trip, find nearest work destination.
    if !work_positions.is_empty() {
        for &home_pos in &residential_positions {
            // Find nearest work position by manhattan distance.
            let mut best_dist = u32::MAX;
            let mut best_work = work_positions[0];
            for &work_pos in &work_positions {
                let dist = home_pos.manhattan_distance(&work_pos);
                if dist < best_dist {
                    best_dist = dist;
                    best_work = work_pos;
                }
            }

            // Increment density at home and work tiles.
            traffic_grid.add(home_pos.x, home_pos.y, 1);
            if home_pos != best_work {
                traffic_grid.add(best_work.x, best_work.y, 1);
            }
            total_trips += 1;
        }
    }

    // Compute congestion stats and emit events.
    let mut congested_segments: u32 = 0;
    let mut density_sum: u64 = 0;
    let mut density_count: u32 = 0;

    for y in 0..map_size.height as i16 {
        for x in 0..map_size.width as i16 {
            let density = traffic_grid.get(x, y);
            if density == 0 {
                continue;
            }

            density_sum += density as u64;
            density_count += 1;

            let pos = TileCoord::new(x, y);
            let capacity = road_capacity_at(road_graph, pos);

            if density > capacity {
                congested_segments += 1;
            }

            // Emit TrafficJam for severe congestion (density > 2x capacity).
            if density > capacity.saturating_mul(TRAFFIC_JAM_MULTIPLIER) {
                events.publish(
                    tick,
                    SimEvent::TrafficJam {
                        location: pos,
                        density,
                    },
                );
            }
        }
    }

    let avg_density = if density_count > 0 {
        (density_sum / density_count as u64) as u16
    } else {
        0
    };

    TrafficStats {
        total_trips,
        congested_segments,
        avg_density,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::ArchetypeDefinition;
    use crate::core::tilemap::TileMap;

    /// Helper: create a residential archetype.
    fn make_residential(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("House {}", id),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 100_000,
            base_upkeep_cents_per_tick: 10,
            power_demand_kw: 5,
            power_supply_kw: 0,
            water_demand: 2,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2,
            desirability_magnitude: 5,
            pollution: 0,
            noise: 1,
            build_time_ticks: 500,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
        }
    }

    /// Helper: create a commercial archetype.
    fn make_commercial(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Shop {}", id),
            tags: vec![ArchetypeTag::Commercial],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 50,
            floors: 1,
            usable_ratio_pct: 80,
            base_cost_cents: 80_000,
            base_upkeep_cents_per_tick: 15,
            power_demand_kw: 10,
            power_supply_kw: 0,
            water_demand: 3,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 3,
            desirability_magnitude: 3,
            pollution: 0,
            noise: 2,
            build_time_ticks: 300,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 25,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create an industrial archetype.
    fn make_industrial(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: format!("Factory {}", id),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 60,
            floors: 1,
            usable_ratio_pct: 70,
            base_cost_cents: 120_000,
            base_upkeep_cents_per_tick: 20,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 5,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 4,
            desirability_magnitude: -10,
            pollution: 5,
            noise: 4,
            build_time_ticks: 800,
            max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 30,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create an active (not under construction) entity.
    fn make_active(entities: &mut EntityStore, arch_id: ArchetypeId, x: i16, y: i16) -> EntityHandle {
        let h = entities.alloc(arch_id, x, y, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    // ─── Test 1: Empty world no panic ────────────────────────────────────

    #[test]
    fn empty_world_no_panic() {
        let mut grid = TrafficGrid::new(16, 16);
        let entities = EntityStore::new(64);
        let registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        assert_eq!(stats.total_trips, 0);
        assert_eq!(stats.congested_segments, 0);
        assert_eq!(stats.avg_density, 0);
        assert!(events.is_empty());
    }

    // ─── Test 2: Traffic grid get/set works ──────────────────────────────

    #[test]
    fn traffic_grid_get_set_works() {
        let mut grid = TrafficGrid::new(10, 10);

        // Initially zero.
        assert_eq!(grid.get(0, 0), 0);
        assert_eq!(grid.get(5, 5), 0);

        // Set and retrieve.
        grid.set(3, 4, 42);
        assert_eq!(grid.get(3, 4), 42);

        // Out-of-bounds returns 0.
        assert_eq!(grid.get(-1, 0), 0);
        assert_eq!(grid.get(10, 0), 0);
        assert_eq!(grid.get(0, 10), 0);

        // Add works.
        grid.add(3, 4, 8);
        assert_eq!(grid.get(3, 4), 50);

        // Clear works.
        grid.clear();
        assert_eq!(grid.get(3, 4), 0);
    }

    // ─── Test 3: Single residential adds density ─────────────────────────

    #[test]
    fn single_residential_adds_density() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        // Place house at (2, 2) and shop at (5, 5).
        make_active(&mut entities, 1, 2, 2);
        make_active(&mut entities, 2, 5, 5);

        // Grant road access to the home tile so trips are counted.
        tile_map.set_flags(2, 2, TileFlags::ROAD_ACCESS);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        // The residential archetype has capacity 6 residents (1x1, 50% cov, 2 floors, 40 m2/person).
        // Each resident generates 1 trip -> 6 trips.
        assert_eq!(stats.total_trips, 6);

        // Density at home tile (2,2) should be 6.
        assert_eq!(grid.get(2, 2), 6);
        // Density at work tile (5,5) should be 6.
        assert_eq!(grid.get(5, 5), 6);
    }

    // ─── Test 4: Congestion detected correctly ───────────────────────────

    #[test]
    fn congestion_detected_correctly() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(256);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        // Place many residential buildings at the same tile to exceed default capacity (200).
        // Each house = 6 residents. We need > 200 density, so 34 houses = 204.
        for i in 0..34 {
            make_active(&mut entities, 1, 2, 2);
            // Need at least one commercial building.
            if i == 0 {
                make_active(&mut entities, 2, 3, 3);
            }
        }

        // Grant road access to the home tile so all 34 houses generate trips.
        tile_map.set_flags(2, 2, TileFlags::ROAD_ACCESS);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        // Density at (2,2) should be 34*6 = 204 which is > 200 (default capacity).
        assert_eq!(grid.get(2, 2), 204);
        assert!(stats.congested_segments > 0);
    }

    // ─── Test 5: TrafficJam event emitted ────────────────────────────────

    #[test]
    fn traffic_jam_event_emitted() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(512);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        // Need density > 2 * 200 = 400 at one tile.
        // 68 houses at same tile = 68*6 = 408 density.
        for i in 0..68 {
            make_active(&mut entities, 1, 2, 2);
            if i == 0 {
                make_active(&mut entities, 2, 3, 3);
            }
        }

        // Grant road access so all 68 houses generate trips.
        tile_map.set_flags(2, 2, TileFlags::ROAD_ACCESS);

        let _stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        // Should have at least one TrafficJam event.
        let drained = events.drain();
        let jams: Vec<_> = drained
            .iter()
            .filter(|e| matches!(e.event, SimEvent::TrafficJam { .. }))
            .collect();
        assert!(!jams.is_empty(), "TrafficJam event should be emitted for severe congestion");

        // Verify the jam is at the correct location.
        if let SimEvent::TrafficJam { location, density } = &jams[0].event {
            assert_eq!(*location, TileCoord::new(2, 2));
            assert!(*density > 400);
        }
    }

    // ─── Test 6: Non-transport tick does nothing ─────────────────────────

    #[test]
    fn non_transport_tick_does_nothing() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));
        make_active(&mut entities, 1, 2, 2);
        make_active(&mut entities, 2, 5, 5);

        // Ticks 1, 2, 3 are not Transport phase (tick % 4 != 0).
        for tick in 1..4 {
            let stats = tick_transport(
                &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, tick, map_size,
            );
            assert_eq!(stats.total_trips, 0);
            assert_eq!(stats.congested_segments, 0);
            assert_eq!(stats.avg_density, 0);
        }
    }

    // ─── Test 7: Under-construction excluded ─────────────────────────────

    #[test]
    fn under_construction_excluded() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        // One active residential.
        make_active(&mut entities, 1, 2, 2);
        // One under construction residential (default from alloc).
        let _h_uc = entities.alloc(1, 3, 3, 0).unwrap();
        // One active commercial.
        make_active(&mut entities, 2, 5, 5);

        // Grant road access to the active home tile so its residents generate trips.
        tile_map.set_flags(2, 2, TileFlags::ROAD_ACCESS);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        // Only the active residential building's residents should generate trips.
        // Active house has 6 residents -> 6 trips.
        assert_eq!(stats.total_trips, 6);
        // Under-construction house at (3,3) should have 0 density.
        assert_eq!(grid.get(3, 3), 0);
    }

    // ─── Test 8: Stats computed correctly ────────────────────────────────

    #[test]
    fn stats_computed_correctly() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        // Place 2 houses at different positions and 1 shop.
        make_active(&mut entities, 1, 0, 0);
        make_active(&mut entities, 1, 4, 4);
        make_active(&mut entities, 2, 2, 2);

        // Grant road access to both home tiles.
        tile_map.set_flags(0, 0, TileFlags::ROAD_ACCESS);
        tile_map.set_flags(4, 4, TileFlags::ROAD_ACCESS);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        // 2 houses * 6 residents each = 12 trips.
        assert_eq!(stats.total_trips, 12);

        // avg_density should be > 0 since there is traffic.
        assert!(stats.avg_density > 0);

        // No congestion: each tile has at most 12 density, well under 200 capacity.
        assert_eq!(stats.congested_segments, 0);
    }

    // ─── Test 9: Road capacity affects congestion detection ──────────────

    #[test]
    fn road_capacity_affects_congestion() {
        use crate::core::network::RoadType;

        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(256);
        let mut registry = ArchetypeRegistry::new();
        let mut road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        // Add Highway roads at both home tile (2,2) and work tile (5,5).
        road_graph.add_segment(TileCoord::new(2, 2), TileCoord::new(2, 3), RoadType::Highway);
        road_graph.add_segment(TileCoord::new(5, 5), TileCoord::new(5, 6), RoadType::Highway);

        // Place 34 houses at (2,2) = 204 density. With highway (cap 2000), no congestion
        // at either tile despite density exceeding the default Local capacity of 200.
        for _ in 0..34 {
            make_active(&mut entities, 1, 2, 2);
        }
        make_active(&mut entities, 2, 5, 5);

        // Grant road access to the home tile so all 34 houses generate trips.
        tile_map.set_flags(2, 2, TileFlags::ROAD_ACCESS);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        // Both tiles have highway capacity 2000, density 204 -> not congested.
        assert_eq!(stats.congested_segments, 0);
    }

    // ─── Test 10: Industrial entities serve as work destinations ─────────

    #[test]
    fn industrial_entities_as_work_destinations() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_industrial(3));

        // Place house at (1, 1) and factory at (5, 5).
        make_active(&mut entities, 1, 1, 1);
        make_active(&mut entities, 3, 5, 5);

        // Grant road access to the home tile.
        tile_map.set_flags(1, 1, TileFlags::ROAD_ACCESS);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        // 6 residents -> 6 trips to the factory.
        assert_eq!(stats.total_trips, 6);
        assert_eq!(grid.get(1, 1), 6); // home density
        assert_eq!(grid.get(5, 5), 6); // work density
    }

    // ─── Test 11: No work destinations means no trips ────────────────────

    #[test]
    fn no_work_destinations_no_trips() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));

        // Only residential, no commercial or industrial.
        make_active(&mut entities, 1, 2, 2);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        assert_eq!(stats.total_trips, 0);
        assert_eq!(grid.get(2, 2), 0);
    }

    // ─── Test 12: Residential without road access generates no trips ──────

    #[test]
    fn residential_without_road_access_generates_no_trips() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let tile_map = TileMap::new(16, 16); // no ROAD_ACCESS set anywhere
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        // Place house and shop, but home tile has no road access.
        make_active(&mut entities, 1, 2, 2);
        make_active(&mut entities, 2, 5, 5);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        assert_eq!(stats.total_trips, 0, "No trips should be generated without road access");
    }

    // ─── Test 13: Residential with road access generates trips ───────────

    #[test]
    fn residential_with_road_access_generates_trips() {
        let mut grid = TrafficGrid::new(16, 16);
        let mut entities = EntityStore::new(64);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        let mut tile_map = TileMap::new(16, 16);
        let mut events = EventBus::new();
        let map_size = MapSize::new(16, 16);

        registry.register(make_residential(1));
        registry.register(make_commercial(2));

        make_active(&mut entities, 1, 2, 2);
        make_active(&mut entities, 2, 5, 5);

        // Set ROAD_ACCESS on the home tile.
        tile_map.set_flags(2, 2, TileFlags::ROAD_ACCESS);

        let stats = tick_transport(
            &mut grid, &entities, &registry, &road_graph, &tile_map, &mut events, 0, map_size,
        );

        assert!(stats.total_trips > 0, "Trips should be generated when home tile has road access");
    }
}
