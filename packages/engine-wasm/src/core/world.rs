//! WorldState — the canonical world state container.
//!
//! Contains all data needed to fully reconstruct a saved game:
//! tile grid, entities, policies, and seeds. Everything else is derived.

use crate::core::entity::EntityStore;
use crate::core_types::*;
use serde::{Deserialize, Serialize};

// ─── Tile ────────────────────────────────────────────────────────────────────

/// Per-tile canonical data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub terrain: TerrainType,
    pub elevation: u8,
    pub zone: ZoneType,
    pub water: bool,
}

impl Tile {
    pub const DEFAULT: Tile = Tile {
        terrain: TerrainType::Grass,
        elevation: 0,
        zone: ZoneType::None,
        water: false,
    };
}

// ─── TileGrid ────────────────────────────────────────────────────────────────

/// 2D grid of tiles indexed by (x, y).
#[derive(Debug)]
pub struct TileGrid {
    tiles: Vec<Tile>,
    width: u16,
    height: u16,
}

impl TileGrid {
    /// Create a new grid filled with default tiles.
    pub fn new(size: MapSize) -> Self {
        let count = size.area() as usize;
        TileGrid {
            tiles: vec![Tile::DEFAULT; count],
            width: size.width,
            height: size.height,
        }
    }

    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u16 {
        self.height
    }

    #[inline]
    pub fn size(&self) -> MapSize {
        MapSize::new(self.width, self.height)
    }

    /// Check if coordinates are within bounds.
    #[inline]
    pub fn in_bounds(&self, x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as u16) < self.width && (y as u16) < self.height
    }

    /// Convert (x, y) to linear index.
    #[inline]
    fn index(&self, x: i16, y: i16) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }

    /// Get a tile at (x, y). Returns None if out of bounds.
    #[inline]
    pub fn get(&self, x: i16, y: i16) -> Option<&Tile> {
        if self.in_bounds(x, y) {
            Some(&self.tiles[self.index(x, y)])
        } else {
            None
        }
    }

    /// Get a mutable reference to a tile.
    #[inline]
    pub fn get_mut(&mut self, x: i16, y: i16) -> Option<&mut Tile> {
        if self.in_bounds(x, y) {
            let idx = self.index(x, y);
            Some(&mut self.tiles[idx])
        } else {
            None
        }
    }

    /// Set a tile. Returns false if out of bounds.
    pub fn set(&mut self, x: i16, y: i16, tile: Tile) -> bool {
        if self.in_bounds(x, y) {
            let idx = self.index(x, y);
            self.tiles[idx] = tile;
            true
        } else {
            false
        }
    }

    /// Set the zone type of a tile.
    pub fn set_zone(&mut self, x: i16, y: i16, zone: ZoneType) -> bool {
        if let Some(t) = self.get_mut(x, y) {
            t.zone = zone;
            true
        } else {
            false
        }
    }

    /// Set the terrain type of a tile.
    pub fn set_terrain(&mut self, x: i16, y: i16, terrain: TerrainType) -> bool {
        if let Some(t) = self.get_mut(x, y) {
            t.terrain = terrain;
            true
        } else {
            false
        }
    }

    /// Iterate over all tiles with their coordinates.
    pub fn iter(&self) -> impl Iterator<Item = (i16, i16, &Tile)> {
        let w = self.width as i16;
        self.tiles.iter().enumerate().map(move |(i, t)| {
            let x = (i as i16) % w;
            let y = (i as i16) / w;
            (x, y, t)
        })
    }
}

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
    pub tiles: TileGrid,
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
            tiles: TileGrid::new(size),
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
        self.tiles.size()
    }

    /// Check if a tile position is buildable (in bounds, not water, no existing entity).
    pub fn is_buildable(&self, x: i16, y: i16) -> bool {
        match self.tiles.get(x, y) {
            Some(tile) => !tile.water && tile.terrain != TerrainType::Water,
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
        if !self.tiles.in_bounds(x, y) {
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

    // ── TileGrid: get/set, in_bounds ─────────────────────────────────────

    #[test]
    fn tile_grid_get_set() {
        let mut grid = TileGrid::new(MapSize::new(8, 8));
        let tile = Tile {
            terrain: TerrainType::Sand,
            elevation: 5,
            zone: ZoneType::Commercial,
            water: false,
        };
        assert!(grid.set(3, 4, tile));
        let got = grid.get(3, 4).unwrap();
        assert_eq!(*got, tile);
    }

    #[test]
    fn tile_grid_in_bounds() {
        let grid = TileGrid::new(MapSize::new(10, 20));
        assert!(grid.in_bounds(0, 0));
        assert!(grid.in_bounds(9, 19));
        assert!(!grid.in_bounds(10, 0));
        assert!(!grid.in_bounds(0, 20));
        assert!(!grid.in_bounds(-1, 0));
        assert!(!grid.in_bounds(0, -1));
    }

    #[test]
    fn tile_grid_zone_setting() {
        let mut grid = TileGrid::new(MapSize::new(8, 8));
        assert!(grid.set_zone(2, 3, ZoneType::Residential));
        assert_eq!(grid.get(2, 3).unwrap().zone, ZoneType::Residential);
    }

    #[test]
    fn tile_grid_terrain_setting() {
        let mut grid = TileGrid::new(MapSize::new(8, 8));
        assert!(grid.set_terrain(1, 1, TerrainType::Forest));
        assert_eq!(grid.get(1, 1).unwrap().terrain, TerrainType::Forest);
    }

    // ── TileGrid: out of bounds returns None ─────────────────────────────

    #[test]
    fn tile_grid_out_of_bounds_returns_none() {
        let grid = TileGrid::new(MapSize::new(4, 4));
        assert!(grid.get(4, 0).is_none());
        assert!(grid.get(0, 4).is_none());
        assert!(grid.get(-1, 0).is_none());
        assert!(grid.get(0, -1).is_none());
    }

    #[test]
    fn tile_grid_set_out_of_bounds_returns_false() {
        let mut grid = TileGrid::new(MapSize::new(4, 4));
        assert!(!grid.set(10, 10, Tile::DEFAULT));
        assert!(!grid.set_zone(-1, 0, ZoneType::Industrial));
        assert!(!grid.set_terrain(0, -1, TerrainType::Rock));
    }

    #[test]
    fn tile_grid_get_mut_out_of_bounds_returns_none() {
        let mut grid = TileGrid::new(MapSize::new(4, 4));
        assert!(grid.get_mut(4, 0).is_none());
        assert!(grid.get_mut(-1, -1).is_none());
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
    fn is_buildable_water_flag() {
        let mut world = WorldState::new(MapSize::new(8, 8), 1);
        let tile = Tile {
            terrain: TerrainType::Grass,
            elevation: 0,
            zone: ZoneType::None,
            water: true,
        };
        world.tiles.set(3, 3, tile);
        assert!(!world.is_buildable(3, 3));
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
        let grid = TileGrid::new(MapSize::new(4, 3));
        let tiles: Vec<_> = grid.iter().collect();
        assert_eq!(tiles.len(), 12);
    }

    #[test]
    fn tile_iteration_correct_coordinates() {
        let grid = TileGrid::new(MapSize::new(3, 2));
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
        let mut grid = TileGrid::new(MapSize::new(4, 4));
        grid.set_zone(2, 1, ZoneType::Industrial);
        let tile = grid.iter().find(|&(x, y, _)| x == 2 && y == 1).unwrap();
        assert_eq!(tile.2.zone, ZoneType::Industrial);
    }

    // ── TileGrid width/height/size ───────────────────────────────────────

    #[test]
    fn tile_grid_dimensions() {
        let grid = TileGrid::new(MapSize::new(32, 64));
        assert_eq!(grid.width(), 32);
        assert_eq!(grid.height(), 64);
        assert_eq!(grid.size(), MapSize::new(32, 64));
    }

    // ── Tile::DEFAULT ────────────────────────────────────────────────────

    #[test]
    fn tile_default_values() {
        let t = Tile::DEFAULT;
        assert_eq!(t.terrain, TerrainType::Grass);
        assert_eq!(t.elevation, 0);
        assert_eq!(t.zone, ZoneType::None);
        assert!(!t.water);
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

    // ── TileGrid new fills with defaults ─────────────────────────────────

    #[test]
    fn tile_grid_new_all_default() {
        let grid = TileGrid::new(MapSize::new(4, 4));
        for (_, _, tile) in grid.iter() {
            assert_eq!(*tile, Tile::DEFAULT);
        }
    }
}
