//! Wealth-density growth matrix: independent wealth x density axes.

/// Wealth tier for a zone tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WealthTier {
    Low = 0,
    Medium = 1,
    High = 2,
}

/// Density class for a zone tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityClass {
    Low = 0,
    Medium = 1,
    High = 2,
}

/// Inputs for wealth-density evaluation.
#[derive(Debug, Clone)]
pub struct GrowthContext {
    pub land_value: u16,         // 0-65535
    pub service_coverage: u16,   // 0-65535 (ratio)
    pub pollution: u16,          // 0-65535
    pub crime: u16,              // 0-65535
    pub transit_access: u16,     // 0-65535
    pub zoning_density_cap: DensityClass,
}

/// Result of wealth-density evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrowthResult {
    pub wealth: WealthTier,
    pub density: DensityClass,
    pub growth_allowed: bool,
}

/// Trait for pluggable wealth-density models.
pub trait IWealthDensityModel {
    fn evaluate(&self, context: &GrowthContext) -> GrowthResult;
    fn name(&self) -> &str;
}

/// Default 3x3 matrix model.
pub struct DefaultWealthDensityModel {
    /// Land value thresholds for wealth tiers [low_to_med, med_to_high]
    pub wealth_thresholds: [u16; 2],
    /// Service + transit thresholds for density [low_to_med, med_to_high]
    pub density_thresholds: [u16; 2],
    /// Pollution threshold above which growth is blocked
    pub pollution_cap: u16,
    /// Crime threshold above which wealth degrades
    pub crime_cap: u16,
}

impl Default for DefaultWealthDensityModel {
    fn default() -> Self {
        Self {
            wealth_thresholds: [20_000, 45_000],
            density_thresholds: [15_000, 40_000],
            pollution_cap: 50_000,
            crime_cap: 40_000,
        }
    }
}

impl IWealthDensityModel for DefaultWealthDensityModel {
    fn evaluate(&self, ctx: &GrowthContext) -> GrowthResult {
        // Growth blocked by high pollution
        if ctx.pollution > self.pollution_cap {
            return GrowthResult {
                wealth: WealthTier::Low,
                density: DensityClass::Low,
                growth_allowed: false,
            };
        }

        // Determine wealth from land value, degraded by crime
        let effective_land_value = if ctx.crime > self.crime_cap {
            ctx.land_value.saturating_sub(ctx.crime / 2)
        } else {
            ctx.land_value
        };

        let wealth = if effective_land_value >= self.wealth_thresholds[1] {
            WealthTier::High
        } else if effective_land_value >= self.wealth_thresholds[0] {
            WealthTier::Medium
        } else {
            WealthTier::Low
        };

        // Determine density from service + transit access, capped by zoning
        let access_score = (ctx.service_coverage as u32 + ctx.transit_access as u32) / 2;
        let uncapped_density = if access_score >= self.density_thresholds[1] as u32 {
            DensityClass::High
        } else if access_score >= self.density_thresholds[0] as u32 {
            DensityClass::Medium
        } else {
            DensityClass::Low
        };

        // Cap by zoning
        let density = if (uncapped_density as u8) > (ctx.zoning_density_cap as u8) {
            ctx.zoning_density_cap
        } else {
            uncapped_density
        };

        GrowthResult {
            wealth,
            density,
            growth_allowed: true,
        }
    }

    fn name(&self) -> &str {
        "default_3x3"
    }
}

/// Get candidate archetype set for a wealth-density cell.
pub fn archetype_candidates(wealth: WealthTier, density: DensityClass) -> (u16, u16) {
    // Returns (min_archetype_id, max_archetype_id) range
    // This is a lookup table for the 3x3 matrix
    let base = (wealth as u16) * 3 + (density as u16);
    (base * 100 + 100, base * 100 + 199)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context(land_value: u16, service_coverage: u16, transit_access: u16) -> GrowthContext {
        GrowthContext {
            land_value,
            service_coverage,
            pollution: 0,
            crime: 0,
            transit_access,
            zoning_density_cap: DensityClass::High,
        }
    }

    // ─── Test 1: Low land value yields WealthTier::Low ──────────────────────

    #[test]
    fn low_land_value_yields_low_wealth() {
        let model = DefaultWealthDensityModel::default();
        let ctx = make_context(5_000, 30_000, 30_000);
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::Low);
        assert!(result.growth_allowed);
    }

    // ─── Test 2: High land value yields WealthTier::High ────────────────────

    #[test]
    fn high_land_value_yields_high_wealth() {
        let model = DefaultWealthDensityModel::default();
        let ctx = make_context(50_000, 30_000, 30_000);
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::High);
    }

    // ─── Test 3: Medium land value yields WealthTier::Medium ────────────────

    #[test]
    fn medium_land_value_yields_medium_wealth() {
        let model = DefaultWealthDensityModel::default();
        let ctx = make_context(30_000, 30_000, 30_000);
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::Medium);
    }

    // ─── Test 4: Crime degrades wealth tier ──────────────────────────────────

    #[test]
    fn crime_degrades_wealth_tier() {
        let model = DefaultWealthDensityModel::default();
        // land_value = 46000 would normally be High (>= 45000)
        // With crime = 50000 (> cap 40000), effective = 46000 - 50000/2 = 46000 - 25000 = 21000 => Medium
        let ctx = GrowthContext {
            land_value: 46_000,
            service_coverage: 30_000,
            pollution: 0,
            crime: 50_000,
            transit_access: 30_000,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::Medium);
        assert!(result.growth_allowed);
    }

    // ─── Test 5: High pollution blocks growth ───────────────────────────────

    #[test]
    fn high_pollution_blocks_growth() {
        let model = DefaultWealthDensityModel::default();
        let ctx = GrowthContext {
            land_value: 60_000,
            service_coverage: 60_000,
            pollution: 55_000, // above cap of 50000
            crime: 0,
            transit_access: 60_000,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert!(!result.growth_allowed);
        assert_eq!(result.wealth, WealthTier::Low);
        assert_eq!(result.density, DensityClass::Low);
    }

    // ─── Test 6: Service + transit determines density class ─────────────────

    #[test]
    fn service_transit_determines_density() {
        let model = DefaultWealthDensityModel::default();

        // Low density: avg access < 15000
        let ctx_low = make_context(30_000, 5_000, 5_000);
        assert_eq!(model.evaluate(&ctx_low).density, DensityClass::Low);

        // Medium density: avg access >= 15000 and < 40000
        let ctx_med = make_context(30_000, 25_000, 25_000);
        assert_eq!(model.evaluate(&ctx_med).density, DensityClass::Medium);

        // High density: avg access >= 40000
        let ctx_high = make_context(30_000, 50_000, 50_000);
        assert_eq!(model.evaluate(&ctx_high).density, DensityClass::High);
    }

    // ─── Test 7: Zoning cap limits density ──────────────────────────────────

    #[test]
    fn zoning_cap_limits_density() {
        let model = DefaultWealthDensityModel::default();
        // High service/transit would give High density, but capped to Low
        let ctx = GrowthContext {
            land_value: 30_000,
            service_coverage: 50_000,
            pollution: 0,
            crime: 0,
            transit_access: 50_000,
            zoning_density_cap: DensityClass::Low,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.density, DensityClass::Low);
        assert!(result.growth_allowed);
    }

    // ─── Test 8: Zoning cap Medium caps High to Medium ──────────────────────

    #[test]
    fn zoning_cap_medium_caps_high_to_medium() {
        let model = DefaultWealthDensityModel::default();
        let ctx = GrowthContext {
            land_value: 30_000,
            service_coverage: 50_000,
            pollution: 0,
            crime: 0,
            transit_access: 50_000,
            zoning_density_cap: DensityClass::Medium,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.density, DensityClass::Medium);
    }

    // ─── Test 9: archetype_candidates returns valid ranges ──────────────────

    #[test]
    fn archetype_candidates_returns_valid_ranges() {
        let (min, max) = archetype_candidates(WealthTier::Low, DensityClass::Low);
        assert_eq!(min, 100);
        assert_eq!(max, 199);
        assert!(min <= max);

        let (min, max) = archetype_candidates(WealthTier::High, DensityClass::High);
        assert_eq!(min, 900);
        assert_eq!(max, 999);
        assert!(min <= max);
    }

    // ─── Test 10: Model name is correct ─────────────────────────────────────

    #[test]
    fn model_name_is_correct() {
        let model = DefaultWealthDensityModel::default();
        assert_eq!(model.name(), "default_3x3");
    }

    // ─── Test 11: GrowthResult equality ─────────────────────────────────────

    #[test]
    fn growth_result_equality() {
        let a = GrowthResult {
            wealth: WealthTier::Medium,
            density: DensityClass::High,
            growth_allowed: true,
        };
        let b = GrowthResult {
            wealth: WealthTier::Medium,
            density: DensityClass::High,
            growth_allowed: true,
        };
        let c = GrowthResult {
            wealth: WealthTier::Low,
            density: DensityClass::High,
            growth_allowed: true,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ─── Test 12: All 9 matrix cells reachable ──────────────────────────────

    #[test]
    fn all_nine_matrix_cells_reachable() {
        let model = DefaultWealthDensityModel::default();

        let wealth_values: [(u16, WealthTier); 3] = [
            (5_000, WealthTier::Low),
            (30_000, WealthTier::Medium),
            (50_000, WealthTier::High),
        ];
        let density_values: [(u16, DensityClass); 3] = [
            (5_000, DensityClass::Low),
            (25_000, DensityClass::Medium),
            (50_000, DensityClass::High),
        ];

        for (lv, expected_wealth) in &wealth_values {
            for (sv, expected_density) in &density_values {
                let ctx = GrowthContext {
                    land_value: *lv,
                    service_coverage: *sv,
                    pollution: 0,
                    crime: 0,
                    transit_access: *sv,
                    zoning_density_cap: DensityClass::High,
                };
                let result = model.evaluate(&ctx);
                assert_eq!(
                    result.wealth, *expected_wealth,
                    "land_value={} expected {:?} got {:?}",
                    lv, expected_wealth, result.wealth
                );
                assert_eq!(
                    result.density, *expected_density,
                    "service={} expected {:?} got {:?}",
                    sv, expected_density, result.density
                );
                assert!(result.growth_allowed);
            }
        }
    }

    // ─── Test 13: Edge case all zeros ───────────────────────────────────────

    #[test]
    fn edge_case_all_zeros() {
        let model = DefaultWealthDensityModel::default();
        let ctx = GrowthContext {
            land_value: 0,
            service_coverage: 0,
            pollution: 0,
            crime: 0,
            transit_access: 0,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::Low);
        assert_eq!(result.density, DensityClass::Low);
        assert!(result.growth_allowed);
    }

    // ─── Test 14: Edge case all max values ──────────────────────────────────

    #[test]
    fn edge_case_all_max_values() {
        let model = DefaultWealthDensityModel::default();
        // pollution = 65535 > cap so growth is blocked
        let ctx = GrowthContext {
            land_value: 65_535,
            service_coverage: 65_535,
            pollution: 65_535,
            crime: 65_535,
            transit_access: 65_535,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert!(!result.growth_allowed);
    }

    // ─── Test 15: All max but pollution below cap ───────────────────────────

    #[test]
    fn all_max_except_pollution_yields_high_high() {
        let model = DefaultWealthDensityModel::default();
        // Crime = 65535 > cap, so effective_land_value = 65535 - 65535/2 = 65535 - 32767 = 32768 => Medium
        // With crime degradation, we need to carefully check
        let ctx = GrowthContext {
            land_value: 65_535,
            service_coverage: 65_535,
            pollution: 0,
            crime: 0,
            transit_access: 65_535,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::High);
        assert_eq!(result.density, DensityClass::High);
        assert!(result.growth_allowed);
    }

    // ─── Test 16: archetype_candidates covers all 9 cells uniquely ──────────

    #[test]
    fn archetype_candidates_all_cells_unique() {
        let mut ranges = Vec::new();
        let wealth_tiers = [WealthTier::Low, WealthTier::Medium, WealthTier::High];
        let density_classes = [DensityClass::Low, DensityClass::Medium, DensityClass::High];

        for w in &wealth_tiers {
            for d in &density_classes {
                let (min, max) = archetype_candidates(*w, *d);
                // Each range spans 100 IDs
                assert_eq!(max - min, 99);
                ranges.push((min, max));
            }
        }

        // Verify no overlap between ranges
        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                assert!(
                    ranges[i].1 < ranges[j].0 || ranges[j].1 < ranges[i].0,
                    "Ranges {:?} and {:?} overlap",
                    ranges[i],
                    ranges[j]
                );
            }
        }
    }

    // ─── Test 17: Pollution exactly at cap allows growth ────────────────────

    #[test]
    fn pollution_at_cap_allows_growth() {
        let model = DefaultWealthDensityModel::default();
        let ctx = GrowthContext {
            land_value: 30_000,
            service_coverage: 30_000,
            pollution: 50_000, // exactly at cap, not above
            crime: 0,
            transit_access: 30_000,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert!(result.growth_allowed);
    }

    // ─── Test 18: Crime at cap does not degrade wealth ──────────────────────

    #[test]
    fn crime_at_cap_does_not_degrade_wealth() {
        let model = DefaultWealthDensityModel::default();
        // crime = 40000 (exactly at cap, not above), so no degradation
        let ctx = GrowthContext {
            land_value: 45_000,
            service_coverage: 30_000,
            pollution: 0,
            crime: 40_000,
            transit_access: 30_000,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::High);
    }

    // ─── Test 19: Crime severely degrades to Low wealth ─────────────────────

    #[test]
    fn crime_severely_degrades_to_low_wealth() {
        let model = DefaultWealthDensityModel::default();
        // land_value = 25000, crime = 60000 (> cap)
        // effective = 25000 - 60000/2 = 25000 - 30000 = 0 (saturating) => Low
        let ctx = GrowthContext {
            land_value: 25_000,
            service_coverage: 30_000,
            pollution: 0,
            crime: 60_000,
            transit_access: 30_000,
            zoning_density_cap: DensityClass::High,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.wealth, WealthTier::Low);
    }

    // ─── Test 20: Default model thresholds are correct ──────────────────────

    #[test]
    fn default_model_thresholds() {
        let model = DefaultWealthDensityModel::default();
        assert_eq!(model.wealth_thresholds, [20_000, 45_000]);
        assert_eq!(model.density_thresholds, [15_000, 40_000]);
        assert_eq!(model.pollution_cap, 50_000);
        assert_eq!(model.crime_cap, 40_000);
    }

    // ─── Test 21: Zoning cap does not affect lower densities ────────────────

    #[test]
    fn zoning_cap_does_not_affect_lower_densities() {
        let model = DefaultWealthDensityModel::default();
        // Low service -> Low density, cap is Medium -> Low stays Low (not capped)
        let ctx = GrowthContext {
            land_value: 30_000,
            service_coverage: 5_000,
            pollution: 0,
            crime: 0,
            transit_access: 5_000,
            zoning_density_cap: DensityClass::Medium,
        };
        let result = model.evaluate(&ctx);
        assert_eq!(result.density, DensityClass::Low);
    }
}
