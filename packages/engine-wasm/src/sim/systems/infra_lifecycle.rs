//! Infrastructure lifecycle: aging, degradation, renovation.

/// Age stages for infrastructure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgeStage {
    New = 0,
    Mature = 1,
    Worn = 2,
    Dilapidated = 3,
}

/// Lifecycle configuration per archetype.
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    pub new_to_mature_days: u16,
    pub mature_to_worn_days: u16,
    pub worn_to_dilapidated_days: u16,
    pub renovation_cost_ratio: f32, // fraction of original build cost
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            new_to_mature_days: 365,
            mature_to_worn_days: 730,
            worn_to_dilapidated_days: 365,
            renovation_cost_ratio: 0.5,
        }
    }
}

/// Result of lifecycle evaluation.
#[derive(Debug, Clone)]
pub struct LifecycleEffect {
    pub upkeep_multiplier: f32,        // 1.0 = normal, 1.25 = worn, 1.5 = dilapidated
    pub effectiveness_multiplier: f32, // 1.0 = normal, 0.9 = worn, 0.7 = dilapidated
    pub abandonment_risk: bool,        // true for dilapidated
}

/// Trait for pluggable lifecycle behavior.
pub trait IInfraLifecycle {
    fn get_stage(&self, age_days: u16, config: &LifecycleConfig) -> AgeStage;
    fn get_effect(&self, stage: AgeStage) -> LifecycleEffect;
    fn name(&self) -> &str;
}

/// Default lifecycle model.
pub struct DefaultInfraLifecycle;

impl IInfraLifecycle for DefaultInfraLifecycle {
    fn get_stage(&self, age_days: u16, config: &LifecycleConfig) -> AgeStage {
        let threshold1 = config.new_to_mature_days;
        let threshold2 = threshold1 + config.mature_to_worn_days;
        let threshold3 = threshold2 + config.worn_to_dilapidated_days;

        if age_days >= threshold3 {
            AgeStage::Dilapidated
        } else if age_days >= threshold2 {
            AgeStage::Worn
        } else if age_days >= threshold1 {
            AgeStage::Mature
        } else {
            AgeStage::New
        }
    }

    fn get_effect(&self, stage: AgeStage) -> LifecycleEffect {
        match stage {
            AgeStage::New => LifecycleEffect {
                upkeep_multiplier: 1.0,
                effectiveness_multiplier: 1.0,
                abandonment_risk: false,
            },
            AgeStage::Mature => LifecycleEffect {
                upkeep_multiplier: 1.0,
                effectiveness_multiplier: 1.0,
                abandonment_risk: false,
            },
            AgeStage::Worn => LifecycleEffect {
                upkeep_multiplier: 1.25,
                effectiveness_multiplier: 0.9,
                abandonment_risk: false,
            },
            AgeStage::Dilapidated => LifecycleEffect {
                upkeep_multiplier: 1.5,
                effectiveness_multiplier: 0.7,
                abandonment_risk: true,
            },
        }
    }

    fn name(&self) -> &str {
        "default_lifecycle"
    }
}

/// Compute renovation cost from original build cost.
pub fn renovation_cost(build_cost: u64, config: &LifecycleConfig) -> u64 {
    (build_cost as f64 * config.renovation_cost_ratio as f64) as u64
}

/// Check if a building should be considered for abandonment.
pub fn should_consider_abandonment(stage: AgeStage, rng_value: u16) -> bool {
    if stage != AgeStage::Dilapidated {
        return false;
    }
    // 5% chance per check for dilapidated buildings
    rng_value < (u16::MAX / 20)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_model() -> DefaultInfraLifecycle {
        DefaultInfraLifecycle
    }

    fn default_config() -> LifecycleConfig {
        LifecycleConfig::default()
    }

    // ─── Test 1: New building at age 0 ──────────────────────────────────

    #[test]
    fn new_building_at_age_zero() {
        let model = default_model();
        let config = default_config();

        let stage = model.get_stage(0, &config);
        assert_eq!(stage, AgeStage::New);
    }

    // ─── Test 2: Transitions to Mature at threshold ─────────────────────

    #[test]
    fn transitions_to_mature_at_threshold() {
        let model = default_model();
        let config = default_config();

        // Just below threshold: still New
        let stage_before = model.get_stage(364, &config);
        assert_eq!(stage_before, AgeStage::New);

        // At threshold: Mature
        let stage_at = model.get_stage(365, &config);
        assert_eq!(stage_at, AgeStage::Mature);
    }

    // ─── Test 3: Transitions to Worn at threshold ───────────────────────

    #[test]
    fn transitions_to_worn_at_threshold() {
        let model = default_model();
        let config = default_config();

        // threshold2 = 365 + 730 = 1095
        let stage_before = model.get_stage(1094, &config);
        assert_eq!(stage_before, AgeStage::Mature);

        let stage_at = model.get_stage(1095, &config);
        assert_eq!(stage_at, AgeStage::Worn);
    }

    // ─── Test 4: Transitions to Dilapidated at threshold ────────────────

    #[test]
    fn transitions_to_dilapidated_at_threshold() {
        let model = default_model();
        let config = default_config();

        // threshold3 = 365 + 730 + 365 = 1460
        let stage_before = model.get_stage(1459, &config);
        assert_eq!(stage_before, AgeStage::Worn);

        let stage_at = model.get_stage(1460, &config);
        assert_eq!(stage_at, AgeStage::Dilapidated);
    }

    // ─── Test 5: Worn effect +25% upkeep, -10% effectiveness ────────────

    #[test]
    fn worn_effect_upkeep_and_effectiveness() {
        let model = default_model();
        let effect = model.get_effect(AgeStage::Worn);

        assert!((effect.upkeep_multiplier - 1.25).abs() < f32::EPSILON);
        assert!((effect.effectiveness_multiplier - 0.9).abs() < f32::EPSILON);
        assert!(!effect.abandonment_risk);
    }

    // ─── Test 6: Dilapidated effect +50% upkeep, -30% eff, abandonment ──

    #[test]
    fn dilapidated_effect_upkeep_effectiveness_abandonment() {
        let model = default_model();
        let effect = model.get_effect(AgeStage::Dilapidated);

        assert!((effect.upkeep_multiplier - 1.5).abs() < f32::EPSILON);
        assert!((effect.effectiveness_multiplier - 0.7).abs() < f32::EPSILON);
        assert!(effect.abandonment_risk);
    }

    // ─── Test 7: renovation_cost proportional to build cost ─────────────

    #[test]
    fn renovation_cost_proportional() {
        let config = default_config();

        assert_eq!(renovation_cost(1000, &config), 500);
        assert_eq!(renovation_cost(2000, &config), 1000);
        assert_eq!(renovation_cost(0, &config), 0);
    }

    // ─── Test 8: should_consider_abandonment only for Dilapidated ───────

    #[test]
    fn abandonment_only_for_dilapidated() {
        // Non-dilapidated stages should never trigger abandonment
        assert!(!should_consider_abandonment(AgeStage::New, 0));
        assert!(!should_consider_abandonment(AgeStage::Mature, 0));
        assert!(!should_consider_abandonment(AgeStage::Worn, 0));

        // Dilapidated with very low rng should trigger
        assert!(should_consider_abandonment(AgeStage::Dilapidated, 0));

        // Dilapidated with high rng should not trigger
        assert!(!should_consider_abandonment(AgeStage::Dilapidated, u16::MAX));
    }

    // ─── Test 9: Default config reasonable values ───────────────────────

    #[test]
    fn default_config_reasonable_values() {
        let config = default_config();

        assert_eq!(config.new_to_mature_days, 365);
        assert_eq!(config.mature_to_worn_days, 730);
        assert_eq!(config.worn_to_dilapidated_days, 365);
        assert!((config.renovation_cost_ratio - 0.5).abs() < f32::EPSILON);
    }

    // ─── Test 10: AgeStage ordering correct ─────────────────────────────

    #[test]
    fn age_stage_ordering() {
        assert!(AgeStage::New < AgeStage::Mature);
        assert!(AgeStage::Mature < AgeStage::Worn);
        assert!(AgeStage::Worn < AgeStage::Dilapidated);
    }

    // ─── Test 11: Model name correct ────────────────────────────────────

    #[test]
    fn model_name_correct() {
        let model = default_model();
        assert_eq!(model.name(), "default_lifecycle");
    }

    // ─── Test 12: New and Mature effects are identical ──────────────────

    #[test]
    fn new_and_mature_effects_identical() {
        let model = default_model();
        let new_effect = model.get_effect(AgeStage::New);
        let mature_effect = model.get_effect(AgeStage::Mature);

        assert!((new_effect.upkeep_multiplier - mature_effect.upkeep_multiplier).abs() < f32::EPSILON);
        assert!((new_effect.effectiveness_multiplier - mature_effect.effectiveness_multiplier).abs() < f32::EPSILON);
        assert_eq!(new_effect.abandonment_risk, mature_effect.abandonment_risk);
    }

    // ─── Test 13: Custom config thresholds work ─────────────────────────

    #[test]
    fn custom_config_thresholds() {
        let model = default_model();
        let config = LifecycleConfig {
            new_to_mature_days: 100,
            mature_to_worn_days: 200,
            worn_to_dilapidated_days: 100,
            renovation_cost_ratio: 0.3,
        };

        assert_eq!(model.get_stage(99, &config), AgeStage::New);
        assert_eq!(model.get_stage(100, &config), AgeStage::Mature);
        assert_eq!(model.get_stage(299, &config), AgeStage::Mature);
        assert_eq!(model.get_stage(300, &config), AgeStage::Worn);
        assert_eq!(model.get_stage(399, &config), AgeStage::Worn);
        assert_eq!(model.get_stage(400, &config), AgeStage::Dilapidated);
    }

    // ─── Test 14: Renovation cost with custom ratio ─────────────────────

    #[test]
    fn renovation_cost_custom_ratio() {
        let config = LifecycleConfig {
            renovation_cost_ratio: 0.75,
            ..Default::default()
        };

        assert_eq!(renovation_cost(1000, &config), 750);
    }
}
