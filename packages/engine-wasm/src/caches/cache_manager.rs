//! Cache invalidation framework with dirty flags.
//!
//! Caches are marked dirty when topology changes. Actual rebuild
//! happens lazily on first access after invalidation.
//!
//! Also provides [`DirtyTileSet`], a `Vec<u64>`-backed per-tile dirty bitset
//! used to track which map tiles need analysis map recomputation.

// ─── DirtyTileSet ─────────────────────────────────────────────────────────────

/// Per-tile dirty bitset for analysis map invalidation.
///
/// Internally stores one bit per tile in `Vec<u64>` words so a 256×256 map
/// uses exactly 1024 u64s (8 KiB). Tile index `t = y * width + x`:
/// - word: `t / 64`
/// - bit:  `1u64 << (t % 64)`
#[derive(Debug, Clone)]
pub struct DirtyTileSet {
    words: Vec<u64>,
    width: u16,
    height: u16,
}

impl DirtyTileSet {
    /// Create a new all-clean dirty set for a map of the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let tiles = width as usize * height as usize;
        let word_count = (tiles + 63) / 64;
        DirtyTileSet {
            words: vec![0u64; word_count],
            width,
            height,
        }
    }

    /// Mark a single tile as dirty. No-op for out-of-bounds coordinates.
    #[inline]
    pub fn mark(&mut self, x: i16, y: i16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        let t = y as usize * self.width as usize + x as usize;
        self.words[t / 64] |= 1u64 << (t % 64);
    }

    /// Mark all tiles within the rectangle `[x, x+w) × [y, y+h)` as dirty.
    pub fn mark_region(&mut self, x: i16, y: i16, w: u8, h: u8) {
        for dy in 0..h as i16 {
            for dx in 0..w as i16 {
                self.mark(x + dx, y + dy);
            }
        }
    }

    /// Mark all tiles within manhattan distance `radius` of `(cx, cy)` dirty.
    pub fn mark_manhattan(&mut self, cx: i16, cy: i16, radius: u8) {
        let r = radius as i16;
        for dy in -r..=r {
            let rem = r - dy.abs();
            for dx in -rem..=rem {
                self.mark(cx + dx, cy + dy);
            }
        }
    }

    /// Returns `true` if the tile at `(x, y)` is dirty.
    #[inline]
    pub fn is_set(&self, x: i16, y: i16) -> bool {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return false;
        }
        let t = y as usize * self.width as usize + x as usize;
        (self.words[t / 64] >> (t % 64)) & 1 != 0
    }

    /// Returns `true` if any tile is dirty.
    pub fn any(&self) -> bool {
        self.words.iter().any(|&w| w != 0)
    }

    /// Clear all dirty bits.
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Iterate flat tile indices of all dirty tiles.
    pub fn iter_dirty_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(wi, &word)| {
            DirtyBitsIter { word, base: wi * 64 }
        })
    }

    /// Map width.
    #[inline]
    pub fn width(&self) -> u16 { self.width }

    /// Map height.
    #[inline]
    pub fn height(&self) -> u16 { self.height }
}

/// Iterator that yields set bit positions within a single u64 word.
struct DirtyBitsIter {
    word: u64,
    base: usize,
}

impl Iterator for DirtyBitsIter {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.word == 0 {
            return None;
        }
        let bit = self.word.trailing_zeros() as usize;
        self.word &= self.word - 1; // clear lowest set bit
        Some(self.base + bit)
    }
}

// ─── CacheType ────────────────────────────────────────────────────────────────

/// Types of caches that can be invalidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CacheType {
    PathField = 0,
    TrafficDensity = 1,
    PowerNetwork = 2,
    WaterNetwork = 3,
    CoverageMap = 4,
    EconomicAggregates = 5,
    Pollution = 6,
    Crime = 7,
    Desirability = 8,
    LandValue = 9,
    Noise = 10,
}

/// Number of cache types.
pub const CACHE_TYPE_COUNT: usize = 11;

/// Reasons a cache can be invalidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationReason {
    RoadAdded,
    RoadRemoved,
    BuildingPlaced,
    BuildingRemoved,
    PowerPlantChange,
    WaterFacilityChange,
    PolicyChanged,
    PopulationChanged,
}

/// Manages dirty flags for all cache types.
#[derive(Debug)]
pub struct CacheManager {
    /// Dirty flags per cache type.
    dirty: [bool; CACHE_TYPE_COUNT],
    /// Version counters per cache type (incremented on rebuild).
    versions: [u32; CACHE_TYPE_COUNT],
}

impl CacheManager {
    pub fn new() -> Self {
        CacheManager {
            dirty: [true; CACHE_TYPE_COUNT], // all dirty initially (need first build)
            versions: [0; CACHE_TYPE_COUNT],
        }
    }

    /// Mark a specific cache as dirty.
    pub fn mark_dirty(&mut self, cache: CacheType) {
        self.dirty[cache as usize] = true;
    }

    /// Mark caches dirty based on an invalidation reason.
    pub fn invalidate(&mut self, reason: InvalidationReason) {
        match reason {
            InvalidationReason::RoadAdded | InvalidationReason::RoadRemoved => {
                self.mark_dirty(CacheType::PathField);
                self.mark_dirty(CacheType::TrafficDensity);
                self.mark_dirty(CacheType::Noise);
            }
            InvalidationReason::BuildingPlaced | InvalidationReason::BuildingRemoved => {
                self.mark_dirty(CacheType::CoverageMap);
                self.mark_dirty(CacheType::EconomicAggregates);
                self.mark_dirty(CacheType::Desirability);
                self.mark_dirty(CacheType::LandValue);
                self.mark_dirty(CacheType::Pollution);
                self.mark_dirty(CacheType::Noise);
            }
            InvalidationReason::PowerPlantChange => {
                self.mark_dirty(CacheType::PowerNetwork);
            }
            InvalidationReason::WaterFacilityChange => {
                self.mark_dirty(CacheType::WaterNetwork);
            }
            InvalidationReason::PolicyChanged => {
                self.mark_dirty(CacheType::EconomicAggregates);
            }
            InvalidationReason::PopulationChanged => {
                self.mark_dirty(CacheType::EconomicAggregates);
                self.mark_dirty(CacheType::TrafficDensity);
            }
        }
    }

    /// Check if a cache is dirty.
    #[inline]
    pub fn is_dirty(&self, cache: CacheType) -> bool {
        self.dirty[cache as usize]
    }

    /// Mark a cache as clean (after rebuild). Increments version.
    pub fn mark_clean(&mut self, cache: CacheType) {
        self.dirty[cache as usize] = false;
        self.versions[cache as usize] += 1;
    }

    /// Get the current version of a cache.
    #[inline]
    pub fn version(&self, cache: CacheType) -> u32 {
        self.versions[cache as usize]
    }

    /// Mark all caches as dirty (e.g., after loading a save).
    pub fn invalidate_all(&mut self) {
        self.dirty = [true; CACHE_TYPE_COUNT];
    }

    /// Check if any cache is dirty.
    pub fn any_dirty(&self) -> bool {
        self.dirty.iter().any(|&d| d)
    }

    /// Get list of all dirty cache types.
    pub fn dirty_caches(&self) -> Vec<CacheType> {
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        all.iter().filter(|&&c| self.is_dirty(c)).copied().collect()
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_all_dirty() {
        let cm = CacheManager::new();
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        for cache in &all {
            assert!(cm.is_dirty(*cache), "{:?} should be dirty on new()", cache);
        }
    }

    #[test]
    fn mark_dirty_sets_specific_cache() {
        let mut cm = CacheManager::new();
        // Clean everything first
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        for cache in &all {
            cm.mark_clean(*cache);
        }
        assert!(!cm.any_dirty());

        // Mark only one dirty
        cm.mark_dirty(CacheType::PowerNetwork);
        assert!(cm.is_dirty(CacheType::PowerNetwork));
        assert!(!cm.is_dirty(CacheType::PathField));
        assert!(!cm.is_dirty(CacheType::TrafficDensity));
    }

    #[test]
    fn mark_clean_clears_dirty_and_increments_version() {
        let mut cm = CacheManager::new();
        assert_eq!(cm.version(CacheType::PathField), 0);
        assert!(cm.is_dirty(CacheType::PathField));

        cm.mark_clean(CacheType::PathField);
        assert!(!cm.is_dirty(CacheType::PathField));
        assert_eq!(cm.version(CacheType::PathField), 1);

        // Mark dirty and clean again to verify version increments
        cm.mark_dirty(CacheType::PathField);
        cm.mark_clean(CacheType::PathField);
        assert_eq!(cm.version(CacheType::PathField), 2);
    }

    #[test]
    fn invalidate_road_added_marks_correct_caches() {
        let mut cm = CacheManager::new();
        // Clean everything
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        for cache in &all {
            cm.mark_clean(*cache);
        }

        cm.invalidate(InvalidationReason::RoadAdded);

        // Should be dirty
        assert!(cm.is_dirty(CacheType::PathField));
        assert!(cm.is_dirty(CacheType::TrafficDensity));
        assert!(cm.is_dirty(CacheType::Noise));

        // Should NOT be dirty
        assert!(!cm.is_dirty(CacheType::PowerNetwork));
        assert!(!cm.is_dirty(CacheType::WaterNetwork));
        assert!(!cm.is_dirty(CacheType::CoverageMap));
        assert!(!cm.is_dirty(CacheType::EconomicAggregates));
        assert!(!cm.is_dirty(CacheType::Pollution));
        assert!(!cm.is_dirty(CacheType::Crime));
        assert!(!cm.is_dirty(CacheType::Desirability));
        assert!(!cm.is_dirty(CacheType::LandValue));
    }

    #[test]
    fn invalidate_building_placed_marks_correct_caches() {
        let mut cm = CacheManager::new();
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        for cache in &all {
            cm.mark_clean(*cache);
        }

        cm.invalidate(InvalidationReason::BuildingPlaced);

        // Should be dirty
        assert!(cm.is_dirty(CacheType::CoverageMap));
        assert!(cm.is_dirty(CacheType::EconomicAggregates));
        assert!(cm.is_dirty(CacheType::Desirability));
        assert!(cm.is_dirty(CacheType::LandValue));
        assert!(cm.is_dirty(CacheType::Pollution));
        assert!(cm.is_dirty(CacheType::Noise));

        // Should NOT be dirty
        assert!(!cm.is_dirty(CacheType::PathField));
        assert!(!cm.is_dirty(CacheType::TrafficDensity));
        assert!(!cm.is_dirty(CacheType::PowerNetwork));
        assert!(!cm.is_dirty(CacheType::WaterNetwork));
        assert!(!cm.is_dirty(CacheType::Crime));
    }

    #[test]
    fn invalidate_policy_changed_marks_only_economic() {
        let mut cm = CacheManager::new();
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        for cache in &all {
            cm.mark_clean(*cache);
        }

        cm.invalidate(InvalidationReason::PolicyChanged);

        assert!(cm.is_dirty(CacheType::EconomicAggregates));

        // Everything else should be clean
        assert!(!cm.is_dirty(CacheType::PathField));
        assert!(!cm.is_dirty(CacheType::TrafficDensity));
        assert!(!cm.is_dirty(CacheType::PowerNetwork));
        assert!(!cm.is_dirty(CacheType::WaterNetwork));
        assert!(!cm.is_dirty(CacheType::CoverageMap));
        assert!(!cm.is_dirty(CacheType::Pollution));
        assert!(!cm.is_dirty(CacheType::Crime));
        assert!(!cm.is_dirty(CacheType::Desirability));
        assert!(!cm.is_dirty(CacheType::LandValue));
        assert!(!cm.is_dirty(CacheType::Noise));
    }

    #[test]
    fn invalidate_all_marks_everything_dirty() {
        let mut cm = CacheManager::new();
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        // Clean everything first
        for cache in &all {
            cm.mark_clean(*cache);
        }
        assert!(!cm.any_dirty());

        cm.invalidate_all();

        for cache in &all {
            assert!(cm.is_dirty(*cache), "{:?} should be dirty after invalidate_all", cache);
        }
    }

    #[test]
    fn any_dirty_works_correctly() {
        let mut cm = CacheManager::new();
        assert!(cm.any_dirty()); // all dirty initially

        // Clean everything
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        for cache in &all {
            cm.mark_clean(*cache);
        }
        assert!(!cm.any_dirty());

        // Dirty one
        cm.mark_dirty(CacheType::Crime);
        assert!(cm.any_dirty());
    }

    #[test]
    fn dirty_caches_returns_correct_list() {
        let mut cm = CacheManager::new();
        // Clean everything
        let all = [
            CacheType::PathField, CacheType::TrafficDensity,
            CacheType::PowerNetwork, CacheType::WaterNetwork,
            CacheType::CoverageMap, CacheType::EconomicAggregates,
            CacheType::Pollution, CacheType::Crime,
            CacheType::Desirability, CacheType::LandValue,
            CacheType::Noise,
        ];
        for cache in &all {
            cm.mark_clean(*cache);
        }

        assert!(cm.dirty_caches().is_empty());

        cm.mark_dirty(CacheType::PathField);
        cm.mark_dirty(CacheType::Crime);

        let dirty = cm.dirty_caches();
        assert_eq!(dirty.len(), 2);
        assert!(dirty.contains(&CacheType::PathField));
        assert!(dirty.contains(&CacheType::Crime));
    }

    #[test]
    fn version_increments_on_rebuild() {
        let mut cm = CacheManager::new();
        assert_eq!(cm.version(CacheType::TrafficDensity), 0);

        cm.mark_clean(CacheType::TrafficDensity);
        assert_eq!(cm.version(CacheType::TrafficDensity), 1);

        cm.mark_dirty(CacheType::TrafficDensity);
        cm.mark_clean(CacheType::TrafficDensity);
        assert_eq!(cm.version(CacheType::TrafficDensity), 2);

        cm.mark_dirty(CacheType::TrafficDensity);
        cm.mark_clean(CacheType::TrafficDensity);
        assert_eq!(cm.version(CacheType::TrafficDensity), 3);

        // Other caches should still be at version 0
        assert_eq!(cm.version(CacheType::PathField), 0);
    }

    // ── DirtyTileSet tests ───────────────────────────────────────────────────

    #[test]
    fn dirty_tile_set_new_is_all_clean() {
        let dts = DirtyTileSet::new(8, 8);
        for y in 0..8i16 {
            for x in 0..8i16 {
                assert!(!dts.is_set(x, y), "({x},{y}) should be clean");
            }
        }
        assert!(!dts.any());
    }

    #[test]
    fn dirty_tile_set_mark_single_tile() {
        let mut dts = DirtyTileSet::new(16, 16);
        dts.mark(5, 7);
        assert!(dts.is_set(5, 7));
        assert!(!dts.is_set(4, 7));
        assert!(!dts.is_set(6, 7));
        assert!(dts.any());
    }

    #[test]
    fn dirty_tile_set_out_of_bounds_no_panic() {
        let mut dts = DirtyTileSet::new(8, 8);
        dts.mark(-1, 0);
        dts.mark(0, -1);
        dts.mark(8, 0);
        dts.mark(0, 8);
        assert!(!dts.any());
        assert!(!dts.is_set(-1, 0));
        assert!(!dts.is_set(100, 100));
    }

    #[test]
    fn dirty_tile_set_clear_resets_all() {
        let mut dts = DirtyTileSet::new(4, 4);
        dts.mark(0, 0);
        dts.mark(3, 3);
        assert!(dts.any());
        dts.clear();
        assert!(!dts.any());
        for y in 0..4i16 {
            for x in 0..4i16 {
                assert!(!dts.is_set(x, y));
            }
        }
    }

    #[test]
    fn dirty_tile_set_mark_region_covers_all_tiles() {
        let mut dts = DirtyTileSet::new(10, 10);
        dts.mark_region(2, 3, 3, 2); // 3×2 region at (2,3)
        for dy in 0..2i16 {
            for dx in 0..3i16 {
                assert!(dts.is_set(2 + dx, 3 + dy), "({},{}) not dirty", 2 + dx, 3 + dy);
            }
        }
        // Tile just outside
        assert!(!dts.is_set(5, 3));
        assert!(!dts.is_set(2, 5));
    }

    #[test]
    fn dirty_tile_set_iter_dirty_yields_all_marked() {
        let mut dts = DirtyTileSet::new(8, 8);
        dts.mark(0, 0);
        dts.mark(7, 7);
        dts.mark(3, 4);

        let expected: Vec<usize> = vec![
            0 * 8 + 0, // (0,0)
            4 * 8 + 3, // (3,4)
            7 * 8 + 7, // (7,7)
        ];
        let mut got: Vec<usize> = dts.iter_dirty_indices().collect();
        got.sort_unstable();
        assert_eq!(got, expected);
    }

    #[test]
    fn dirty_tile_set_mark_manhattan_covers_diamond() {
        let mut dts = DirtyTileSet::new(10, 10);
        dts.mark_manhattan(5, 5, 2);
        // Center and all tiles with manhattan dist <= 2 should be dirty.
        // Diamond has 1 + 4 + 8 = 13 tiles.
        let count = dts.iter_dirty_indices().count();
        assert_eq!(count, 13);
        assert!(dts.is_set(5, 5)); // center
        assert!(dts.is_set(3, 5)); // dist 2
        assert!(dts.is_set(7, 5)); // dist 2
        assert!(!dts.is_set(3, 3)); // dist 4, outside
    }

    #[test]
    fn dirty_tile_set_word_boundary_tile_63_and_64() {
        // Tile 63 is in word 0, bit 63. Tile 64 is in word 1, bit 0.
        let mut dts = DirtyTileSet::new(128, 128);
        // Tile 63 = (63, 0) in a 128-wide map
        dts.mark(63, 0);
        assert!(dts.is_set(63, 0));
        assert!(!dts.is_set(64, 0));
        // Tile 64 = (64, 0)
        dts.mark(64, 0);
        assert!(dts.is_set(64, 0));

        let indices: Vec<usize> = dts.iter_dirty_indices().collect();
        assert!(indices.contains(&63));
        assert!(indices.contains(&64));
    }
}
