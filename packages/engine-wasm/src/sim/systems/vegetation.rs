//! Vegetation and natural growth system.
//!
//! Models per-tile vegetation lifecycle: natural growth on unbuilt tiles,
//! clearing on construction, minimum floors in parks, and derived environmental
//! effects (noise reduction, pollution absorption, desirability bonus).

/// Trait for per-tile vegetation lifecycle.
pub trait IVegetationModel {
    /// Advance vegetation for one game-day.
    ///
    /// Mutates `vegetation` in place. Each slice must have the same length
    /// (one entry per tile).
    fn tick_vegetation(
        &self,
        vegetation: &mut [u8],
        near_water: &[bool],
        is_built: &[bool],
        is_park: &[bool],
    );

    /// Compute noise reduction factor from vegetation level.
    ///
    /// Returns a value in `[0.0, noise_factor]` representing the fraction of
    /// noise absorbed.
    fn noise_reduction(&self, vegetation_level: u8) -> f32;

    /// Compute pollution reduction factor from vegetation level.
    ///
    /// Returns a value in `[0.0, pollution_factor]` representing the fraction
    /// of pollution absorbed.
    fn pollution_reduction(&self, vegetation_level: u8) -> f32;

    /// Compute desirability bonus from vegetation level.
    fn desirability_bonus(&self, vegetation_level: u8) -> i16;
}

/// Default vegetation model with configurable parameters.
pub struct DefaultVegetationModel {
    /// Base growth per game-day on unbuilt, non-park tiles (default 1).
    pub growth_rate: u8,
    /// Extra growth per game-day for tiles near water (default 2).
    pub water_bonus: u8,
    /// Minimum vegetation level enforced on park tiles (default 128).
    pub park_floor: u8,
    /// Noise reduction fraction per 128 vegetation levels (default 0.10).
    pub noise_factor: f32,
    /// Pollution reduction fraction per 128 vegetation levels (default 0.15).
    pub pollution_factor: f32,
    /// Desirability bonus per 64 vegetation levels (default 5).
    pub desirability_per_64: i16,
}

impl Default for DefaultVegetationModel {
    fn default() -> Self {
        Self {
            growth_rate: 1,
            water_bonus: 2,
            park_floor: 128,
            noise_factor: 0.10,
            pollution_factor: 0.15,
            desirability_per_64: 5,
        }
    }
}

impl IVegetationModel for DefaultVegetationModel {
    fn tick_vegetation(
        &self,
        vegetation: &mut [u8],
        near_water: &[bool],
        is_built: &[bool],
        is_park: &[bool],
    ) {
        let len = vegetation.len();
        for i in 0..len {
            if is_built[i] {
                // Construction clears vegetation entirely.
                vegetation[i] = 0;
            } else if is_park[i] {
                // Parks grow naturally but enforce a minimum floor.
                let growth = self.growth_rate;
                let bonus = if near_water[i] { self.water_bonus } else { 0 };
                let total_growth = (growth as u16).saturating_add(bonus as u16);
                let new_val = (vegetation[i] as u16).saturating_add(total_growth).min(255) as u8;
                vegetation[i] = new_val.max(self.park_floor);
            } else {
                // Unbuilt, non-park: natural growth.
                let growth = self.growth_rate;
                let bonus = if near_water[i] { self.water_bonus } else { 0 };
                let total_growth = (growth as u16).saturating_add(bonus as u16);
                let new_val = (vegetation[i] as u16).saturating_add(total_growth).min(255) as u8;
                vegetation[i] = new_val;
            }
        }
    }

    fn noise_reduction(&self, vegetation_level: u8) -> f32 {
        let raw = (vegetation_level as f32 / 128.0) * self.noise_factor;
        raw.min(self.noise_factor)
    }

    fn pollution_reduction(&self, vegetation_level: u8) -> f32 {
        let raw = (vegetation_level as f32 / 128.0) * self.pollution_factor;
        raw.min(self.pollution_factor)
    }

    fn desirability_bonus(&self, vegetation_level: u8) -> i16 {
        (vegetation_level as i16 / 64) * self.desirability_per_64
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_model() -> DefaultVegetationModel {
        DefaultVegetationModel::default()
    }

    // ---- 1. Unbuilt tile grows by growth_rate ----
    #[test]
    fn unbuilt_tile_grows_by_growth_rate() {
        let model = default_model();
        let mut veg = vec![0u8];
        let near_water = vec![false];
        let is_built = vec![false];
        let is_park = vec![false];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        assert_eq!(veg[0], 1, "should grow by growth_rate=1");
    }

    // ---- 2. Near-water tile grows faster ----
    #[test]
    fn near_water_tile_grows_faster() {
        let model = default_model();
        let mut veg = vec![0u8];
        let near_water = vec![true];
        let is_built = vec![false];
        let is_park = vec![false];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        // growth_rate(1) + water_bonus(2) = 3
        assert_eq!(veg[0], 3, "should grow by growth_rate + water_bonus = 3");
    }

    // ---- 3. Built tile vegetation stays at 0 ----
    #[test]
    fn built_tile_vegetation_stays_at_zero() {
        let model = default_model();
        let mut veg = vec![100u8];
        let near_water = vec![false];
        let is_built = vec![true];
        let is_park = vec![false];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        assert_eq!(veg[0], 0, "built tile should be cleared to 0");
    }

    // ---- 4. Park tile has minimum vegetation floor ----
    #[test]
    fn park_tile_has_minimum_vegetation_floor() {
        let model = default_model();
        let mut veg = vec![0u8];
        let near_water = vec![false];
        let is_built = vec![false];
        let is_park = vec![true];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        assert_eq!(
            veg[0], 128,
            "park tile should be raised to park_floor=128"
        );
    }

    // ---- 5. Vegetation caps at 255 ----
    #[test]
    fn vegetation_caps_at_255() {
        let model = default_model();
        let mut veg = vec![254u8];
        let near_water = vec![true];
        let is_built = vec![false];
        let is_park = vec![false];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        // 254 + 3 = 257 => capped at 255
        assert_eq!(veg[0], 255, "should cap at 255");
    }

    // ---- 6. Construction clears vegetation ----
    #[test]
    fn construction_clears_vegetation() {
        let model = default_model();
        let mut veg = vec![200u8];
        let near_water = vec![true];
        let is_built = vec![true];
        let is_park = vec![false];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        assert_eq!(veg[0], 0, "construction should clear vegetation to 0");
    }

    // ---- 7. Noise reduction at 0 vegetation is 0 ----
    #[test]
    fn noise_reduction_at_zero_vegetation() {
        let model = default_model();
        let nr = model.noise_reduction(0);
        assert!((nr - 0.0).abs() < f32::EPSILON, "no vegetation => no noise reduction");
    }

    // ---- 8. Noise reduction at 128 vegetation ----
    #[test]
    fn noise_reduction_at_128_vegetation() {
        let model = default_model();
        let nr = model.noise_reduction(128);
        // (128/128) * 0.10 = 0.10
        assert!(
            (nr - 0.10).abs() < f32::EPSILON,
            "128 veg should give full noise_factor=0.10, got {}",
            nr
        );
    }

    // ---- 9. Noise reduction at 255 vegetation (capped) ----
    #[test]
    fn noise_reduction_at_255_vegetation_capped() {
        let model = default_model();
        let nr = model.noise_reduction(255);
        // (255/128) * 0.10 = ~0.199, but capped at noise_factor=0.10
        assert!(
            (nr - 0.10).abs() < f32::EPSILON,
            "255 veg should be capped at noise_factor=0.10, got {}",
            nr
        );
    }

    // ---- 10. Pollution reduction at various levels ----
    #[test]
    fn pollution_reduction_at_various_levels() {
        let model = default_model();

        let pr_0 = model.pollution_reduction(0);
        assert!((pr_0 - 0.0).abs() < f32::EPSILON);

        let pr_64 = model.pollution_reduction(64);
        // (64/128) * 0.15 = 0.075
        assert!(
            (pr_64 - 0.075).abs() < 0.001,
            "64 veg should give 0.075, got {}",
            pr_64
        );

        let pr_128 = model.pollution_reduction(128);
        // (128/128) * 0.15 = 0.15
        assert!(
            (pr_128 - 0.15).abs() < f32::EPSILON,
            "128 veg should give 0.15, got {}",
            pr_128
        );

        let pr_255 = model.pollution_reduction(255);
        // capped at 0.15
        assert!(
            (pr_255 - 0.15).abs() < f32::EPSILON,
            "255 veg should be capped at 0.15, got {}",
            pr_255
        );
    }

    // ---- 11. Desirability bonus at various levels ----
    #[test]
    fn desirability_bonus_at_various_levels() {
        let model = default_model();

        assert_eq!(model.desirability_bonus(0), 0);
        assert_eq!(model.desirability_bonus(63), 0);   // 63/64 = 0
        assert_eq!(model.desirability_bonus(64), 5);   // 64/64 = 1 * 5
        assert_eq!(model.desirability_bonus(128), 10);  // 128/64 = 2 * 5
        assert_eq!(model.desirability_bonus(192), 15);  // 192/64 = 3 * 5
        assert_eq!(model.desirability_bonus(255), 15);  // 255/64 = 3 * 5 (integer division: 3)
    }

    // ---- 12. Default model parameters are reasonable ----
    #[test]
    fn default_model_parameters_are_reasonable() {
        let model = default_model();
        assert_eq!(model.growth_rate, 1);
        assert_eq!(model.water_bonus, 2);
        assert_eq!(model.park_floor, 128);
        assert!((model.noise_factor - 0.10).abs() < f32::EPSILON);
        assert!((model.pollution_factor - 0.15).abs() < f32::EPSILON);
        assert_eq!(model.desirability_per_64, 5);
    }

    // ---- 13. Multiple ticks accumulate growth ----
    #[test]
    fn multiple_ticks_accumulate_growth() {
        let model = default_model();
        let mut veg = vec![0u8];
        let near_water = vec![false];
        let is_built = vec![false];
        let is_park = vec![false];

        for _ in 0..10 {
            model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        }
        // 10 ticks * growth_rate(1) = 10
        assert_eq!(veg[0], 10, "10 ticks should give vegetation=10");
    }

    // ---- 14. Mixed tile types in single tick ----
    #[test]
    fn mixed_tile_types_in_single_tick() {
        let model = default_model();
        //               unbuilt, near_water, built, park
        let mut veg =     vec![50,  50,         200,   10];
        let near_water =  vec![false, true,     false, false];
        let is_built =    vec![false, false,    true,  false];
        let is_park =     vec![false, false,    false, true];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);

        assert_eq!(veg[0], 51, "unbuilt: 50 + 1 = 51");
        assert_eq!(veg[1], 53, "near water: 50 + 1 + 2 = 53");
        assert_eq!(veg[2], 0, "built: cleared to 0");
        assert_eq!(veg[3], 128, "park: max(10+1, 128) = 128");
    }

    // ---- 15. Growth rate of 0 means no growth ----
    #[test]
    fn growth_rate_zero_means_no_growth() {
        let model = DefaultVegetationModel {
            growth_rate: 0,
            water_bonus: 0,
            ..Default::default()
        };
        let mut veg = vec![50u8];
        let near_water = vec![false];
        let is_built = vec![false];
        let is_park = vec![false];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        assert_eq!(veg[0], 50, "zero growth rate should not change vegetation");
    }

    // ---- 16. Park tile already above floor stays above ----
    #[test]
    fn park_tile_above_floor_stays_above() {
        let model = default_model();
        let mut veg = vec![200u8];
        let near_water = vec![false];
        let is_built = vec![false];
        let is_park = vec![true];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        // 200 + 1 = 201, max(201, 128) = 201
        assert_eq!(veg[0], 201, "park tile above floor should keep growing");
    }

    // ---- 17. Built overrides park ----
    #[test]
    fn built_overrides_park() {
        let model = default_model();
        let mut veg = vec![200u8];
        let near_water = vec![false];
        let is_built = vec![true];
        let is_park = vec![true]; // both built and park

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        // Built takes priority: vegetation cleared to 0
        assert_eq!(veg[0], 0, "built should override park flag");
    }

    // ---- 18. Custom model parameters work correctly ----
    #[test]
    fn custom_model_parameters() {
        let model = DefaultVegetationModel {
            growth_rate: 5,
            water_bonus: 10,
            park_floor: 200,
            noise_factor: 0.20,
            pollution_factor: 0.30,
            desirability_per_64: 10,
        };

        let mut veg = vec![0u8];
        let near_water = vec![true];
        let is_built = vec![false];
        let is_park = vec![false];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        // 0 + 5 + 10 = 15
        assert_eq!(veg[0], 15);

        assert!((model.noise_reduction(128) - 0.20).abs() < f32::EPSILON);
        assert!((model.pollution_reduction(128) - 0.30).abs() < f32::EPSILON);
        assert_eq!(model.desirability_bonus(128), 20); // (128/64) * 10
    }

    // ---- 19. Park near water grows and respects floor ----
    #[test]
    fn park_near_water_grows_and_respects_floor() {
        let model = default_model();
        let mut veg = vec![126u8];
        let near_water = vec![true];
        let is_built = vec![false];
        let is_park = vec![true];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        // 126 + 1 + 2 = 129, max(129, 128) = 129
        assert_eq!(veg[0], 129, "park near water should grow past floor");
    }

    // ---- 20. Empty slices do not panic ----
    #[test]
    fn empty_slices_do_not_panic() {
        let model = default_model();
        let mut veg: Vec<u8> = vec![];
        let near_water: Vec<bool> = vec![];
        let is_built: Vec<bool> = vec![];
        let is_park: Vec<bool> = vec![];

        model.tick_vegetation(&mut veg, &near_water, &is_built, &is_park);
        assert_eq!(veg.len(), 0);
    }
}
