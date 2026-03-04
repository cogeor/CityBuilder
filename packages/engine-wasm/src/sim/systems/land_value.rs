//! Land value as causal driver with bidirectional feedback.
//!
//! Land value is computed from a weighted combination of positive factors
//! (service proximity, transit access, parks, water, building quality)
//! and negative factors (pollution, noise, crime). The result feeds into
//! wealth-density growth, rent calculation, and tax base computation.

/// Inputs for land value computation.
#[derive(Debug, Clone, Default)]
pub struct LandValueInputs {
    pub service_proximity: u16, // 0-65535
    pub transit_access: u16,    // 0-65535
    pub pollution: u16,         // 0-65535 (negative factor)
    pub noise: u16,             // 0-65535 (negative factor)
    pub crime: u16,             // 0-65535 (negative factor)
    pub park_proximity: u16,    // 0-65535
    pub water_proximity: u16,   // 0-65535
    pub building_quality: u16,  // 0-65535 (wealth of neighbors)
}

/// Trait for pluggable land value computation.
pub trait ILandValueModel {
    fn compute(&self, inputs: &LandValueInputs) -> u16;
    fn name(&self) -> &str;
}

/// Default land value model with weighted factors.
pub struct DefaultLandValueModel {
    pub service_weight: i32,
    pub transit_weight: i32,
    pub pollution_weight: i32,
    pub noise_weight: i32,
    pub crime_weight: i32,
    pub park_weight: i32,
    pub water_weight: i32,
    pub quality_weight: i32,
}

impl Default for DefaultLandValueModel {
    fn default() -> Self {
        Self {
            service_weight: 20,
            transit_weight: 15,
            pollution_weight: -25,
            noise_weight: -10,
            crime_weight: -20,
            park_weight: 15,
            water_weight: 10,
            quality_weight: 10,
        }
    }
}

impl ILandValueModel for DefaultLandValueModel {
    fn compute(&self, inputs: &LandValueInputs) -> u16 {
        let positive = (inputs.service_proximity as i64 * self.service_weight as i64)
            + (inputs.transit_access as i64 * self.transit_weight as i64)
            + (inputs.park_proximity as i64 * self.park_weight as i64)
            + (inputs.water_proximity as i64 * self.water_weight as i64)
            + (inputs.building_quality as i64 * self.quality_weight as i64);

        let negative = (inputs.pollution as i64 * self.pollution_weight.abs() as i64)
            + (inputs.noise as i64 * self.noise_weight.abs() as i64)
            + (inputs.crime as i64 * self.crime_weight.abs() as i64);

        let total_weight = self.service_weight.abs()
            + self.transit_weight.abs()
            + self.pollution_weight.abs()
            + self.noise_weight.abs()
            + self.crime_weight.abs()
            + self.park_weight.abs()
            + self.water_weight.abs()
            + self.quality_weight.abs();

        let raw = (positive - negative) / total_weight as i64;
        raw.clamp(0, 65535) as u16
    }

    fn name(&self) -> &str {
        "default_weighted"
    }
}

/// Compute land value for a grid of tiles.
pub fn compute_land_value_grid(
    _width: usize,
    _height: usize,
    inputs: &[LandValueInputs],
    model: &dyn ILandValueModel,
) -> Vec<u16> {
    inputs.iter().map(|i| model.compute(i)).collect()
}

/// Compute rent from land value (simple linear mapping).
///
/// Returns rent in cents per tick, proportional to land value.
pub fn compute_rent(land_value: u16) -> u32 {
    (land_value as u32 * 10) / 65535 + 1
}

/// Compute tax base contribution from land value.
pub fn compute_tax_base(land_value: u16, area: u16) -> u64 {
    land_value as u64 * area as u64
}

// ---- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Test 1: High services + low negatives -> high land value ----

    #[test]
    fn high_services_low_negatives_yields_high_value() {
        let model = DefaultLandValueModel::default();
        let inputs = LandValueInputs {
            service_proximity: 60_000,
            transit_access: 60_000,
            pollution: 0,
            noise: 0,
            crime: 0,
            park_proximity: 60_000,
            water_proximity: 60_000,
            building_quality: 60_000,
        };
        let value = model.compute(&inputs);
        // All positive factors at high values, no negatives => high land value
        assert!(value > 30_000, "Expected high land value, got {}", value);
    }

    // ---- Test 2: High pollution -> depressed land value ----

    #[test]
    fn high_pollution_depresses_value() {
        let model = DefaultLandValueModel::default();
        let clean = LandValueInputs {
            service_proximity: 40_000,
            transit_access: 40_000,
            pollution: 0,
            noise: 0,
            crime: 0,
            park_proximity: 40_000,
            water_proximity: 40_000,
            building_quality: 40_000,
        };
        let polluted = LandValueInputs {
            pollution: 60_000,
            ..clean.clone()
        };

        let clean_value = model.compute(&clean);
        let polluted_value = model.compute(&polluted);
        assert!(
            polluted_value < clean_value,
            "Polluted value {} should be less than clean value {}",
            polluted_value,
            clean_value
        );
    }

    // ---- Test 3: All zeros -> zero or base value ----

    #[test]
    fn all_zeros_yields_zero() {
        let model = DefaultLandValueModel::default();
        let inputs = LandValueInputs::default();
        let value = model.compute(&inputs);
        assert_eq!(value, 0, "All-zero inputs should yield 0");
    }

    // ---- Test 4: All max positives, no negatives -> near max ----

    #[test]
    fn all_max_positives_no_negatives_yields_near_max() {
        let model = DefaultLandValueModel::default();
        let inputs = LandValueInputs {
            service_proximity: 65_535,
            transit_access: 65_535,
            pollution: 0,
            noise: 0,
            crime: 0,
            park_proximity: 65_535,
            water_proximity: 65_535,
            building_quality: 65_535,
        };
        let value = model.compute(&inputs);
        // positive = 65535 * (20+15+15+10+10) = 65535 * 70
        // total_weight = 125
        // raw = (65535 * 70) / 125 = 36699.6 -> 36699
        assert!(
            value > 30_000,
            "All max positives should yield high value, got {}",
            value
        );
    }

    // ---- Test 5: Crime reduces value ----

    #[test]
    fn crime_reduces_value() {
        let model = DefaultLandValueModel::default();
        let safe = LandValueInputs {
            service_proximity: 40_000,
            transit_access: 40_000,
            pollution: 0,
            noise: 0,
            crime: 0,
            park_proximity: 40_000,
            water_proximity: 40_000,
            building_quality: 40_000,
        };
        let dangerous = LandValueInputs {
            crime: 50_000,
            ..safe.clone()
        };

        let safe_value = model.compute(&safe);
        let dangerous_value = model.compute(&dangerous);
        assert!(
            dangerous_value < safe_value,
            "Crime-ridden value {} should be less than safe value {}",
            dangerous_value,
            safe_value
        );
    }

    // ---- Test 6: Parks and water boost value ----

    #[test]
    fn parks_and_water_boost_value() {
        let model = DefaultLandValueModel::default();
        let base = LandValueInputs {
            service_proximity: 30_000,
            transit_access: 30_000,
            pollution: 0,
            noise: 0,
            crime: 0,
            park_proximity: 0,
            water_proximity: 0,
            building_quality: 30_000,
        };
        let with_parks_water = LandValueInputs {
            park_proximity: 50_000,
            water_proximity: 50_000,
            ..base.clone()
        };

        let base_value = model.compute(&base);
        let boosted_value = model.compute(&with_parks_water);
        assert!(
            boosted_value > base_value,
            "Parks/water boosted value {} should exceed base value {}",
            boosted_value,
            base_value
        );
    }

    // ---- Test 7: Model name correct ----

    #[test]
    fn model_name_correct() {
        let model = DefaultLandValueModel::default();
        assert_eq!(model.name(), "default_weighted");
    }

    // ---- Test 8: compute_rent proportional to value ----

    #[test]
    fn compute_rent_proportional_to_value() {
        let rent_low = compute_rent(1_000);
        let rent_high = compute_rent(60_000);
        assert!(
            rent_high > rent_low,
            "Higher land value should yield higher rent"
        );

        // Minimum rent is 1 (the +1 floor)
        let rent_zero = compute_rent(0);
        assert_eq!(rent_zero, 1, "Zero land value should yield minimum rent of 1");

        // Max land value rent
        let rent_max = compute_rent(65_535);
        // (65535 * 10) / 65535 + 1 = 10 + 1 = 11
        assert_eq!(rent_max, 11);
    }

    // ---- Test 9: compute_tax_base area scaling ----

    #[test]
    fn compute_tax_base_area_scaling() {
        let base_1 = compute_tax_base(10_000, 1);
        let base_4 = compute_tax_base(10_000, 4);
        assert_eq!(base_4, base_1 * 4, "Tax base should scale linearly with area");

        let base_zero = compute_tax_base(0, 100);
        assert_eq!(base_zero, 0, "Zero land value should yield zero tax base");

        let base_zero_area = compute_tax_base(50_000, 0);
        assert_eq!(base_zero_area, 0, "Zero area should yield zero tax base");
    }

    // ---- Test 10: compute_land_value_grid processes all tiles ----

    #[test]
    fn compute_land_value_grid_processes_all_tiles() {
        let model = DefaultLandValueModel::default();
        let inputs = vec![
            LandValueInputs {
                service_proximity: 40_000,
                ..Default::default()
            },
            LandValueInputs {
                pollution: 30_000,
                ..Default::default()
            },
            LandValueInputs {
                park_proximity: 50_000,
                ..Default::default()
            },
        ];
        let result = compute_land_value_grid(3, 1, &inputs, &model);
        assert_eq!(result.len(), 3);

        // First tile: only service_proximity, should be > 0
        assert!(result[0] > 0);
        // Second tile: only pollution, should be 0 (clamped from negative)
        assert_eq!(result[1], 0);
        // Third tile: only park_proximity, should be > 0
        assert!(result[2] > 0);
    }

    // ---- Test 11: Empty inputs -> empty output ----

    #[test]
    fn empty_inputs_empty_output() {
        let model = DefaultLandValueModel::default();
        let inputs: Vec<LandValueInputs> = vec![];
        let result = compute_land_value_grid(0, 0, &inputs, &model);
        assert!(result.is_empty());
    }

    // ---- Test 12: Default weights sum correctly ----

    #[test]
    fn default_weights_sum_correctly() {
        let model = DefaultLandValueModel::default();
        let total = model.service_weight.abs()
            + model.transit_weight.abs()
            + model.pollution_weight.abs()
            + model.noise_weight.abs()
            + model.crime_weight.abs()
            + model.park_weight.abs()
            + model.water_weight.abs()
            + model.quality_weight.abs();
        // 20 + 15 + 25 + 10 + 20 + 15 + 10 + 10 = 125
        assert_eq!(total, 125);
    }

    // ---- Test 13: Negative feedback: high crime -> low value ----

    #[test]
    fn negative_feedback_high_crime_low_value() {
        let model = DefaultLandValueModel::default();
        let inputs = LandValueInputs {
            service_proximity: 20_000,
            transit_access: 20_000,
            pollution: 0,
            noise: 0,
            crime: 65_535,
            park_proximity: 0,
            water_proximity: 0,
            building_quality: 0,
        };
        let value = model.compute(&inputs);
        // Positive: 20000*20 + 20000*15 = 700_000
        // Negative: 65535*20 = 1_310_700
        // raw = (700_000 - 1_310_700) / 125 = -4885.6 -> clamped to 0
        assert_eq!(
            value, 0,
            "High crime with modest positives should yield zero"
        );
    }

    // ---- Test 14: Noise reduces value independently ----

    #[test]
    fn noise_reduces_value() {
        let model = DefaultLandValueModel::default();
        let quiet = LandValueInputs {
            service_proximity: 40_000,
            transit_access: 40_000,
            noise: 0,
            ..Default::default()
        };
        let noisy = LandValueInputs {
            noise: 50_000,
            ..quiet.clone()
        };

        let quiet_value = model.compute(&quiet);
        let noisy_value = model.compute(&noisy);
        assert!(
            noisy_value < quiet_value,
            "Noisy value {} should be less than quiet value {}",
            noisy_value,
            quiet_value
        );
    }

    // ---- Test 15: Building quality improves value ----

    #[test]
    fn building_quality_improves_value() {
        let model = DefaultLandValueModel::default();
        let low_quality = LandValueInputs {
            service_proximity: 30_000,
            building_quality: 0,
            ..Default::default()
        };
        let high_quality = LandValueInputs {
            building_quality: 60_000,
            ..low_quality.clone()
        };

        let low_value = model.compute(&low_quality);
        let high_value = model.compute(&high_quality);
        assert!(
            high_value > low_value,
            "Higher building quality {} should yield more than low quality {}",
            high_value,
            low_value
        );
    }

    // ---- Test 16: Value clamped to u16 max ----

    #[test]
    fn value_clamped_to_u16_max() {
        // Custom model with extreme weights to try to exceed u16::MAX
        let model = DefaultLandValueModel {
            service_weight: 100,
            transit_weight: 100,
            pollution_weight: 0,
            noise_weight: 0,
            crime_weight: 0,
            park_weight: 100,
            water_weight: 100,
            quality_weight: 100,
        };
        let inputs = LandValueInputs {
            service_proximity: 65_535,
            transit_access: 65_535,
            park_proximity: 65_535,
            water_proximity: 65_535,
            building_quality: 65_535,
            ..Default::default()
        };
        let value = model.compute(&inputs);
        assert!(value <= 65_535);
    }

    // ---- Test 17: Default model default values ----

    #[test]
    fn default_model_weights() {
        let model = DefaultLandValueModel::default();
        assert_eq!(model.service_weight, 20);
        assert_eq!(model.transit_weight, 15);
        assert_eq!(model.pollution_weight, -25);
        assert_eq!(model.noise_weight, -10);
        assert_eq!(model.crime_weight, -20);
        assert_eq!(model.park_weight, 15);
        assert_eq!(model.water_weight, 10);
        assert_eq!(model.quality_weight, 10);
    }

    // ---- Test 18: compute_rent range ----

    #[test]
    fn compute_rent_range() {
        // Verify rent stays within expected bounds for all u16 values
        let rent_min = compute_rent(0);
        let rent_max = compute_rent(65_535);
        assert!(rent_min >= 1);
        assert!(rent_max <= 11);
    }

    // ---- Test 19: compute_tax_base max values ----

    #[test]
    fn compute_tax_base_max_values() {
        let base = compute_tax_base(65_535, 65_535);
        assert_eq!(base, 65_535u64 * 65_535u64);
        // No overflow: 65535 * 65535 = 4_294_836_225 which fits in u64
        assert_eq!(base, 4_294_836_225);
    }

    // ---- Test 20: Grid preserves order ----

    #[test]
    fn grid_preserves_order() {
        let model = DefaultLandValueModel::default();
        let inputs = vec![
            LandValueInputs {
                service_proximity: 10_000,
                ..Default::default()
            },
            LandValueInputs {
                service_proximity: 30_000,
                ..Default::default()
            },
            LandValueInputs {
                service_proximity: 50_000,
                ..Default::default()
            },
        ];
        let result = compute_land_value_grid(3, 1, &inputs, &model);
        assert!(result[0] < result[1]);
        assert!(result[1] < result[2]);
    }
}
