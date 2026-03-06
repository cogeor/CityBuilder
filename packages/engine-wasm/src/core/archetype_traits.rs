//! Archetype trait cache: Flyweight optimization for hot-path lookups.
//!
//! Precomputes compact trait structs from `ArchetypeDefinition` so that
//! simulation hot paths can query archetype properties via O(1) index
//! lookup instead of hash-map access + field inspection.

use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use crate::core_types::ArchetypeId;

// ─── CategoryFlags ──────────────────────────────────────────────────────────

/// Compact bitflags for archetype category.
///
/// Multiple categories can be combined (e.g., a civic building that is also
/// a service provider). Uses a `u16` backing store with one bit per category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryFlags(u16);

impl CategoryFlags {
    pub const NONE: Self = Self(0);
    pub const RESIDENTIAL: Self = Self(1 << 0);
    pub const COMMERCIAL: Self = Self(1 << 1);
    pub const INDUSTRIAL: Self = Self(1 << 2);
    pub const CIVIC: Self = Self(1 << 3);
    pub const UTILITY: Self = Self(1 << 4);
    pub const TRANSPORT: Self = Self(1 << 5);
    pub const PARK: Self = Self(1 << 6);
    pub const EDUCATION: Self = Self(1 << 7);
    pub const HEALTHCARE: Self = Self(1 << 8);

    /// Returns `true` if all bits in `other` are set in `self`.
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns `true` if no category bits are set.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Combine two category flag sets.
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl std::ops::BitOr for CategoryFlags {
    type Output = CategoryFlags;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

// ─── CapacityClass ──────────────────────────────────────────────────────────

/// Capacity classification for quick capacity lookups.
///
/// Derived from the combined resident + job capacity of an archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityClass {
    /// No capacity (e.g., roads, parks, decorations).
    None,
    /// 1-10 units of capacity.
    Small,
    /// 11-50 units of capacity.
    Medium,
    /// 51+ units of capacity.
    Large,
}

// ─── ArchetypeTraits ────────────────────────────────────────────────────────

/// Precomputed compact archetype traits for hot-path lookups.
///
/// Each field is derived from `ArchetypeDefinition` at cache-build time
/// so that simulation systems can avoid repeated computation.
#[derive(Debug, Clone)]
pub struct ArchetypeTraits {
    pub id: ArchetypeId,
    pub category: CategoryFlags,
    pub capacity_class: CapacityClass,
    pub provides_power: bool,
    pub provides_water: bool,
    pub is_employer: bool,
    /// Pollution level: 0=none, 1=low, 2=medium, 3=high.
    pub pollution_level: u8,
    pub service_provider: bool,
}

// ─── ArchetypeTraitCache ────────────────────────────────────────────────────

/// Cache of precomputed archetype traits for hot-path lookups.
///
/// Uses a flat `Vec` indexed by `ArchetypeId` for O(1) access.
/// Slots for unregistered IDs are `None`.
pub struct ArchetypeTraitCache {
    traits: Vec<Option<ArchetypeTraits>>,
}

impl ArchetypeTraitCache {
    /// Build the cache from an `ArchetypeRegistry`.
    ///
    /// Iterates all registered archetypes, computes traits for each, and
    /// stores them in a flat vector indexed by archetype ID.
    pub fn from_registry(registry: &ArchetypeRegistry) -> Self {
        let ids = registry.list_ids();
        if ids.is_empty() {
            return Self { traits: Vec::new() };
        }

        let max_id = ids.iter().copied().max().unwrap_or(0) as usize;
        let mut traits = vec![None; max_id + 1];

        for id in ids {
            if let Some(def) = registry.get(id) {
                traits[id as usize] = Some(compute_traits(def));
            }
        }

        Self { traits }
    }

    /// Look up precomputed traits by archetype ID. O(1) access.
    #[inline]
    pub fn get(&self, id: ArchetypeId) -> Option<&ArchetypeTraits> {
        self.traits.get(id as usize).and_then(|opt| opt.as_ref())
    }

    /// Number of cached entries (non-None slots).
    pub fn len(&self) -> usize {
        self.traits.iter().filter(|t| t.is_some()).count()
    }

    /// Returns `true` if no archetypes are cached.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ─── Helper Functions ───────────────────────────────────────────────────────

/// Compute `CategoryFlags` from an `ArchetypeTag` list.
///
/// Maps each recognized tag to its corresponding category bit.
/// Tags that are not categories (e.g., density modifiers) are ignored.
fn tags_to_category(tags: &[ArchetypeTag]) -> CategoryFlags {
    let mut flags = CategoryFlags::NONE;
    for tag in tags {
        let bit = match tag {
            ArchetypeTag::Residential => CategoryFlags::RESIDENTIAL,
            ArchetypeTag::Commercial => CategoryFlags::COMMERCIAL,
            ArchetypeTag::Industrial => CategoryFlags::INDUSTRIAL,
            ArchetypeTag::Civic => CategoryFlags::CIVIC,
            ArchetypeTag::Utility => CategoryFlags::UTILITY,
            ArchetypeTag::Transport => CategoryFlags::TRANSPORT,
            // Service, density, and infrastructure tags are not category flags
            ArchetypeTag::Service
            | ArchetypeTag::LowDensity
            | ArchetypeTag::MediumDensity
            | ArchetypeTag::HighDensity
            | ArchetypeTag::PowerLine => CategoryFlags::NONE,
        };
        flags = flags.union(bit);
    }
    flags
}

/// Compute `CapacityClass` from an archetype definition.
///
/// Uses the combined resident + job capacity to classify.
fn compute_capacity_class(def: &ArchetypeDefinition) -> CapacityClass {
    let total = def.resident_capacity() + def.job_capacity();
    match total {
        0 => CapacityClass::None,
        1..=10 => CapacityClass::Small,
        11..=50 => CapacityClass::Medium,
        _ => CapacityClass::Large,
    }
}

/// Compute `ArchetypeTraits` from an `ArchetypeDefinition`.
fn compute_traits(def: &ArchetypeDefinition) -> ArchetypeTraits {
    let pollution_level = match def.pollution {
        0 => 0,
        1..=3 => 1,
        4..=6 => 2,
        _ => 3,
    };

    ArchetypeTraits {
        id: def.id,
        category: tags_to_category(&def.tags),
        capacity_class: compute_capacity_class(def),
        provides_power: def.power_supply_kw > 0,
        provides_water: def.water_supply > 0,
        is_employer: def.job_capacity() > 0,
        pollution_level,
        service_provider: def.has_tag(ArchetypeTag::Service) || def.service_radius > 0,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag, Prerequisite};

    /// Helper: create a residential archetype (small house).
    fn make_residential() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 1,
            name: "Small House".to_string(),
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
            prerequisites: vec![Prerequisite::RoadAccess, Prerequisite::PowerConnection],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 40,
        }
    }

    /// Helper: create a power plant archetype.
    fn make_power_plant() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 2,
            name: "Power Plant".to_string(),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 3,
            footprint_h: 3,
            coverage_ratio_pct: 60,
            floors: 2,
            usable_ratio_pct: 70,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 50,
            power_demand_kw: 0,
            power_supply_kw: 5000,
            water_demand: 10,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 5,
            desirability_magnitude: -20,
            pollution: 8,
            noise: 6,
            build_time_ticks: 2000,
            max_level: 5,
            prerequisites: vec![Prerequisite::RoadAccess],
            workspace_per_job_m2: 50,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create an industrial factory archetype.
    fn make_factory() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 3,
            name: "Factory".to_string(),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 70,
            floors: 2,
            usable_ratio_pct: 75,
            base_cost_cents: 200_000,
            base_upkeep_cents_per_tick: 30,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 5,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 4,
            desirability_magnitude: -15,
            pollution: 5,
            noise: 7,
            build_time_ticks: 1000,
            max_level: 3,
            prerequisites: vec![Prerequisite::RoadAccess, Prerequisite::PowerConnection],
            workspace_per_job_m2: 20,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create a small commercial shop.
    fn make_shop() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 4,
            name: "Shop".to_string(),
            tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 70,
            floors: 1,
            usable_ratio_pct: 85,
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
            prerequisites: vec![Prerequisite::RoadAccess],
            workspace_per_job_m2: 25,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create a water tower (utility, provides water).
    fn make_water_tower() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 5,
            name: "Water Tower".to_string(),
            tags: vec![ArchetypeTag::Utility],
            footprint_w: 1,
            footprint_h: 1,
            coverage_ratio_pct: 40,
            floors: 1,
            usable_ratio_pct: 90,
            base_cost_cents: 50_000,
            base_upkeep_cents_per_tick: 5,
            power_demand_kw: 2,
            power_supply_kw: 0,
            water_demand: 0,
            water_supply: 100,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 1,
            desirability_magnitude: -2,
            pollution: 0,
            noise: 0,
            build_time_ticks: 200,
            max_level: 2,
            prerequisites: vec![Prerequisite::RoadAccess],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create a hospital (civic + service).
    fn make_hospital() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 6,
            name: "Hospital".to_string(),
            tags: vec![ArchetypeTag::Civic, ArchetypeTag::Service],
            footprint_w: 3,
            footprint_h: 3,
            coverage_ratio_pct: 50,
            floors: 4,
            usable_ratio_pct: 70,
            base_cost_cents: 1_000_000,
            base_upkeep_cents_per_tick: 100,
            power_demand_kw: 100,
            power_supply_kw: 0,
            water_demand: 20,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 15,
            desirability_radius: 5,
            desirability_magnitude: 10,
            pollution: 0,
            noise: 3,
            build_time_ticks: 3000,
            max_level: 5,
            prerequisites: vec![
                Prerequisite::RoadAccess,
                Prerequisite::PowerConnection,
                Prerequisite::WaterConnection,
            ],
            workspace_per_job_m2: 30,
            living_space_per_person_m2: 0,
        }
    }

    /// Helper: create a large apartment building for Large capacity class.
    fn make_large_apartment() -> ArchetypeDefinition {
        ArchetypeDefinition {
            id: 7,
            name: "Large Apartment".to_string(),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::HighDensity],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 70,
            floors: 6,
            usable_ratio_pct: 75,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 40,
            power_demand_kw: 30,
            power_supply_kw: 0,
            water_demand: 15,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 3,
            desirability_magnitude: 2,
            pollution: 0,
            noise: 3,
            build_time_ticks: 1500,
            max_level: 5,
            prerequisites: vec![Prerequisite::RoadAccess, Prerequisite::PowerConnection],
            workspace_per_job_m2: 0,
            living_space_per_person_m2: 30,
        }
    }

    // ─── Residential Archetype -> RESIDENTIAL Flag ──────────────────────────

    #[test]
    fn residential_archetype_has_residential_flag() {
        let def = make_residential();
        let traits = compute_traits(&def);
        assert!(traits.category.contains(CategoryFlags::RESIDENTIAL));
        assert!(!traits.category.contains(CategoryFlags::COMMERCIAL));
        assert!(!traits.category.contains(CategoryFlags::INDUSTRIAL));
    }

    // ─── Power Plant -> provides_power = true ───────────────────────────────

    #[test]
    fn power_plant_provides_power() {
        let def = make_power_plant();
        let traits = compute_traits(&def);
        assert!(traits.provides_power);
        assert!(!traits.provides_water);
        assert!(traits.category.contains(CategoryFlags::UTILITY));
    }

    // ─── Industrial -> is_employer = true, pollution_level > 0 ──────────────

    #[test]
    fn industrial_is_employer_with_pollution() {
        let def = make_factory();
        let traits = compute_traits(&def);
        assert!(traits.is_employer);
        assert!(traits.pollution_level > 0);
        assert!(traits.category.contains(CategoryFlags::INDUSTRIAL));
    }

    // ─── CategoryFlags contains works correctly ─────────────────────────────

    #[test]
    fn category_flags_contains_single() {
        let flags = CategoryFlags::RESIDENTIAL;
        assert!(flags.contains(CategoryFlags::RESIDENTIAL));
        assert!(!flags.contains(CategoryFlags::COMMERCIAL));
    }

    #[test]
    fn category_flags_contains_combined() {
        let flags = CategoryFlags::RESIDENTIAL | CategoryFlags::COMMERCIAL;
        assert!(flags.contains(CategoryFlags::RESIDENTIAL));
        assert!(flags.contains(CategoryFlags::COMMERCIAL));
        assert!(!flags.contains(CategoryFlags::INDUSTRIAL));
    }

    #[test]
    fn category_flags_contains_subset() {
        let flags = CategoryFlags::RESIDENTIAL | CategoryFlags::COMMERCIAL | CategoryFlags::CIVIC;
        let subset = CategoryFlags::RESIDENTIAL | CategoryFlags::COMMERCIAL;
        assert!(flags.contains(subset));
    }

    #[test]
    fn category_flags_none_is_empty() {
        assert!(CategoryFlags::NONE.is_empty());
        assert!(!CategoryFlags::RESIDENTIAL.is_empty());
    }

    #[test]
    fn category_flags_contains_none() {
        // NONE (0) is always contained in any flag set.
        let flags = CategoryFlags::RESIDENTIAL;
        assert!(flags.contains(CategoryFlags::NONE));
    }

    // ─── CapacityClass Computation ──────────────────────────────────────────

    #[test]
    fn capacity_class_small() {
        let def = make_residential();
        // Small house: 1x1, 50% coverage, 2 floors -> 256 m2
        // resident_capacity = 256 / 40 = 6 (Small)
        let class = compute_capacity_class(&def);
        assert_eq!(class, CapacityClass::Small);
    }

    #[test]
    fn capacity_class_medium() {
        let def = make_shop();
        // Shop: 1x1, 70% coverage, 1 floor -> 179 m2
        // job_capacity = 179 / 25 = 7 (Small)
        // Actually let's verify:
        let cap = def.job_capacity();
        let class = compute_capacity_class(&def);
        assert!(cap >= 1 && cap <= 10, "cap={}", cap);
        assert_eq!(class, CapacityClass::Small);
    }

    #[test]
    fn capacity_class_large() {
        let def = make_power_plant();
        // Power plant: 3x3, 60% coverage, 2 floors -> 2764 m2
        // job_capacity = 2764 / 50 = 55 (Large)
        let class = compute_capacity_class(&def);
        assert_eq!(class, CapacityClass::Large);
    }

    #[test]
    fn capacity_class_none() {
        let def = make_water_tower();
        // Water tower: 1x1, 40% coverage, 1 floor -> 102 m2
        // No living_space_per_person, no workspace_per_job -> 0 capacity
        let class = compute_capacity_class(&def);
        assert_eq!(class, CapacityClass::None);
    }

    #[test]
    fn capacity_class_medium_range() {
        // We need 11-50 total capacity for Medium.
        // A small apartment: 1x1, 60% coverage, 3 floors = 256*60*3/100 = 460
        // 460 / 30 living_space = 15 -> Medium
        let mut medium_apt = make_residential();
        medium_apt.id = 99;
        medium_apt.coverage_ratio_pct = 60;
        medium_apt.floors = 3;
        medium_apt.living_space_per_person_m2 = 30;
        let cap = medium_apt.resident_capacity();
        assert!(cap >= 11 && cap <= 50, "cap={}", cap);
        let class = compute_capacity_class(&medium_apt);
        assert_eq!(class, CapacityClass::Medium);
    }

    #[test]
    fn capacity_class_large_apartment() {
        let def = make_large_apartment();
        // 2x2 = 1024 m2, 70% coverage, 6 floors = 1024*70*6/100 = 4300
        // 4300 / 30 = 143 -> Large
        let class = compute_capacity_class(&def);
        assert_eq!(class, CapacityClass::Large);
    }

    // ─── Cache built from registry, O(1) lookup ─────────────────────────────

    #[test]
    fn cache_from_registry_lookup() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_residential());
        reg.register(make_power_plant());
        reg.register(make_factory());

        let cache = ArchetypeTraitCache::from_registry(&reg);
        assert_eq!(cache.len(), 3);
        assert!(!cache.is_empty());

        // O(1) lookup
        let t1 = cache.get(1).unwrap();
        assert_eq!(t1.id, 1);
        assert!(t1.category.contains(CategoryFlags::RESIDENTIAL));

        let t2 = cache.get(2).unwrap();
        assert_eq!(t2.id, 2);
        assert!(t2.provides_power);

        let t3 = cache.get(3).unwrap();
        assert_eq!(t3.id, 3);
        assert!(t3.is_employer);
        assert!(t3.category.contains(CategoryFlags::INDUSTRIAL));
    }

    // ─── Unknown ID returns None ────────────────────────────────────────────

    #[test]
    fn cache_unknown_id_returns_none() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_residential());

        let cache = ArchetypeTraitCache::from_registry(&reg);
        assert!(cache.get(999).is_none());
        assert!(cache.get(0).is_none());
    }

    // ─── Empty registry -> empty cache ──────────────────────────────────────

    #[test]
    fn empty_registry_produces_empty_cache() {
        let reg = ArchetypeRegistry::new();
        let cache = ArchetypeTraitCache::from_registry(&reg);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.get(0).is_none());
        assert!(cache.get(1).is_none());
    }

    // ─── Additional Edge Cases ──────────────────────────────────────────────

    #[test]
    fn water_tower_provides_water() {
        let def = make_water_tower();
        let traits = compute_traits(&def);
        assert!(traits.provides_water);
        assert!(!traits.provides_power);
    }

    #[test]
    fn hospital_is_service_provider() {
        let def = make_hospital();
        let traits = compute_traits(&def);
        assert!(traits.service_provider);
        assert!(traits.category.contains(CategoryFlags::CIVIC));
    }

    #[test]
    fn pollution_level_none() {
        let def = make_residential();
        let traits = compute_traits(&def);
        assert_eq!(traits.pollution_level, 0);
    }

    #[test]
    fn pollution_level_medium() {
        let def = make_factory();
        // factory pollution = 5 -> medium (4-6)
        let traits = compute_traits(&def);
        assert_eq!(traits.pollution_level, 2);
    }

    #[test]
    fn pollution_level_high() {
        let def = make_power_plant();
        // power plant pollution = 8 -> high (7+)
        let traits = compute_traits(&def);
        assert_eq!(traits.pollution_level, 3);
    }

    #[test]
    fn tags_to_category_ignores_density_tags() {
        let tags = vec![ArchetypeTag::Residential, ArchetypeTag::HighDensity];
        let flags = tags_to_category(&tags);
        assert!(flags.contains(CategoryFlags::RESIDENTIAL));
        // HighDensity should not set any extra category bits
        assert!(!flags.contains(CategoryFlags::COMMERCIAL));
        assert!(!flags.contains(CategoryFlags::INDUSTRIAL));
    }

    #[test]
    fn tags_to_category_multiple_categories() {
        let tags = vec![ArchetypeTag::Civic, ArchetypeTag::Transport];
        let flags = tags_to_category(&tags);
        assert!(flags.contains(CategoryFlags::CIVIC));
        assert!(flags.contains(CategoryFlags::TRANSPORT));
    }

    #[test]
    fn residential_is_not_employer() {
        let def = make_residential();
        let traits = compute_traits(&def);
        assert!(!traits.is_employer);
    }

    #[test]
    fn cache_preserves_all_registered_archetypes() {
        let mut reg = ArchetypeRegistry::new();
        reg.register(make_residential());
        reg.register(make_power_plant());
        reg.register(make_factory());
        reg.register(make_shop());
        reg.register(make_water_tower());
        reg.register(make_hospital());

        let cache = ArchetypeTraitCache::from_registry(&reg);
        assert_eq!(cache.len(), 6);

        for id in 1..=6 {
            assert!(cache.get(id).is_some(), "id={} should be cached", id);
        }
    }
}
