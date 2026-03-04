//! Commute time -> happiness feedback model.

/// Commute happiness evaluation inputs.
#[derive(Debug, Clone)]
pub struct CommuteContext {
    pub commute_ticks: u32, // average commute time in sim ticks
    pub has_transit: bool,  // transit available reduces perceived commute
}

/// Result of commute happiness evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct HappinessModifier {
    pub value: i16, // -100 to +100, applied to desirability
    pub category: CommuteCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommuteCategory {
    Excellent, // < short threshold
    Good,      // short to medium
    Tolerable, // medium to long
    Poor,      // > long threshold
}

/// Trait for pluggable commute happiness evaluation.
pub trait ICommuteHappinessModel {
    fn evaluate(&self, context: &CommuteContext) -> HappinessModifier;
    fn name(&self) -> &str;
}

/// Default commute happiness model with threshold-based categories.
pub struct DefaultCommuteModel {
    /// Commute ticks below this = no penalty (Excellent)
    pub short_threshold: u32,
    /// Above this = moderate penalty (Poor start)
    pub medium_threshold: u32,
    /// Above this = severe penalty
    pub long_threshold: u32,
    /// Transit reduces effective commute by this percentage (0-100)
    pub transit_reduction_pct: u32,
    /// Max positive bonus for excellent commute
    pub max_bonus: i16,
    /// Max negative penalty for poor commute
    pub max_penalty: i16,
}

impl Default for DefaultCommuteModel {
    fn default() -> Self {
        Self {
            short_threshold: 50,   // ~5 minutes game time
            medium_threshold: 200, // ~20 minutes
            long_threshold: 500,   // ~50 minutes
            transit_reduction_pct: 30,
            max_bonus: 20,
            max_penalty: -50,
        }
    }
}

impl ICommuteHappinessModel for DefaultCommuteModel {
    fn evaluate(&self, ctx: &CommuteContext) -> HappinessModifier {
        let effective_commute = if ctx.has_transit {
            ctx.commute_ticks * (100 - self.transit_reduction_pct) / 100
        } else {
            ctx.commute_ticks
        };

        if effective_commute <= self.short_threshold {
            HappinessModifier {
                value: self.max_bonus,
                category: CommuteCategory::Excellent,
            }
        } else if effective_commute <= self.medium_threshold {
            // Linear interpolation from bonus to 0
            let range = self.medium_threshold - self.short_threshold;
            let progress = effective_commute - self.short_threshold;
            let value = self.max_bonus - (self.max_bonus as u32 * progress / range) as i16;
            HappinessModifier {
                value,
                category: CommuteCategory::Good,
            }
        } else if effective_commute <= self.long_threshold {
            // Linear interpolation from 0 to max_penalty
            let range = self.long_threshold - self.medium_threshold;
            let progress = effective_commute - self.medium_threshold;
            let value = -((self.max_penalty.unsigned_abs() as u32 * progress / range) as i16);
            HappinessModifier {
                value,
                category: CommuteCategory::Tolerable,
            }
        } else {
            HappinessModifier {
                value: self.max_penalty,
                category: CommuteCategory::Poor,
            }
        }
    }

    fn name(&self) -> &str {
        "default_threshold"
    }
}

/// Batch evaluate happiness for a set of residential zones.
pub fn batch_evaluate(
    commute_times: &[(u32, bool)], // (ticks, has_transit)
    model: &dyn ICommuteHappinessModel,
) -> Vec<HappinessModifier> {
    commute_times
        .iter()
        .map(|(ticks, transit)| {
            model.evaluate(&CommuteContext {
                commute_ticks: *ticks,
                has_transit: *transit,
            })
        })
        .collect()
}

/// Compute average happiness modifier from a batch.
pub fn average_modifier(modifiers: &[HappinessModifier]) -> i16 {
    if modifiers.is_empty() {
        return 0;
    }
    let sum: i32 = modifiers.iter().map(|m| m.value as i32).sum();
    (sum / modifiers.len() as i32) as i16
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Test 1: Zero commute -> Excellent, positive bonus ──────────────────

    #[test]
    fn zero_commute_is_excellent() {
        let model = DefaultCommuteModel::default();
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 0,
            has_transit: false,
        });
        assert_eq!(result.category, CommuteCategory::Excellent);
        assert_eq!(result.value, 20);
        assert!(result.value > 0);
    }

    // ─── Test 2: Short commute -> Good, moderate bonus ──────────────────────

    #[test]
    fn short_commute_is_good() {
        let model = DefaultCommuteModel::default();
        // 100 ticks: between short_threshold (50) and medium_threshold (200)
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 100,
            has_transit: false,
        });
        assert_eq!(result.category, CommuteCategory::Good);
        assert!(result.value >= 0);
        assert!(result.value < model.max_bonus);
    }

    // ─── Test 3: Medium commute -> Tolerable, small penalty ─────────────────

    #[test]
    fn medium_commute_is_tolerable() {
        let model = DefaultCommuteModel::default();
        // 350 ticks: between medium_threshold (200) and long_threshold (500)
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 350,
            has_transit: false,
        });
        assert_eq!(result.category, CommuteCategory::Tolerable);
        assert!(result.value < 0);
    }

    // ─── Test 4: Long commute -> Poor, max penalty ──────────────────────────

    #[test]
    fn long_commute_is_poor() {
        let model = DefaultCommuteModel::default();
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 600,
            has_transit: false,
        });
        assert_eq!(result.category, CommuteCategory::Poor);
        assert_eq!(result.value, -50);
    }

    // ─── Test 5: Transit reduces effective commute ──────────────────────────

    #[test]
    fn transit_reduces_effective_commute() {
        let model = DefaultCommuteModel::default();
        // 60 ticks without transit -> just over short_threshold (50), Good
        let without = model.evaluate(&CommuteContext {
            commute_ticks: 60,
            has_transit: false,
        });
        assert_eq!(without.category, CommuteCategory::Good);

        // 60 ticks with transit -> 60 * 70/100 = 42, below short_threshold, Excellent
        let with = model.evaluate(&CommuteContext {
            commute_ticks: 60,
            has_transit: true,
        });
        assert_eq!(with.category, CommuteCategory::Excellent);
        assert!(with.value > without.value);
    }

    // ─── Test 6: Transit can push from Tolerable to Good ────────────────────

    #[test]
    fn transit_pushes_tolerable_to_good() {
        let model = DefaultCommuteModel::default();
        // 250 ticks without transit -> Tolerable (between 200 and 500)
        let without = model.evaluate(&CommuteContext {
            commute_ticks: 250,
            has_transit: false,
        });
        assert_eq!(without.category, CommuteCategory::Tolerable);

        // 250 ticks with transit -> 250 * 70/100 = 175, below 200 -> Good
        let with = model.evaluate(&CommuteContext {
            commute_ticks: 250,
            has_transit: true,
        });
        assert_eq!(with.category, CommuteCategory::Good);
    }

    // ─── Test 7: Default thresholds are reasonable ──────────────────────────

    #[test]
    fn default_thresholds_are_reasonable() {
        let model = DefaultCommuteModel::default();
        assert_eq!(model.short_threshold, 50);
        assert_eq!(model.medium_threshold, 200);
        assert_eq!(model.long_threshold, 500);
        assert_eq!(model.transit_reduction_pct, 30);
        assert_eq!(model.max_bonus, 20);
        assert_eq!(model.max_penalty, -50);
        // Thresholds are in ascending order
        assert!(model.short_threshold < model.medium_threshold);
        assert!(model.medium_threshold < model.long_threshold);
    }

    // ─── Test 8: batch_evaluate processes all entries ────────────────────────

    #[test]
    fn batch_evaluate_processes_all() {
        let model = DefaultCommuteModel::default();
        let inputs = vec![(0, false), (100, false), (350, false), (600, true)];
        let results = batch_evaluate(&inputs, &model);
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].category, CommuteCategory::Excellent);
        assert_eq!(results[1].category, CommuteCategory::Good);
        assert_eq!(results[2].category, CommuteCategory::Tolerable);
        // 600 * 70/100 = 420 -> still Tolerable (between 200 and 500)
        assert_eq!(results[3].category, CommuteCategory::Tolerable);
    }

    // ─── Test 9: average_modifier computes correctly ────────────────────────

    #[test]
    fn average_modifier_computes_correctly() {
        let modifiers = vec![
            HappinessModifier {
                value: 20,
                category: CommuteCategory::Excellent,
            },
            HappinessModifier {
                value: -50,
                category: CommuteCategory::Poor,
            },
        ];
        // (20 + -50) / 2 = -15
        assert_eq!(average_modifier(&modifiers), -15);
    }

    // ─── Test 10: Empty batch -> zero average ───────────────────────────────

    #[test]
    fn empty_batch_gives_zero_average() {
        assert_eq!(average_modifier(&[]), 0);
    }

    // ─── Test 11: Model name is correct ─────────────────────────────────────

    #[test]
    fn model_name_is_correct() {
        let model = DefaultCommuteModel::default();
        assert_eq!(model.name(), "default_threshold");
    }

    // ─── Test 12: Boundary - exactly at short threshold ─────────────────────

    #[test]
    fn boundary_at_short_threshold() {
        let model = DefaultCommuteModel::default();
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 50, // exactly at short_threshold
            has_transit: false,
        });
        // <= short_threshold means Excellent
        assert_eq!(result.category, CommuteCategory::Excellent);
        assert_eq!(result.value, model.max_bonus);
    }

    // ─── Test 13: Boundary - exactly at medium threshold ────────────────────

    #[test]
    fn boundary_at_medium_threshold() {
        let model = DefaultCommuteModel::default();
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 200, // exactly at medium_threshold
            has_transit: false,
        });
        // <= medium_threshold means Good, value should be 0 (end of interpolation)
        assert_eq!(result.category, CommuteCategory::Good);
        assert_eq!(result.value, 0);
    }

    // ─── Test 14: Boundary - exactly at long threshold ──────────────────────

    #[test]
    fn boundary_at_long_threshold() {
        let model = DefaultCommuteModel::default();
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 500, // exactly at long_threshold
            has_transit: false,
        });
        // <= long_threshold means Tolerable, value should be max_penalty
        assert_eq!(result.category, CommuteCategory::Tolerable);
        assert_eq!(result.value, model.max_penalty);
    }

    // ─── Test 15: HappinessModifier equality ────────────────────────────────

    #[test]
    fn happiness_modifier_equality() {
        let a = HappinessModifier {
            value: 10,
            category: CommuteCategory::Good,
        };
        let b = HappinessModifier {
            value: 10,
            category: CommuteCategory::Good,
        };
        let c = HappinessModifier {
            value: -10,
            category: CommuteCategory::Tolerable,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ─── Test 16: Max penalty is capped ─────────────────────────────────────

    #[test]
    fn max_penalty_is_capped() {
        let model = DefaultCommuteModel::default();
        // Very large commute should still produce max_penalty, not worse
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 10_000,
            has_transit: false,
        });
        assert_eq!(result.value, model.max_penalty);
        assert_eq!(result.category, CommuteCategory::Poor);
    }

    // ─── Test 17: Custom model thresholds work ──────────────────────────────

    #[test]
    fn custom_model_thresholds() {
        let model = DefaultCommuteModel {
            short_threshold: 10,
            medium_threshold: 20,
            long_threshold: 30,
            transit_reduction_pct: 50,
            max_bonus: 100,
            max_penalty: -100,
            ..Default::default()
        };
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 5,
            has_transit: false,
        });
        assert_eq!(result.category, CommuteCategory::Excellent);
        assert_eq!(result.value, 100);

        let result = model.evaluate(&CommuteContext {
            commute_ticks: 35,
            has_transit: false,
        });
        assert_eq!(result.category, CommuteCategory::Poor);
        assert_eq!(result.value, -100);
    }

    // ─── Test 18: average_modifier with single element ──────────────────────

    #[test]
    fn average_modifier_single_element() {
        let modifiers = vec![HappinessModifier {
            value: 15,
            category: CommuteCategory::Good,
        }];
        assert_eq!(average_modifier(&modifiers), 15);
    }

    // ─── Test 19: batch_evaluate with empty input ───────────────────────────

    #[test]
    fn batch_evaluate_empty_input() {
        let model = DefaultCommuteModel::default();
        let results = batch_evaluate(&[], &model);
        assert!(results.is_empty());
    }

    // ─── Test 20: Good category linear interpolation ────────────────────────

    #[test]
    fn good_category_linear_interpolation() {
        let model = DefaultCommuteModel::default();
        // Midpoint of Good range: (50 + 200) / 2 = 125
        let result = model.evaluate(&CommuteContext {
            commute_ticks: 125,
            has_transit: false,
        });
        assert_eq!(result.category, CommuteCategory::Good);
        // Progress = 125 - 50 = 75, range = 150
        // value = 20 - (20 * 75 / 150) = 20 - 10 = 10
        assert_eq!(result.value, 10);
    }
}
