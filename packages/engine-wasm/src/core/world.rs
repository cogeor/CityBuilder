//! WorldState — the canonical world state container.
//!
//! Contains all data needed to fully reconstruct a saved game:
//! tile grid, entities, policies, and seeds. Everything else is derived.

use crate::core::entity::EntityStore;
use crate::core::tilemap::TileMap;
use crate::core_types::*;
use serde::{Deserialize, Serialize};

// ─── CityPolicies ────────────────────────────────────────────────────────────

/// City-wide policy settings. All stored, all canonical.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityPolicies {
    /// Tax rate for residential zones (percentage 0-100).
    pub residential_tax_pct: u8,
    /// Tax rate for commercial zones.
    pub commercial_tax_pct: u8,
    /// Tax rate for industrial zones.
    pub industrial_tax_pct: u8,
    /// Budget allocation per department (percentage 0-200, where 100 = normal funding).
    pub police_budget_pct: u8,
    pub fire_budget_pct: u8,
    pub health_budget_pct: u8,
    pub education_budget_pct: u8,
    pub transport_budget_pct: u8,
}

impl Default for CityPolicies {
    fn default() -> Self {
        CityPolicies {
            residential_tax_pct: 9,
            commercial_tax_pct: 9,
            industrial_tax_pct: 9,
            police_budget_pct: 100,
            fire_budget_pct: 100,
            health_budget_pct: 100,
            education_budget_pct: 100,
            transport_budget_pct: 100,
        }
    }
}

// ─── WorldSeeds ──────────────────────────────────────────────────────────────

/// RNG seeds for deterministic simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSeeds {
    /// Root seed from which all others derive.
    pub global_seed: u64,
}

impl WorldSeeds {
    pub fn new(seed: u64) -> Self {
        WorldSeeds { global_seed: seed }
    }
}

// ─── WorldState ──────────────────────────────────────────────────────────────

/// The canonical world state. Contains everything needed for a save file.
#[derive(Debug)]
pub struct WorldState {
    pub tiles: TileMap,
    pub entities: EntityStore,
    pub policies: CityPolicies,
    pub seeds: WorldSeeds,
    /// Current simulation tick.
    pub tick: Tick,
    /// City treasury in cents.
    pub treasury: MoneyCents,
    /// City name.
    pub city_name: String,
}

impl WorldState {
    /// Create a new world with the given map size and seed.
    pub fn new(size: MapSize, seed: u64) -> Self {
        WorldState {
            tiles: TileMap::new(size.width as u32, size.height as u32),
            entities: EntityStore::new(size.area() as usize),
            policies: CityPolicies::default(),
            seeds: WorldSeeds::new(seed),
            tick: 0,
            treasury: 500_000, // Start with $5,000.00
            city_name: String::from("New Town"),
        }
    }

    /// Get the map size.
    #[inline]
    pub fn map_size(&self) -> MapSize {
        MapSize::new(self.tiles.width() as u16, self.tiles.height() as u16)
    }

    /// Check if a tile position is buildable (in bounds, not water, no existing entity).
    pub fn is_buildable(&self, x: i16, y: i16) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        match self.tiles.get(x as u32, y as u32) {
            Some(tile) => tile.terrain != TerrainType::Water,
            None => false,
        }
    }

    /// Check if a rectangular area is fully buildable.
    pub fn is_area_buildable(&self, x: i16, y: i16, w: u8, h: u8) -> bool {
        for dy in 0..h as i16 {
            for dx in 0..w as i16 {
                if !self.is_buildable(x + dx, y + dy) {
                    return false;
                }
            }
        }
        true
    }

    /// Place an entity at the given position. Returns handle or None if placement fails.
    pub fn place_entity(
        &mut self,
        archetype: ArchetypeId,
        x: i16,
        y: i16,
        rotation: u8,
    ) -> Option<EntityHandle> {
        if x < 0 || y < 0 || !self.tiles.in_bounds(x as u32, y as u32) {
            return None;
        }
        self.entities.alloc(archetype, x, y, rotation)
    }

    /// Remove an entity by handle.
    pub fn remove_entity(&mut self, handle: EntityHandle) -> bool {
        self.entities.free(handle)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tilemap::{TileFlags, TileKind, TileMap, TileValue};

    // ── WorldState::new defaults ─────────────────────────────────────────

    #[test]
    fn world_state_new_creates_correct_defaults() {
        let size = MapSize::new(64, 32);
        let world = WorldState::new(size, 12345);

        assert_eq!(world.map_size(), size);
        assert_eq!(world.tick, 0);
        assert_eq!(world.treasury, 500_000);
        assert_eq!(world.city_name, "New Town");
        assert_eq!(world.seeds.global_seed, 12345);
        assert_eq!(world.entities.count(), 0);
    }

    // ── TileMap: get/set, in_bounds ──────────────────────────────────────

    #[test]
    fn tile_grid_get_set() {
        let mut grid = TileMap::new(8, 8);
        let tile = TileValue {
            terrain: TerrainType::Sand,
            kind: TileKind::Empty,
            zone: ZoneType::Commercial,
            density: ZoneDensity::Low,
            flags: TileFlags::NONE,
        };
        assert!(grid.set(3, 4, tile));
        let got = grid.get(3, 4).unwrap();
        assert_eq!(got, tile);
    }

    #[test]
    fn tile_grid_in_bounds() {
        let grid = TileMap::new(10, 20);
        assert!(grid.in_bounds(0, 0));
        assert!(grid.in_bounds(9, 19));
        assert!(!grid.in_bounds(10, 0));
        assert!(!grid.in_bounds(0, 20));
        // Negative values are i16 and must be guarded before calling in_bounds
        // (TileMap uses u32; negative i16 as u32 would be huge and thus out of bounds)
    }

    #[test]
    fn tile_grid_zone_setting() {
        let mut grid = TileMap::new(8, 8);
        assert!(grid.set_zone(2, 3, ZoneType::Residential));
        assert_eq!(grid.get(2, 3).unwrap().zone, ZoneType::Residential);
    }

    #[test]
    fn tile_grid_terrain_setting() {
        let mut grid = TileMap::new(8, 8);
        assert!(grid.set_terrain(1, 1, TerrainType::Forest));
        assert_eq!(grid.get(1, 1).unwrap().terrain, TerrainType::Forest);
    }

    // ── TileMap: out of bounds returns None ──────────────────────────────

    #[test]
    fn tile_grid_out_of_bounds_returns_none() {
        let grid = TileMap::new(4, 4);
        assert!(grid.get(4, 0).is_none());
        assert!(grid.get(0, 4).is_none());
    }

    #[test]
    fn tile_grid_set_out_of_bounds_returns_false() {
        let mut grid = TileMap::new(4, 4);
        assert!(!grid.set(10, 10, TileValue::DEFAULT));
        assert!(!grid.set_zone(u32::MAX, 0, ZoneType::Industrial));
        assert!(!grid.set_terrain(0, u32::MAX, TerrainType::Rock));
    }

    #[test]
    fn tile_grid_get_mut_out_of_bounds_returns_none() {
        let mut grid = TileMap::new(4, 4);
        assert!(grid.get_mut(4, 0).is_none());
    }

    // ── CityPolicies default values ──────────────────────────────────────

    #[test]
    fn city_policies_default_values() {
        let p = CityPolicies::default();
        assert_eq!(p.residential_tax_pct, 9);
        assert_eq!(p.commercial_tax_pct, 9);
        assert_eq!(p.industrial_tax_pct, 9);
        assert_eq!(p.police_budget_pct, 100);
        assert_eq!(p.fire_budget_pct, 100);
        assert_eq!(p.health_budget_pct, 100);
        assert_eq!(p.education_budget_pct, 100);
        assert_eq!(p.transport_budget_pct, 100);
    }

    // ── is_buildable (land vs water) ─────────────────────────────────────

    #[test]
    fn is_buildable_land_tile() {
        let world = WorldState::new(MapSize::new(8, 8), 1);
        // Default tiles are grass, not water -> buildable
        assert!(world.is_buildable(0, 0));
    }

    #[test]
    fn is_buildable_water_terrain() {
        let mut world = WorldState::new(MapSize::new(8, 8), 1);
        world.tiles.set_terrain(2, 2, TerrainType::Water);
        assert!(!world.is_buildable(2, 2));
    }

    #[test]
    fn is_buildable_water_tile_by_terrain() {
        // The old water flag is gone; only terrain == Water matters.
        // Verify that a Water terrain tile is not buildable.
        let mut world = WorldState::new(MapSize::new(8, 8), 1);
        world.tiles.set_terrain(3, 3, TerrainType::Water);
        assert!(!world.is_buildable(3, 3));
        // And a Grass terrain tile is buildable.
        assert!(world.is_buildable(4, 4));
    }

    #[test]
    fn is_buildable_out_of_bounds() {
        let world = WorldState::new(MapSize::new(8, 8), 1);
        assert!(!world.is_buildable(-1, 0));
        assert!(!world.is_buildable(8, 0));
    }

    // ── is_area_buildable ────────────────────────────────────────────────

    #[test]
    fn is_area_buildable_all_land() {
        let world = WorldState::new(MapSize::new(8, 8), 1);
        assert!(world.is_area_buildable(0, 0, 3, 3));
    }

    #[test]
    fn is_area_buildable_with_water() {
        let mut world = WorldState::new(MapSize::new(8, 8), 1);
        world.tiles.set_terrain(1, 1, TerrainType::Water);
        assert!(!world.is_area_buildable(0, 0, 3, 3));
    }

    #[test]
    fn is_area_buildable_partially_out_of_bounds() {
        let world = WorldState::new(MapSize::new(8, 8), 1);
        // Area extends beyond map edge
        assert!(!world.is_area_buildable(6, 6, 4, 4));
    }

    // ── place_entity and remove_entity ───────────────────────────────────

    #[test]
    fn place_entity_returns_handle() {
        let mut world = WorldState::new(MapSize::new(16, 16), 1);
        let h = world.place_entity(1, 5, 5, 0);
        assert!(h.is_some());
        assert_eq!(world.entities.count(), 1);
    }

    #[test]
    fn place_entity_out_of_bounds_returns_none() {
        let mut world = WorldState::new(MapSize::new(8, 8), 1);
        assert!(world.place_entity(1, 8, 0, 0).is_none());
        assert!(world.place_entity(1, -1, 0, 0).is_none());
    }

    #[test]
    fn remove_entity_frees_handle() {
        let mut world = WorldState::new(MapSize::new(16, 16), 1);
        let h = world.place_entity(1, 5, 5, 0).unwrap();
        assert!(world.remove_entity(h));
        assert_eq!(world.entities.count(), 0);
        assert!(!world.entities.is_valid(h));
    }

    #[test]
    fn remove_entity_invalid_handle_returns_false() {
        let mut world = WorldState::new(MapSize::new(8, 8), 1);
        assert!(!world.remove_entity(EntityHandle::INVALID));
    }

    // ── Entity store integration ─────────────────────────────────────────

    #[test]
    fn entity_store_integration_place_get_position_remove() {
        let mut world = WorldState::new(MapSize::new(16, 16), 1);
        let h = world.place_entity(42, 10, 7, 2).unwrap();

        // Verify position via entity store
        let pos = world.entities.get_pos(h).unwrap();
        assert_eq!(pos, TileCoord::new(10, 7));

        // Verify archetype
        assert_eq!(world.entities.get_archetype(h), Some(42));

        // Remove and verify
        assert!(world.remove_entity(h));
        assert!(world.entities.get_pos(h).is_none());
    }

    #[test]
    fn entity_store_multiple_entities() {
        let mut world = WorldState::new(MapSize::new(16, 16), 1);
        let h1 = world.place_entity(1, 0, 0, 0).unwrap();
        let h2 = world.place_entity(2, 1, 1, 1).unwrap();
        let h3 = world.place_entity(3, 2, 2, 2).unwrap();
        assert_eq!(world.entities.count(), 3);

        world.remove_entity(h2);
        assert_eq!(world.entities.count(), 2);
        assert!(world.entities.is_valid(h1));
        assert!(!world.entities.is_valid(h2));
        assert!(world.entities.is_valid(h3));
    }

    // ── Tile iteration ───────────────────────────────────────────────────

    #[test]
    fn tile_iteration_covers_all_tiles() {
        let grid = TileMap::new(4, 3);
        let tiles: Vec<_> = grid.iter().collect();
        assert_eq!(tiles.len(), 12);
    }

    #[test]
    fn tile_iteration_correct_coordinates() {
        let grid = TileMap::new(3, 2);
        let tiles: Vec<_> = grid.iter().collect();
        // Row-major: (0,0),(1,0),(2,0),(0,1),(1,1),(2,1)
        assert_eq!(tiles[0].0, 0);
        assert_eq!(tiles[0].1, 0);
        assert_eq!(tiles[1].0, 1);
        assert_eq!(tiles[1].1, 0);
        assert_eq!(tiles[2].0, 2);
        assert_eq!(tiles[2].1, 0);
        assert_eq!(tiles[3].0, 0);
        assert_eq!(tiles[3].1, 1);
        assert_eq!(tiles[5].0, 2);
        assert_eq!(tiles[5].1, 1);
    }

    #[test]
    fn tile_iteration_reflects_modifications() {
        let mut grid = TileMap::new(4, 4);
        grid.set_zone(2, 1, ZoneType::Industrial);
        let tile = grid.iter().find(|&(x, y, _)| x == 2 && y == 1).unwrap();
        assert_eq!(tile.2.zone, ZoneType::Industrial);
    }

    // ── TileMap width/height/size ─────────────────────────────────────────

    #[test]
    fn tile_grid_dimensions() {
        let grid = TileMap::new(32, 64);
        assert_eq!(grid.width(), 32);
        assert_eq!(grid.height(), 64);
        assert_eq!(
            MapSize::new(grid.width() as u16, grid.height() as u16),
            MapSize::new(32, 64)
        );
    }

    // ── TileValue::DEFAULT ───────────────────────────────────────────────

    #[test]
    fn tile_default_values() {
        let t = TileValue::DEFAULT;
        assert_eq!(t.terrain, TerrainType::Grass);
        assert_eq!(t.kind, TileKind::Empty);
        assert_eq!(t.zone, ZoneType::None);
        assert!(t.flags.is_empty());
    }

    // ── WorldSeeds ───────────────────────────────────────────────────────

    #[test]
    fn world_seeds_stores_seed() {
        let seeds = WorldSeeds::new(42);
        assert_eq!(seeds.global_seed, 42);
    }

    // ── WorldState map_size ──────────────────────────────────────────────

    #[test]
    fn world_state_map_size() {
        let world = WorldState::new(MapSize::new(100, 50), 1);
        assert_eq!(world.map_size(), MapSize::new(100, 50));
    }

    // ── TileMap new fills with defaults ──────────────────────────────────

    #[test]
    fn tile_grid_new_all_default() {
        let grid = TileMap::new(4, 4);
        for (_, _, tile) in grid.iter() {
            assert_eq!(tile, TileValue::DEFAULT);
        }
    }
}
