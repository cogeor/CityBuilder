//! SoA entity storage with generational handles.
//!
//! Entities are stored as parallel arrays for cache-friendly iteration.
//! Each entity slot has a generation counter; stale handles are detected
//! by comparing handle.generation against the slot's current generation.

use city_core::*;

/// Maximum number of entities.
pub const MAX_ENTITIES: usize = 65536;

/// Data stored per-entity in SoA layout.
/// Each field is a separate array for cache-friendly access patterns.
#[derive(Debug)]
pub struct EntityStore {
    // ─── Per-slot metadata ────
    generation: Vec<u32>,
    alive: Vec<bool>,

    // ─── Entity fields (SoA) ─────
    pub archetype_id: Vec<ArchetypeId>,
    pub pos_x: Vec<i16>,
    pub pos_y: Vec<i16>,
    pub rotation: Vec<u8>,
    pub level: Vec<u8>,
    pub flags: Vec<StatusFlags>,
    pub construction_progress: Vec<u16>,
    pub enabled: Vec<bool>,

    // ─── Bookkeeping ─────────────
    free_list: Vec<u32>,
    count: u32,
}

impl EntityStore {
    /// Create a new empty entity store with pre-allocated capacity.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.min(MAX_ENTITIES);
        let free_list: Vec<u32> = (0..cap as u32).rev().collect();

        EntityStore {
            generation: vec![0; cap],
            alive: vec![false; cap],
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
        self.alive[idx] = true;
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

    /// Free an entity by handle. Returns true if freed successfully.
    pub fn free(&mut self, handle: EntityHandle) -> bool {
        if !self.is_valid(handle) {
            return false;
        }
        let idx = handle.index as usize;
        self.alive[idx] = false;
        self.generation[idx] = self.generation[idx].wrapping_add(1);
        self.free_list.push(handle.index);
        self.count -= 1;
        true
    }

    /// Check if a handle refers to a currently alive entity.
    #[inline]
    pub fn is_valid(&self, handle: EntityHandle) -> bool {
        let idx = handle.index as usize;
        idx < self.alive.len()
            && self.alive[idx]
            && self.generation[idx] == handle.generation
    }

    #[inline]
    pub fn count(&self) -> u32 { self.count }

    #[inline]
    pub fn capacity(&self) -> usize { self.alive.len() }

    #[inline]
    pub fn get_pos(&self, handle: EntityHandle) -> Option<TileCoord> {
        if !self.is_valid(handle) { return None; }
        let idx = handle.index as usize;
        Some(TileCoord::new(self.pos_x[idx], self.pos_y[idx]))
    }

    #[inline]
    pub fn get_archetype(&self, handle: EntityHandle) -> Option<ArchetypeId> {
        if !self.is_valid(handle) { return None; }
        Some(self.archetype_id[handle.index as usize])
    }

    #[inline]
    pub fn get_flags(&self, handle: EntityHandle) -> Option<StatusFlags> {
        if !self.is_valid(handle) { return None; }
        Some(self.flags[handle.index as usize])
    }

    #[inline]
    pub fn set_flags(&mut self, handle: EntityHandle, flags: StatusFlags) -> bool {
        if !self.is_valid(handle) { return false; }
        self.flags[handle.index as usize] = flags;
        true
    }

    #[inline]
    pub fn get_level(&self, handle: EntityHandle) -> Option<u8> {
        if !self.is_valid(handle) { return None; }
        Some(self.level[handle.index as usize])
    }

    #[inline]
    pub fn set_level(&mut self, handle: EntityHandle, level: u8) -> bool {
        if !self.is_valid(handle) { return false; }
        self.level[handle.index as usize] = level;
        true
    }

    #[inline]
    pub fn get_construction_progress(&self, handle: EntityHandle) -> Option<u16> {
        if !self.is_valid(handle) { return None; }
        Some(self.construction_progress[handle.index as usize])
    }

    #[inline]
    pub fn set_construction_progress(&mut self, handle: EntityHandle, progress: u16) -> bool {
        if !self.is_valid(handle) { return false; }
        self.construction_progress[handle.index as usize] = progress;
        true
    }

    #[inline]
    pub fn get_enabled(&self, handle: EntityHandle) -> Option<bool> {
        if !self.is_valid(handle) { return None; }
        Some(self.enabled[handle.index as usize])
    }

    #[inline]
    pub fn set_enabled(&mut self, handle: EntityHandle, en: bool) -> bool {
        if !self.is_valid(handle) { return false; }
        self.enabled[handle.index as usize] = en;
        true
    }

    /// Iterate over all alive entity handles.
    pub fn iter_alive(&self) -> impl Iterator<Item = EntityHandle> + '_ {
        self.alive.iter().enumerate().filter_map(|(i, &alive)| {
            if alive {
                Some(EntityHandle::new(i as u32, self.generation[i]))
            } else {
                None
            }
        })
    }

    /// Iterate alive entities that have specific flags set.
    pub fn iter_with_flags(&self, required: StatusFlags) -> impl Iterator<Item = EntityHandle> + '_ {
        self.alive.iter().enumerate().filter_map(move |(i, &alive)| {
            if alive && self.flags[i].contains(required) {
                Some(EntityHandle::new(i as u32, self.generation[i]))
            } else {
                None
            }
        })
    }
}

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
        assert!(!store.is_valid(h1));
        assert_eq!(store.get_pos(h1), None);
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
    fn capacity_respects_max() {
        let store = EntityStore::new(MAX_ENTITIES + 100);
        assert_eq!(store.capacity(), MAX_ENTITIES);
    }

    #[test]
    fn iter_alive_returns_all_alive() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        let h2 = store.alloc(2, 1, 1, 0).unwrap();
        let h3 = store.alloc(3, 2, 2, 0).unwrap();
        store.free(h1);
        let alive: Vec<EntityHandle> = store.iter_alive().collect();
        assert_eq!(alive.len(), 2);
        assert!(!alive.contains(&h1));
        assert!(alive.contains(&h2));
        assert!(alive.contains(&h3));
    }

    #[test]
    fn iter_with_flags_filters() {
        let mut store = EntityStore::new(16);
        let h1 = store.alloc(1, 0, 0, 0).unwrap();
        let h2 = store.alloc(2, 1, 1, 0).unwrap();
        let h3 = store.alloc(3, 2, 2, 0).unwrap();
        store.set_flags(h1, StatusFlags::UNDER_CONSTRUCTION | StatusFlags::POWERED);
        // h2: keep default (UNDER_CONSTRUCTION only)
        store.set_flags(h3, StatusFlags::POWERED);
        let powered: Vec<EntityHandle> = store.iter_with_flags(StatusFlags::POWERED).collect();
        assert_eq!(powered.len(), 2);
        assert!(powered.contains(&h1));
        assert!(!powered.contains(&h2));
        assert!(powered.contains(&h3));
    }

    #[test]
    fn invalid_handle_ops() {
        let mut store = EntityStore::new(16);
        let invalid = EntityHandle::INVALID;
        assert_eq!(store.get_pos(invalid), None);
        assert_eq!(store.get_archetype(invalid), None);
        assert!(!store.set_flags(invalid, StatusFlags::POWERED));
        assert!(!store.free(invalid));
    }

    #[test]
    fn multiple_cycles() {
        let mut store = EntityStore::new(4);
        let handles: Vec<EntityHandle> = (0..4)
            .map(|i| store.alloc(i as u16, i as i16, i as i16, 0).unwrap())
            .collect();
        assert_eq!(store.count(), 4);
        for h in &handles { store.free(*h); }
        assert_eq!(store.count(), 0);
        let handles2: Vec<EntityHandle> = (0..4)
            .map(|i| store.alloc(i as u16, 0, 0, 0).unwrap())
            .collect();
        for h in &handles2 { assert_eq!(h.generation, 1); }
    }
}
