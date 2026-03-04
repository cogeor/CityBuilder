//! Cache invalidation framework with dirty flags.
//!
//! Caches are marked dirty when topology changes. Actual rebuild
//! happens lazily on first access after invalidation.

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
}
