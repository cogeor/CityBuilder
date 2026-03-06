//! SoA entity storage with generational handles.
//!
//! Entities are stored as parallel arrays for cache-friendly iteration.
//! Each entity slot has a generation counter; stale handles are detected
//! by comparing handle.generation against the slot's current generation.

use crate::core_types::*;

/// Maximum number of entities. Chosen to fit comfortably in WASM memory.
pub const MAX_ENTITIES: usize = 65536;

/// Data stored per-entity in SoA layout.
/// Each field is a separate array for cache-friendly access patterns.
#[derive(Debug)]
pub struct EntityStore {
    // ─── Per-slot metadata ────────
    /// Generation counter per slot. Incremented on free.
    generation: Vec<u32>,
    /// Bitset tracking which slots are occupied. Each u64 word covers 64 slots.
    alive: Vec<u64>,
    /// Actual requested capacity (capped at MAX_ENTITIES).
    capacity: usize,

    // ─── Entity fields (SoA) ─────
    /// Archetype identifier.
    pub archetype_id: Vec<ArchetypeId>,
    /// Tile X position.
    pub pos_x: Vec<i16>,
    /// Tile Y position.
    pub pos_y: Vec<i16>,
    /// Rotation (0-3, 90-degree increments).
    pub rotation: Vec<u8>,
    /// Current building level / upgrade tier.
    pub level: Vec<u8>,
    /// Status bitflags (powered, staffed, etc.).
    pub flags: Vec<StatusFlags>,
    /// Construction progress (Q0.16: 0 = not started, 0xFFFF = complete).
    pub construction_progress: Vec<u16>,
    /// Whether entity is enabled by player.
    pub enabled: Vec<bool>,

    // ─── Bookkeeping ─────────────
    /// Free list (stack of available slot indices).
    free_list: Vec<u32>,
    /// Current number of alive entities.
    count: u32,
}

impl EntityStore {
    /// Create a new empty entity store with pre-allocated capacity.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.min(MAX_ENTITIES);
        let words = (cap + 63) / 64;
        let free_list: Vec<u32> = (0..cap as u32).rev().collect();

        EntityStore {
            generation: vec![0; cap],
            alive: vec![0u64; words],
            capacity: cap,
            archetype_id: vec![0; cap],
            pos_x: vec![0; cap],
            pos_y: vec![0; cap],
            rotation: vec![0; cap],
            level: vec![0; cap],
            flags: vec![StatusFlags::NONE; cap],
            construction_progress: vec![0; cap],
            enabled: vec![true; cap],
            free_list,
            count: 0,
        }
    }

    /// Allocate a new entity and return its handle.
    /// Returns None if the store is full.
    pub fn alloc(
        &mut self,
        archetype: ArchetypeId,
        x: i16,
        y: i16,
        rotation: u8,
    ) -> Option<EntityHandle> {
        let index = self.free_list.pop()?;
        let idx = index as usize;

        let gen = self.generation[idx];
        self.alive[idx / 64] |= 1u64 << (idx % 64);
        self.archetype_id[idx] = archetype;
        self.pos_x[idx] = x;
        self.pos_y[idx] = y;
        self.rotation[idx] = rotation;
        self.level[idx] = 1;
        self.flags[idx] = StatusFlags::UNDER_CONSTRUCTION;
        self.construction_progress[idx] = 0;
        self.enabled[idx] = true;
        self.count += 1;

        Some(EntityHandle::new(index, gen))
    }

    /// Free an entity by handle. Returns true if the entity was successfully freed.
    /// Returns false if the handle is stale or invalid.
    pub fn free(&mut self, handle: EntityHandle) -> bool {
        if !self.is_valid(handle) {
            return false;
        }
        let idx = handle.index as usize;
        self.alive[idx / 64] &= !(1u64 << (idx % 64));
        self.generation[idx] = self.generation[idx].wrapping_add(1);
        self.free_list.push(handle.index);
        self.count -= 1;
        true
    }

    /// Check if a handle refers to a currently alive entity.
    #[inline]
    pub fn is_valid(&self, handle: EntityHandle) -> bool {
        let idx = handle.index as usize;
        idx < self.capacity
            && (self.alive[idx / 64] >> (idx % 64)) & 1 != 0
            && self.generation[idx] == handle.generation
    }

    /// Get the number of alive entities.
    #[inline]
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Get the storage capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get position of an entity. Returns None for invalid handles.
    #[inline]
    pub fn get_pos(&self, handle: EntityHandle) -> Option<TileCoord> {
        if !self.is_valid(handle) {
            return None;
        }
        let idx = handle.index as usize;
        Some(TileCoord::new(self.pos_x[idx], self.pos_y[idx]))
    }

    /// Get archetype of an entity.
    #[inline]
    pub fn get_archetype(&self, handle: EntityHandle) -> Option<ArchetypeId> {
        if !self.is_valid(handle) {
            return None;
        }
        Some(self.archetype_id[handle.index as usize])
    }

    /// Get status flags of an entity.
    #[inline]
    pub fn get_flags(&self, handle: EntityHandle) -> Option<StatusFlags> {
        if !self.is_valid(handle) {
            return None;
        }
        Some(self.flags[handle.index as usize])
    }

    /// Set status flags on an entity.
    #[inline]
    pub fn set_flags(&mut self, handle: EntityHandle, flags: StatusFlags) -> bool {
        if !self.is_valid(handle) {
            return false;
        }
        self.flags[handle.index as usize] = flags;
        true
    }

    /// Get level of an entity.
    #[inline]
    pub fn get_level(&self, handle: EntityHandle) -> Option<u8> {
        if !self.is_valid(handle) {
            return None;
        }
        Some(self.level[handle.index as usize])
    }

    /// Set level of an entity.
    #[inline]
    pub fn set_level(&mut self, handle: EntityHandle, level: u8) -> bool {
        if !self.is_valid(handle) {
            return false;
        }
        self.level[handle.index as usize] = level;
        true
    }

    /// Get construction progress (Q0.16).
    #[inline]
    pub fn get_construction_progress(&self, handle: EntityHandle) -> Option<u16> {
        if !self.is_valid(handle) {
            return None;
        }
        Some(self.construction_progress[handle.index as usize])
    }

    /// Set construction progress.
    #[inline]
    pub fn set_construction_progress(&mut self, handle: EntityHandle, progress: u16) -> bool {
        if !self.is_valid(handle) {
            return false;
        }
        self.construction_progress[handle.index as usize] = progress;
        true
    }

    /// Get enabled state.
    #[inline]
    pub fn get_enabled(&self, handle: EntityHandle) -> Option<bool> {
        if !self.is_valid(handle) {
            return None;
        }
        Some(self.enabled[handle.index as usize])
    }

    /// Set enabled state (player toggle).
    #[inline]
    pub fn set_enabled(&mut self, handle: EntityHandle, en: bool) -> bool {
        if !self.is_valid(handle) {
            return false;
        }
        self.enabled[handle.index as usize] = en;
        true
    }

    /// Iterate over all alive entity handles.
    pub fn iter_alive(&self) -> impl Iterator<Item = EntityHandle> + '_ {
        let cap = self.capacity;
        (0..cap).filter_map(move |i| {
            if (self.alive[i / 64] >> (i % 64)) & 1 != 0 {
                Some(EntityHandle::new(i as u32, self.generation[i]))
            } else {
                None
            }
        })
    }

    /// Iterate alive entities that have specific flags set.
    pub fn iter_with_flags(&self, required: StatusFlags) -> impl Iterator<Item = EntityHandle> + '_ {
        let cap = self.capacity;
        (0..cap).filter_map(move |i| {
            if (self.alive[i / 64] >> (i % 64)) & 1 != 0 && self.flags[i].contains(required) {
                Some(EntityHandle::new(i as u32, self.generation[i]))
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

    #[test]
    fn alloc_returns_valid_handle() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 10, 20, 0).unwrap();
        assert!(store.is_valid(h));
    }

    #[test]
    fn alloc_sets_initial_fields() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(42, 10, 20, 2).unwrap();
        assert_eq!(store.get_archetype(h), Some(42));
        assert_eq!(store.get_pos(h), Some(TileCoord::new(10, 20)));
        assert_eq!(store.get_level(h), Some(1));
        assert_eq!(store.get_flags(h), Some(StatusFlags::UNDER_CONSTRUCTION));
        assert_eq!(store.get_construction_progress(h), Some(0));
        assert_eq!(store.get_enabled(h), Some(true));
    }

    #[test]
    fn free_invalidates_handle() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 0, 0, 0).unwrap();
        assert!(store.free(h));
        assert!(!store.is_valid(h));
    }

    #[test]
    fn generation_increments_on_free() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        assert_eq!(h1.generation, 0);
        store.free(h1);
        // Re-alloc same slot
        let h2 = store.alloc(2, 0, 0, 0).unwrap();
        assert_eq!(h2.index, h1.index);
        assert_eq!(h2.generation, 1);
    }

    #[test]
    fn stale_handle_detected() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        store.free(h1);
        let _h2 = store.alloc(2, 5, 5, 0).unwrap();
        // h1 is stale: same index but old generation
        assert!(!store.is_valid(h1));
        assert_eq!(store.get_pos(h1), None);
        assert_eq!(store.get_archetype(h1), None);
    }

    #[test]
    fn stale_handle_free_returns_false() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        store.free(h1);
        let _h2 = store.alloc(2, 0, 0, 0).unwrap();
        // Freeing with stale handle should fail
        assert!(!store.free(h1));
    }

    #[test]
    fn alloc_returns_none_when_full() {
        let mut store = EntityStore::new(4);
        for i in 0..4 {
            assert!(store.alloc(i as u16, 0, 0, 0).is_some());
        }
        assert!(store.alloc(99, 0, 0, 0).is_none());
    }

    #[test]
    fn count_tracks_alive_entities() {
        let mut store = EntityStore::new(16);
        assert_eq!(store.count(), 0);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        assert_eq!(store.count(), 1);
        let h2 = store.alloc(2, 0, 0, 0).unwrap();
        assert_eq!(store.count(), 2);
        store.free(h1);
        assert_eq!(store.count(), 1);
        store.free(h2);
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn capacity_respects_max_entities() {
        let store = EntityStore::new(MAX_ENTITIES + 100);
        assert_eq!(store.capacity(), MAX_ENTITIES);
    }

    #[test]
    fn capacity_respects_requested() {
        let store = EntityStore::new(32);
        assert_eq!(store.capacity(), 32);
    }

    #[test]
    fn iter_alive_returns_all_alive() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        let _h2 = store.alloc(2, 1, 1, 0).unwrap();
        let h3 = store.alloc(3, 2, 2, 0).unwrap();
        store.free(h1);

        let alive: Vec<EntityHandle> = store.iter_alive().collect();
        assert_eq!(alive.len(), 2);
        // h2 and h3 should be alive; h1 is freed
        assert!(!alive.contains(&h1));
        assert!(alive.contains(&_h2));
        assert!(alive.contains(&h3));
    }

    #[test]
    fn iter_alive_empty_store() {
        let store = EntityStore::new(16);
        let alive: Vec<EntityHandle> = store.iter_alive().collect();
        assert!(alive.is_empty());
    }

    #[test]
    fn iter_with_flags_filters_correctly() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        let h2 = store.alloc(2, 1, 1, 0).unwrap();
        let h3 = store.alloc(3, 2, 2, 0).unwrap();

        // h1: add POWERED flag
        store.set_flags(h1, StatusFlags::UNDER_CONSTRUCTION | StatusFlags::POWERED);
        // h2: keep default (UNDER_CONSTRUCTION only)
        // h3: add POWERED flag
        store.set_flags(h3, StatusFlags::POWERED);

        let powered: Vec<EntityHandle> = store.iter_with_flags(StatusFlags::POWERED).collect();
        assert_eq!(powered.len(), 2);
        assert!(powered.contains(&h1));
        assert!(!powered.contains(&h2));
        assert!(powered.contains(&h3));
    }

    #[test]
    fn iter_with_flags_under_construction() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        let h2 = store.alloc(2, 0, 0, 0).unwrap();

        // h2: clear UNDER_CONSTRUCTION
        store.set_flags(h2, StatusFlags::NONE);

        let under_construction: Vec<EntityHandle> =
            store.iter_with_flags(StatusFlags::UNDER_CONSTRUCTION).collect();
        assert_eq!(under_construction.len(), 1);
        assert!(under_construction.contains(&h1));
    }

    #[test]
    fn set_and_get_flags() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 0, 0, 0).unwrap();
        let new_flags = StatusFlags::POWERED | StatusFlags::STAFFED;
        assert!(store.set_flags(h, new_flags));
        assert_eq!(store.get_flags(h), Some(new_flags));
    }

    #[test]
    fn set_and_get_level() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 0, 0, 0).unwrap();
        assert_eq!(store.get_level(h), Some(1)); // default
        assert!(store.set_level(h, 5));
        assert_eq!(store.get_level(h), Some(5));
    }

    #[test]
    fn set_and_get_construction_progress() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 0, 0, 0).unwrap();
        assert_eq!(store.get_construction_progress(h), Some(0));
        assert!(store.set_construction_progress(h, 0x8000));
        assert_eq!(store.get_construction_progress(h), Some(0x8000));
        assert!(store.set_construction_progress(h, 0xFFFF));
        assert_eq!(store.get_construction_progress(h), Some(0xFFFF));
    }

    #[test]
    fn set_and_get_enabled() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 0, 0, 0).unwrap();
        assert_eq!(store.get_enabled(h), Some(true)); // default
        assert!(store.set_enabled(h, false));
        assert_eq!(store.get_enabled(h), Some(false));
    }

    #[test]
    fn get_set_on_invalid_handle_returns_none_or_false() {
        let mut store = EntityStore::new(16);
        let invalid = EntityHandle::INVALID;
        assert_eq!(store.get_pos(invalid), None);
        assert_eq!(store.get_archetype(invalid), None);
        assert_eq!(store.get_flags(invalid), None);
        assert_eq!(store.get_level(invalid), None);
        assert_eq!(store.get_construction_progress(invalid), None);
        assert_eq!(store.get_enabled(invalid), None);
        assert!(!store.set_flags(invalid, StatusFlags::POWERED));
        assert!(!store.set_level(invalid, 5));
        assert!(!store.set_construction_progress(invalid, 100));
        assert!(!store.set_enabled(invalid, false));
    }

    #[test]
    fn get_set_on_freed_handle_returns_none_or_false() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 5, 10, 0).unwrap();
        store.free(h);
        assert_eq!(store.get_pos(h), None);
        assert_eq!(store.get_archetype(h), None);
        assert_eq!(store.get_flags(h), None);
        assert_eq!(store.get_level(h), None);
        assert!(!store.set_flags(h, StatusFlags::POWERED));
        assert!(!store.set_level(h, 3));
    }

    #[test]
    fn multiple_alloc_free_cycles() {
        let mut store = EntityStore::new(4);

        // Fill up
        let handles: Vec<EntityHandle> = (0..4)
            .map(|i| store.alloc(i as u16, i as i16, i as i16, 0).unwrap())
            .collect();
        assert_eq!(store.count(), 4);
        assert!(store.alloc(99, 0, 0, 0).is_none());

        // Free all
        for h in &handles {
            assert!(store.free(*h));
        }
        assert_eq!(store.count(), 0);

        // All old handles are now stale
        for h in &handles {
            assert!(!store.is_valid(*h));
        }

        // Re-allocate all slots
        let handles2: Vec<EntityHandle> = (0..4)
            .map(|i| store.alloc(i as u16 + 10, i as i16, i as i16, 0).unwrap())
            .collect();
        assert_eq!(store.count(), 4);

        // All new handles should be generation 1
        for h in &handles2 {
            assert_eq!(h.generation, 1);
            assert!(store.is_valid(*h));
        }

        // Free and re-allocate again
        for h in &handles2 {
            store.free(*h);
        }
        let handles3: Vec<EntityHandle> = (0..4)
            .map(|i| store.alloc(i as u16 + 20, 0, 0, 0).unwrap())
            .collect();
        for h in &handles3 {
            assert_eq!(h.generation, 2);
            assert!(store.is_valid(*h));
        }
    }

    #[test]
    fn slot_reuse_after_free() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 10, 20, 0).unwrap();
        let slot = h1.index;
        store.free(h1);

        let h2 = store.alloc(2, 30, 40, 1).unwrap();
        // Should reuse the same slot
        assert_eq!(h2.index, slot);
        // But with incremented generation
        assert_eq!(h2.generation, h1.generation + 1);
        // New data
        assert_eq!(store.get_pos(h2), Some(TileCoord::new(30, 40)));
        assert_eq!(store.get_archetype(h2), Some(2));
    }

    #[test]
    fn alloc_assigns_sequential_slots_initially() {
        let mut store = EntityStore::new(16);
        let h0 = store.alloc(0, 0, 0, 0).unwrap();
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        let h2 = store.alloc(2, 0, 0, 0).unwrap();
        // Free list was built with .rev(), so slot 0 is popped first
        assert_eq!(h0.index, 0);
        assert_eq!(h1.index, 1);
        assert_eq!(h2.index, 2);
    }

    #[test]
    fn free_double_free_returns_false() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, 0, 0, 0).unwrap();
        assert!(store.free(h));
        assert!(!store.free(h)); // double free
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn position_with_negative_coords() {
        let mut store = EntityStore::new(16);
        let h = store.alloc(1, -50, -100, 3).unwrap();
        assert_eq!(store.get_pos(h), Some(TileCoord::new(-50, -100)));
    }
}
