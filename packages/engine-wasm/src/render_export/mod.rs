//! Render export — builds RenderInstance buffers from simulation state.
//!
//! This module sits at the render boundary: it reads canonical sim data
//! (tile grid, entity store) and produces tightly-packed GPU-ready buffers.
//! Floating-point types are used here because this is render output, not sim.

use crate::core::entity::EntityStore;
use crate::core::world::{TileGrid, WorldState};
use crate::core_types::*;

// ─── RenderFlags ────────────────────────────────────────────────────────────

/// Bitflag constants for per-instance render state.
pub struct RenderFlags;

impl RenderFlags {
    pub const VISIBLE: u8 = 1;
    pub const HIGHLIGHTED: u8 = 2;
    pub const SHADOW: u8 = 4;
    pub const ANIMATED: u8 = 8;
}

// ─── RenderInstance ─────────────────────────────────────────────────────────

/// GPU-ready per-instance data. Tightly packed for upload.
///
/// Total size: 48 bytes. Uses `#[repr(C)]` to guarantee field ordering
/// matches the GPU vertex/instance layout.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RenderInstance {
    /// Sprite index within the atlas.
    pub sprite_id: u16,
    /// Which texture atlas this sprite belongs to.
    pub atlas_id: u8,
    /// Padding byte to align screen_x to 4-byte boundary.
    pub _pad0: u8,
    /// Screen X coordinate (isometric projection).
    pub screen_x: f32,
    /// Screen Y coordinate (isometric projection).
    pub screen_y: f32,
    /// Depth-sort key (painter's algorithm ordering).
    pub z_order: u32,
    /// Palette swap index.
    pub palette_id: u8,
    /// Mask flags for shader effects.
    pub mask_flags: u8,
    /// Current animation frame.
    pub anim_frame: u8,
    /// Render state flags (VISIBLE, HIGHLIGHTED, SHADOW, ANIMATED).
    pub render_flags: u8,
    /// Horizontal scale factor.
    pub scale_x: f32,
    /// Vertical scale factor.
    pub scale_y: f32,
    /// Rotation in radians.
    pub rotation: f32,
    /// Tint color red channel.
    pub tint_r: u8,
    /// Tint color green channel.
    pub tint_g: u8,
    /// Tint color blue channel.
    pub tint_b: u8,
    /// Tint color alpha channel.
    pub tint_a: u8,
    /// Reserved bytes for future use / GPU alignment padding to 48 bytes.
    pub _reserved: [u8; 12],
}

impl RenderInstance {
    /// Create a default (zeroed) render instance.
    #[inline]
    pub fn default_visible() -> Self {
        RenderInstance {
            sprite_id: 0,
            atlas_id: 0,
            _pad0: 0,
            screen_x: 0.0,
            screen_y: 0.0,
            z_order: 0,
            palette_id: 0,
            mask_flags: 0,
            anim_frame: 0,
            render_flags: RenderFlags::VISIBLE,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            tint_r: 255,
            tint_g: 255,
            tint_b: 255,
            tint_a: 255,
            _reserved: [0; 12],
        }
    }
}

// ─── Isometric Projection ───────────────────────────────────────────────────

/// Convert tile coordinates to screen-space using simplified isometric projection.
///
/// screen_x = x * 64 - y * 64
/// screen_y = x * 32 + y * 32
#[inline]
fn tile_to_screen(x: i16, y: i16) -> (f32, f32) {
    let fx = x as f32;
    let fy = y as f32;
    (fx * 64.0 - fy * 64.0, fx * 32.0 + fy * 32.0)
}

// ─── Terrain Instances ──────────────────────────────────────────────────────

/// Build render instances for all terrain tiles.
///
/// Each tile produces one RenderInstance with:
/// - sprite_id mapped from terrain type
/// - isometric screen position
/// - z_order = y * width + x (painter's order, back-to-front)
pub fn build_terrain_instances(tiles: &TileGrid) -> Vec<RenderInstance> {
    let w = tiles.width() as u32;
    let mut instances = Vec::with_capacity((tiles.width() as usize) * (tiles.height() as usize));

    for (x, y, tile) in tiles.iter() {
        let (sx, sy) = tile_to_screen(x, y);
        let mut inst = RenderInstance::default_visible();
        inst.sprite_id = tile.terrain as u16;
        inst.screen_x = sx;
        inst.screen_y = sy;
        inst.z_order = (y as u32) * w + (x as u32);
        instances.push(inst);
    }

    instances
}

// ─── Entity Instances ───────────────────────────────────────────────────────

/// Build render instances for all alive entities.
///
/// Each alive entity produces one RenderInstance with:
/// - sprite_id = archetype_id
/// - isometric screen position from tile position
/// - z_order = y * 65536 + x (entities sort above terrain at same tile)
/// - StatusFlags mapped to visual state:
///   - ON_FIRE -> red tint (255, 80, 40)
///   - UNDER_CONSTRUCTION -> alpha 128
///   - DAMAGED -> dark tint (128, 128, 128)
/// - anim_frame from construction_progress (0-3 stages)
pub fn build_entity_instances(entities: &EntityStore) -> Vec<RenderInstance> {
    let mut instances = Vec::new();

    for handle in entities.iter_alive() {
        let pos = match entities.get_pos(handle) {
            Some(p) => p,
            None => continue,
        };
        let archetype = match entities.get_archetype(handle) {
            Some(a) => a,
            None => continue,
        };
        let flags = entities.get_flags(handle).unwrap_or(StatusFlags::NONE);
        let progress = entities.get_construction_progress(handle).unwrap_or(0);

        let (sx, sy) = tile_to_screen(pos.x, pos.y);

        let mut inst = RenderInstance::default_visible();
        inst.sprite_id = archetype;
        inst.screen_x = sx;
        inst.screen_y = sy;
        inst.z_order = (pos.y as u32) * 65536 + (pos.x as u32);

        // Map construction_progress (Q0.16: 0..0xFFFF) to 4 animation frames (0-3)
        inst.anim_frame = (progress >> 14) as u8; // 0xFFFF >> 14 = 3

        // Apply status flag visual effects
        if flags.contains(StatusFlags::ON_FIRE) {
            inst.tint_r = 255;
            inst.tint_g = 80;
            inst.tint_b = 40;
            inst.render_flags |= RenderFlags::ANIMATED;
        }

        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
            inst.tint_a = 128;
        }

        if flags.contains(StatusFlags::DAMAGED) {
            inst.tint_r = 128;
            inst.tint_g = 128;
            inst.tint_b = 128;
        }

        instances.push(inst);
    }

    instances
}

// ─── Combined Render Buffer ─────────────────────────────────────────────────

/// Build the complete render buffer from world state.
///
/// Combines terrain and entity instances, then sorts by z_order
/// for correct painter's algorithm rendering.
pub fn build_render_buffer(world: &WorldState) -> Vec<RenderInstance> {
    let mut buffer = build_terrain_instances(&world.tiles);
    let entity_instances = build_entity_instances(&world.entities);
    buffer.extend(entity_instances);
    buffer.sort_by_key(|inst| inst.z_order);
    buffer
}

// ─── ChunkDirtyTracker ──────────────────────────────────────────────────────

/// Chunk size in tiles for dirty-rectangle tracking.
pub const CHUNK_SIZE: u16 = 32;

/// Tracks which map chunks have been modified and need re-rendering.
///
/// The map is divided into CHUNK_SIZE x CHUNK_SIZE tile chunks.
/// When a tile changes, the containing chunk is marked dirty.
/// The renderer can query dirty chunks to do incremental updates.
pub struct ChunkDirtyTracker {
    dirty: Vec<bool>,
    pub chunks_w: u16,
    pub chunks_h: u16,
}

impl ChunkDirtyTracker {
    /// Create a new tracker for a map of the given tile dimensions.
    /// All chunks start clean.
    pub fn new(map_width: u16, map_height: u16) -> Self {
        let cw = (map_width + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let ch = (map_height + CHUNK_SIZE - 1) / CHUNK_SIZE;
        ChunkDirtyTracker {
            dirty: vec![false; (cw as usize) * (ch as usize)],
            chunks_w: cw,
            chunks_h: ch,
        }
    }

    /// Mark the chunk containing the given tile as dirty.
    pub fn mark_dirty(&mut self, tile_x: u16, tile_y: u16) {
        let cx = tile_x / CHUNK_SIZE;
        let cy = tile_y / CHUNK_SIZE;
        if cx < self.chunks_w && cy < self.chunks_h {
            let idx = (cy as usize) * (self.chunks_w as usize) + (cx as usize);
            self.dirty[idx] = true;
        }
    }

    /// Check if a specific chunk is dirty.
    pub fn is_dirty(&self, chunk_x: u16, chunk_y: u16) -> bool {
        if chunk_x < self.chunks_w && chunk_y < self.chunks_h {
            let idx = (chunk_y as usize) * (self.chunks_w as usize) + (chunk_x as usize);
            self.dirty[idx]
        } else {
            false
        }
    }

    /// Clear all dirty flags.
    pub fn clear_all(&mut self) {
        for d in self.dirty.iter_mut() {
            *d = false;
        }
    }

    /// Iterate over all dirty chunk coordinates.
    pub fn iter_dirty(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        let w = self.chunks_w;
        self.dirty.iter().enumerate().filter_map(move |(i, &d)| {
            if d {
                let cx = (i as u16) % w;
                let cy = (i as u16) / w;
                Some((cx, cy))
            } else {
                None
            }
        })
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::world::TileGrid;
    use std::mem;

    // ── RenderInstance size ──────────────────────────────────────────────

    #[test]
    fn render_instance_is_48_bytes() {
        assert_eq!(mem::size_of::<RenderInstance>(), 48);
    }

    // ── Terrain instances ───────────────────────────────────────────────

    #[test]
    fn terrain_instances_generated_for_each_tile() {
        let grid = TileGrid::new(MapSize::new(4, 3));
        let instances = build_terrain_instances(&grid);
        assert_eq!(instances.len(), 12); // 4 * 3
    }

    #[test]
    fn terrain_instance_sprite_id_matches_terrain_type() {
        let mut grid = TileGrid::new(MapSize::new(2, 1));
        grid.set_terrain(1, 0, TerrainType::Forest);

        let instances = build_terrain_instances(&grid);
        assert_eq!(instances[0].sprite_id, TerrainType::Grass as u16);
        assert_eq!(instances[1].sprite_id, TerrainType::Forest as u16);
    }

    #[test]
    fn terrain_z_order_correct() {
        let grid = TileGrid::new(MapSize::new(4, 3));
        let instances = build_terrain_instances(&grid);
        // Tile (0,0) -> z_order = 0*4 + 0 = 0
        assert_eq!(instances[0].z_order, 0);
        // Tile (1,0) -> z_order = 0*4 + 1 = 1
        assert_eq!(instances[1].z_order, 1);
        // Tile (0,1) -> z_order = 1*4 + 0 = 4
        assert_eq!(instances[4].z_order, 4);
        // Tile (3,2) -> z_order = 2*4 + 3 = 11
        assert_eq!(instances[11].z_order, 11);
    }

    #[test]
    fn terrain_instance_screen_position() {
        let grid = TileGrid::new(MapSize::new(4, 4));
        let instances = build_terrain_instances(&grid);

        // Tile (0,0): screen_x = 0, screen_y = 0
        assert_eq!(instances[0].screen_x, 0.0);
        assert_eq!(instances[0].screen_y, 0.0);

        // Tile (1,0): screen_x = 64, screen_y = 32
        assert_eq!(instances[1].screen_x, 64.0);
        assert_eq!(instances[1].screen_y, 32.0);

        // Tile (0,1): screen_x = -64, screen_y = 32
        assert_eq!(instances[4].screen_x, -64.0);
        assert_eq!(instances[4].screen_y, 32.0);
    }

    #[test]
    fn terrain_instances_all_visible() {
        let grid = TileGrid::new(MapSize::new(3, 3));
        let instances = build_terrain_instances(&grid);
        for inst in &instances {
            assert!(inst.render_flags & RenderFlags::VISIBLE != 0);
        }
    }

    // ── Entity instances ────────────────────────────────────────────────

    #[test]
    fn entity_instances_generated_for_alive_entities() {
        let mut store = EntityStore::new(16);
        let _h1 = store.alloc(10, 2, 3, 0).unwrap();
        let _h2 = store.alloc(20, 5, 7, 0).unwrap();

        let instances = build_entity_instances(&store);
        assert_eq!(instances.len(), 2);
    }

    #[test]
    fn entity_z_order_above_terrain() {
        // Terrain at (2,3): z_order = 3 * width + 2 (for any reasonable width)
        // Entity at (2,3): z_order = 3 * 65536 + 2
        let mut store = EntityStore::new(16);
        let _h = store.alloc(10, 2, 3, 0).unwrap();
        let instances = build_entity_instances(&store);
        assert_eq!(instances[0].z_order, 3 * 65536 + 2);

        // For a 256-wide map, terrain z_order at (2,3) = 3*256 + 2 = 770
        // Entity z_order = 3*65536 + 2 = 196610 which is >> 770
        assert!(instances[0].z_order > 3 * 256 + 2);
    }

    #[test]
    fn on_fire_tint_applied() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(10, 0, 0, 0).unwrap();
        store.set_flags(h, StatusFlags::ON_FIRE);

        let instances = build_entity_instances(&store);
        assert_eq!(instances[0].tint_r, 255);
        assert_eq!(instances[0].tint_g, 80);
        assert_eq!(instances[0].tint_b, 40);
        assert!(instances[0].render_flags & RenderFlags::ANIMATED != 0);
    }

    #[test]
    fn under_construction_alpha() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(10, 0, 0, 0).unwrap();
        // Entity starts with UNDER_CONSTRUCTION by default
        assert!(store.get_flags(h).unwrap().contains(StatusFlags::UNDER_CONSTRUCTION));

        let instances = build_entity_instances(&store);
        assert_eq!(instances[0].tint_a, 128);
    }

    #[test]
    fn damaged_tint_applied() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(10, 0, 0, 0).unwrap();
        store.set_flags(h, StatusFlags::DAMAGED);

        let instances = build_entity_instances(&store);
        assert_eq!(instances[0].tint_r, 128);
        assert_eq!(instances[0].tint_g, 128);
        assert_eq!(instances[0].tint_b, 128);
    }

    #[test]
    fn entity_anim_frame_from_construction_progress() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(10, 0, 0, 0).unwrap();

        // progress 0 -> frame 0
        store.set_construction_progress(h, 0);
        let instances = build_entity_instances(&store);
        assert_eq!(instances[0].anim_frame, 0);

        // progress 0x4000 (25%) -> frame 1
        store.set_construction_progress(h, 0x4000);
        let instances = build_entity_instances(&store);
        assert_eq!(instances[0].anim_frame, 1);

        // progress 0xFFFF (100%) -> frame 3
        store.set_construction_progress(h, 0xFFFF);
        let instances = build_entity_instances(&store);
        assert_eq!(instances[0].anim_frame, 3);
    }

    // ── ChunkDirtyTracker ───────────────────────────────────────────────

    #[test]
    fn chunk_tracker_new_all_clean() {
        let tracker = ChunkDirtyTracker::new(128, 128);
        assert_eq!(tracker.chunks_w, 4); // 128 / 32
        assert_eq!(tracker.chunks_h, 4);
        for cy in 0..tracker.chunks_h {
            for cx in 0..tracker.chunks_w {
                assert!(!tracker.is_dirty(cx, cy));
            }
        }
    }

    #[test]
    fn chunk_tracker_mark_dirty_and_is_dirty() {
        let mut tracker = ChunkDirtyTracker::new(128, 128);
        // Tile (35, 40) is in chunk (1, 1)
        tracker.mark_dirty(35, 40);
        assert!(tracker.is_dirty(1, 1));
        assert!(!tracker.is_dirty(0, 0));
        assert!(!tracker.is_dirty(2, 2));
    }

    #[test]
    fn chunk_tracker_clear_all_resets() {
        let mut tracker = ChunkDirtyTracker::new(64, 64);
        tracker.mark_dirty(0, 0);
        tracker.mark_dirty(33, 33);
        assert!(tracker.is_dirty(0, 0));
        assert!(tracker.is_dirty(1, 1));

        tracker.clear_all();
        assert!(!tracker.is_dirty(0, 0));
        assert!(!tracker.is_dirty(1, 1));
    }

    #[test]
    fn chunk_tracker_iter_dirty() {
        let mut tracker = ChunkDirtyTracker::new(96, 96);
        // 96/32 = 3 chunks per axis
        tracker.mark_dirty(0, 0);   // chunk (0,0)
        tracker.mark_dirty(64, 64); // chunk (2,2)

        let dirty: Vec<(u16, u16)> = tracker.iter_dirty().collect();
        assert_eq!(dirty.len(), 2);
        assert!(dirty.contains(&(0, 0)));
        assert!(dirty.contains(&(2, 2)));
    }

    // ── Combined buffer ─────────────────────────────────────────────────

    #[test]
    fn build_buffer_combines_and_sorts() {
        let mut world = WorldState::new(MapSize::new(4, 4), 42);
        // Place an entity at (2, 1)
        world.place_entity(100, 2, 1, 0);

        let buffer = build_render_buffer(&world);
        // 16 terrain + 1 entity = 17
        assert_eq!(buffer.len(), 17);

        // Verify sorted by z_order
        for pair in buffer.windows(2) {
            assert!(pair[0].z_order <= pair[1].z_order);
        }

        // Entity at (2,1) has z_order = 1*65536 + 2 = 65538
        // It should appear after all terrain tiles (max terrain z_order = 3*4+3 = 15)
        let entity_inst = buffer.iter().find(|i| i.sprite_id == 100).unwrap();
        assert_eq!(entity_inst.z_order, 65538);
    }

    #[test]
    fn render_flags_constants() {
        assert_eq!(RenderFlags::VISIBLE, 1);
        assert_eq!(RenderFlags::HIGHLIGHTED, 2);
        assert_eq!(RenderFlags::SHADOW, 4);
        assert_eq!(RenderFlags::ANIMATED, 8);
    }

    #[test]
    fn chunk_tracker_non_aligned_map_size() {
        // Map size not evenly divisible by CHUNK_SIZE
        let tracker = ChunkDirtyTracker::new(50, 70);
        // 50/32 = 1.5625 -> ceil = 2
        // 70/32 = 2.1875 -> ceil = 3
        assert_eq!(tracker.chunks_w, 2);
        assert_eq!(tracker.chunks_h, 3);
    }
}
