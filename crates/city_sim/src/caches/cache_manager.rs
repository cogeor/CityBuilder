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
    dirty: [bool; CACHE_TYPE_COUNT],
    versions: [u32; CACHE_TYPE_COUNT],
}

impl CacheManager {
    pub fn new() -> Self {
        CacheManager {
            dirty: [true; CACHE_TYPE_COUNT],
            versions: [0; CACHE_TYPE_COUNT],
        }
    }

    pub fn mark_dirty(&mut self, cache: CacheType) {
        self.dirty[cache as usize] = true;
    }

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

    #[inline]
    pub fn is_dirty(&self, cache: CacheType) -> bool {
        self.dirty[cache as usize]
    }

    pub fn mark_clean(&mut self, cache: CacheType) {
        self.dirty[cache as usize] = false;
        self.versions[cache as usize] += 1;
    }

    #[inline]
    pub fn version(&self, cache: CacheType) -> u32 {
        self.versions[cache as usize]
    }

    pub fn invalidate_all(&mut self) {
        self.dirty = [true; CACHE_TYPE_COUNT];
    }

    pub fn any_dirty(&self) -> bool {
        self.dirty.iter().any(|&d| d)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [CacheType; 11] = [
        CacheType::PathField, CacheType::TrafficDensity,
        CacheType::PowerNetwork, CacheType::WaterNetwork,
        CacheType::CoverageMap, CacheType::EconomicAggregates,
        CacheType::Pollution, CacheType::Crime,
        CacheType::Desirability, CacheType::LandValue,
        CacheType::Noise,
    ];

    fn clean_all(cm: &mut CacheManager) {
        for cache in &ALL { cm.mark_clean(*cache); }
    }

    #[test]
    fn new_starts_all_dirty() {
        let cm = CacheManager::new();
        for cache in &ALL {
            assert!(cm.is_dirty(*cache));
        }
    }

    #[test]
    fn mark_dirty_sets_specific_cache() {
        let mut cm = CacheManager::new();
        clean_all(&mut cm);
        assert!(!cm.any_dirty());
        cm.mark_dirty(CacheType::PowerNetwork);
        assert!(cm.is_dirty(CacheType::PowerNetwork));
        assert!(!cm.is_dirty(CacheType::PathField));
    }

    #[test]
    fn mark_clean_clears_dirty_and_increments_version() {
        let mut cm = CacheManager::new();
        assert_eq!(cm.version(CacheType::PathField), 0);
        cm.mark_clean(CacheType::PathField);
        assert!(!cm.is_dirty(CacheType::PathField));
        assert_eq!(cm.version(CacheType::PathField), 1);
        cm.mark_dirty(CacheType::PathField);
        cm.mark_clean(CacheType::PathField);
        assert_eq!(cm.version(CacheType::PathField), 2);
    }

    #[test]
    fn invalidate_road_added() {
        let mut cm = CacheManager::new();
        clean_all(&mut cm);
        cm.invalidate(InvalidationReason::RoadAdded);
        assert!(cm.is_dirty(CacheType::PathField));
        assert!(cm.is_dirty(CacheType::TrafficDensity));
        assert!(cm.is_dirty(CacheType::Noise));
        assert!(!cm.is_dirty(CacheType::PowerNetwork));
    }

    #[test]
    fn invalidate_building_placed() {
        let mut cm = CacheManager::new();
        clean_all(&mut cm);
        cm.invalidate(InvalidationReason::BuildingPlaced);
        assert!(cm.is_dirty(CacheType::CoverageMap));
        assert!(cm.is_dirty(CacheType::EconomicAggregates));
        assert!(cm.is_dirty(CacheType::Desirability));
        assert!(cm.is_dirty(CacheType::LandValue));
        assert!(cm.is_dirty(CacheType::Pollution));
        assert!(cm.is_dirty(CacheType::Noise));
        assert!(!cm.is_dirty(CacheType::PathField));
    }

    #[test]
    fn invalidate_policy_changed() {
        let mut cm = CacheManager::new();
        clean_all(&mut cm);
        cm.invalidate(InvalidationReason::PolicyChanged);
        assert!(cm.is_dirty(CacheType::EconomicAggregates));
        assert!(!cm.is_dirty(CacheType::PathField));
    }

    #[test]
    fn invalidate_all_marks_everything() {
        let mut cm = CacheManager::new();
        clean_all(&mut cm);
        assert!(!cm.any_dirty());
        cm.invalidate_all();
        for cache in &ALL {
            assert!(cm.is_dirty(*cache));
        }
    }

    #[test]
    fn dirty_caches_returns_correct_list() {
        let mut cm = CacheManager::new();
        clean_all(&mut cm);
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
        assert_eq!(cm.version(CacheType::PathField), 0);
    }
}
