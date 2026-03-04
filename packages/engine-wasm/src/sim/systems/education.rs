//! Education cohort pipeline: multi-stage temporal education model.

/// Education stages (pipeline buckets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EducationStage {
    Uneducated = 0,
    Elementary = 1,
    HighSchool = 2,
    University = 3,
    Graduated = 4,
}

impl EducationStage {
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Uneducated => Some(Self::Elementary),
            Self::Elementary => Some(Self::HighSchool),
            Self::HighSchool => Some(Self::University),
            Self::University => Some(Self::Graduated),
            Self::Graduated => None,
        }
    }
}

/// Configuration for education timing.
#[derive(Debug, Clone)]
pub struct EducationConfig {
    /// Ticks per stage to advance (configurable by plugin)
    pub ticks_per_stage: [u32; 4], // elem, hs, uni, (graduated is instant)
    /// School capacity affects throughput
    pub capacity_multiplier: f32,
}

impl Default for EducationConfig {
    fn default() -> Self {
        Self {
            ticks_per_stage: [200, 300, 400, 0], // elementary, hs, university
            capacity_multiplier: 1.0,
        }
    }
}

/// Cohort tracking for each education stage.
#[derive(Debug, Clone, Default)]
pub struct EducationCohorts {
    pub uneducated: u32,
    pub elementary: u32,
    pub high_school: u32,
    pub university: u32,
    pub graduated: u32,
}

impl EducationCohorts {
    pub fn total(&self) -> u32 {
        self.uneducated + self.elementary + self.high_school + self.university + self.graduated
    }

    pub fn get(&self, stage: EducationStage) -> u32 {
        match stage {
            EducationStage::Uneducated => self.uneducated,
            EducationStage::Elementary => self.elementary,
            EducationStage::HighSchool => self.high_school,
            EducationStage::University => self.university,
            EducationStage::Graduated => self.graduated,
        }
    }

    pub fn set(&mut self, stage: EducationStage, value: u32) {
        match stage {
            EducationStage::Uneducated => self.uneducated = value,
            EducationStage::Elementary => self.elementary = value,
            EducationStage::HighSchool => self.high_school = value,
            EducationStage::University => self.university = value,
            EducationStage::Graduated => self.graduated = value,
        }
    }
}

/// Trait for pluggable education pipeline.
pub trait IEducationPipeline {
    fn advance_cohorts(
        &self,
        cohorts: &mut EducationCohorts,
        school_capacity: u32,
        tick: u64,
    ) -> EducationUpdate;
    fn name(&self) -> &str;
}

/// Result of an education tick.
#[derive(Debug, Clone, Default)]
pub struct EducationUpdate {
    pub newly_enrolled: u32,
    pub newly_graduated: u32,
    pub stalled: u32, // couldn't advance due to capacity
}

/// Default education pipeline.
pub struct DefaultEducationPipeline {
    pub config: EducationConfig,
}

impl DefaultEducationPipeline {
    pub fn new(config: EducationConfig) -> Self {
        Self { config }
    }
}

impl Default for DefaultEducationPipeline {
    fn default() -> Self {
        Self {
            config: EducationConfig::default(),
        }
    }
}

impl IEducationPipeline for DefaultEducationPipeline {
    fn advance_cohorts(
        &self,
        cohorts: &mut EducationCohorts,
        school_capacity: u32,
        tick: u64,
    ) -> EducationUpdate {
        let mut update = EducationUpdate::default();

        // Process stages in reverse order to avoid double-counting
        // University -> Graduated
        if cohorts.university > 0 && tick % self.config.ticks_per_stage[2] as u64 == 0 {
            let advance = cohorts.university.min(school_capacity);
            cohorts.university -= advance;
            cohorts.graduated += advance;
            update.newly_graduated += advance;
            if cohorts.university > 0 {
                update.stalled += cohorts.university;
            }
        }

        // HighSchool -> University
        if cohorts.high_school > 0 && tick % self.config.ticks_per_stage[1] as u64 == 0 {
            let advance = cohorts.high_school.min(school_capacity);
            cohorts.high_school -= advance;
            cohorts.university += advance;
        }

        // Elementary -> HighSchool
        if cohorts.elementary > 0 && tick % self.config.ticks_per_stage[0] as u64 == 0 {
            let advance = cohorts.elementary.min(school_capacity);
            cohorts.elementary -= advance;
            cohorts.high_school += advance;
        }

        // Uneducated -> Elementary (always enrolling)
        if cohorts.uneducated > 0 {
            let enroll = cohorts.uneducated.min(school_capacity);
            cohorts.uneducated -= enroll;
            cohorts.elementary += enroll;
            update.newly_enrolled = enroll;
        }

        update
    }

    fn name(&self) -> &str {
        "default_pipeline"
    }
}

/// Compute workforce skill tier from education distribution.
pub fn workforce_skill_tier(cohorts: &EducationCohorts) -> u8 {
    let total = cohorts.total();
    if total == 0 {
        return 0;
    }
    let educated = cohorts.graduated + cohorts.university;
    let ratio = (educated as u64 * 100 / total as u64) as u8;
    if ratio >= 60 {
        3
    } else if ratio >= 30 {
        2
    } else if ratio >= 10 {
        1
    } else {
        0
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Test 1: EducationStage next() progression ──────────────────────────

    #[test]
    fn stage_next_progression() {
        assert_eq!(EducationStage::Uneducated.next(), Some(EducationStage::Elementary));
        assert_eq!(EducationStage::Elementary.next(), Some(EducationStage::HighSchool));
        assert_eq!(EducationStage::HighSchool.next(), Some(EducationStage::University));
        assert_eq!(EducationStage::University.next(), Some(EducationStage::Graduated));
    }

    // ─── Test 2: EducationStage Graduated has no next ───────────────────────

    #[test]
    fn graduated_has_no_next() {
        assert_eq!(EducationStage::Graduated.next(), None);
    }

    // ─── Test 3: EducationCohorts total() sums all ──────────────────────────

    #[test]
    fn cohorts_total_sums_all() {
        let cohorts = EducationCohorts {
            uneducated: 10,
            elementary: 20,
            high_school: 30,
            university: 40,
            graduated: 50,
        };
        assert_eq!(cohorts.total(), 150);
    }

    // ─── Test 4: get/set work for each stage ────────────────────────────────

    #[test]
    fn get_set_work_for_each_stage() {
        let mut cohorts = EducationCohorts::default();

        cohorts.set(EducationStage::Uneducated, 11);
        assert_eq!(cohorts.get(EducationStage::Uneducated), 11);

        cohorts.set(EducationStage::Elementary, 22);
        assert_eq!(cohorts.get(EducationStage::Elementary), 22);

        cohorts.set(EducationStage::HighSchool, 33);
        assert_eq!(cohorts.get(EducationStage::HighSchool), 33);

        cohorts.set(EducationStage::University, 44);
        assert_eq!(cohorts.get(EducationStage::University), 44);

        cohorts.set(EducationStage::Graduated, 55);
        assert_eq!(cohorts.get(EducationStage::Graduated), 55);
    }

    // ─── Test 5: Default cohorts are zeroed ─────────────────────────────────

    #[test]
    fn default_cohorts_are_zeroed() {
        let cohorts = EducationCohorts::default();
        assert_eq!(cohorts.uneducated, 0);
        assert_eq!(cohorts.elementary, 0);
        assert_eq!(cohorts.high_school, 0);
        assert_eq!(cohorts.university, 0);
        assert_eq!(cohorts.graduated, 0);
        assert_eq!(cohorts.total(), 0);
    }

    // ─── Test 6: DefaultEducationPipeline enrolls uneducated ────────────────

    #[test]
    fn pipeline_enrolls_uneducated() {
        let pipeline = DefaultEducationPipeline::default();
        let mut cohorts = EducationCohorts {
            uneducated: 100,
            ..Default::default()
        };
        let update = pipeline.advance_cohorts(&mut cohorts, 50, 1);
        // Should enroll up to capacity
        assert_eq!(update.newly_enrolled, 50);
        assert_eq!(cohorts.uneducated, 50);
        assert_eq!(cohorts.elementary, 50);
    }

    // ─── Test 7: Pipeline advances elementary to high school at tick interval ─

    #[test]
    fn pipeline_advances_elementary_at_interval() {
        let pipeline = DefaultEducationPipeline::default();
        let mut cohorts = EducationCohorts {
            elementary: 40,
            ..Default::default()
        };
        // ticks_per_stage[0] = 200, so tick 200 triggers advancement
        let _update = pipeline.advance_cohorts(&mut cohorts, 100, 200);
        assert_eq!(cohorts.elementary, 0);
        assert_eq!(cohorts.high_school, 40);
    }

    // ─── Test 8: Pipeline advances through all stages over time ─────────────

    #[test]
    fn pipeline_advances_through_all_stages() {
        let config = EducationConfig {
            ticks_per_stage: [10, 20, 30, 0],
            capacity_multiplier: 1.0,
        };
        let pipeline = DefaultEducationPipeline::new(config);
        let mut cohorts = EducationCohorts {
            uneducated: 5,
            ..Default::default()
        };

        // Tick 0: enroll uneducated -> elementary
        // (tick 0 also triggers all modulo conditions, so process carefully)
        // Use tick 1 to just enroll
        pipeline.advance_cohorts(&mut cohorts, 100, 1);
        assert_eq!(cohorts.uneducated, 0);
        assert_eq!(cohorts.elementary, 5);

        // Tick 10: elementary -> high_school
        pipeline.advance_cohorts(&mut cohorts, 100, 10);
        assert_eq!(cohorts.elementary, 0);
        assert_eq!(cohorts.high_school, 5);

        // Tick 20: high_school -> university
        pipeline.advance_cohorts(&mut cohorts, 100, 20);
        assert_eq!(cohorts.high_school, 0);
        assert_eq!(cohorts.university, 5);

        // Tick 30: university -> graduated
        pipeline.advance_cohorts(&mut cohorts, 100, 30);
        assert_eq!(cohorts.university, 0);
        assert_eq!(cohorts.graduated, 5);
    }

    // ─── Test 9: Capacity limits throughput (stalled count) ─────────────────

    #[test]
    fn capacity_limits_throughput() {
        let config = EducationConfig {
            ticks_per_stage: [10, 20, 30, 0],
            capacity_multiplier: 1.0,
        };
        let pipeline = DefaultEducationPipeline::new(config);
        let mut cohorts = EducationCohorts {
            university: 50,
            ..Default::default()
        };
        // Capacity of only 10; tick 30 triggers university -> graduated
        let update = pipeline.advance_cohorts(&mut cohorts, 10, 30);
        assert_eq!(update.newly_graduated, 10);
        assert_eq!(cohorts.graduated, 10);
        assert_eq!(cohorts.university, 40);
        assert_eq!(update.stalled, 40); // remaining could not advance
    }

    // ─── Test 10: workforce_skill_tier returns 0 for all uneducated ─────────

    #[test]
    fn skill_tier_zero_for_all_uneducated() {
        let cohorts = EducationCohorts {
            uneducated: 100,
            ..Default::default()
        };
        assert_eq!(workforce_skill_tier(&cohorts), 0);
    }

    // ─── Test 11: workforce_skill_tier returns 3 for mostly graduated ───────

    #[test]
    fn skill_tier_three_for_mostly_graduated() {
        let cohorts = EducationCohorts {
            uneducated: 10,
            graduated: 90,
            ..Default::default()
        };
        // 90 / 100 = 90% >= 60%
        assert_eq!(workforce_skill_tier(&cohorts), 3);
    }

    // ─── Test 12: EducationConfig defaults are reasonable ───────────────────

    #[test]
    fn config_defaults_are_reasonable() {
        let config = EducationConfig::default();
        assert_eq!(config.ticks_per_stage[0], 200);
        assert_eq!(config.ticks_per_stage[1], 300);
        assert_eq!(config.ticks_per_stage[2], 400);
        assert_eq!(config.ticks_per_stage[3], 0);
        assert!((config.capacity_multiplier - 1.0).abs() < f32::EPSILON);
    }

    // ─── Test 13: Empty cohorts produce zero update ─────────────────────────

    #[test]
    fn empty_cohorts_produce_zero_update() {
        let pipeline = DefaultEducationPipeline::default();
        let mut cohorts = EducationCohorts::default();
        let update = pipeline.advance_cohorts(&mut cohorts, 100, 200);
        assert_eq!(update.newly_enrolled, 0);
        assert_eq!(update.newly_graduated, 0);
        assert_eq!(update.stalled, 0);
        assert_eq!(cohorts.total(), 0);
    }

    // ─── Test 14: Pipeline name is correct ──────────────────────────────────

    #[test]
    fn pipeline_name_is_correct() {
        let pipeline = DefaultEducationPipeline::default();
        assert_eq!(pipeline.name(), "default_pipeline");
    }

    // ─── Test 15: workforce_skill_tier returns 0 for empty cohorts ──────────

    #[test]
    fn skill_tier_zero_for_empty_cohorts() {
        let cohorts = EducationCohorts::default();
        assert_eq!(workforce_skill_tier(&cohorts), 0);
    }

    // ─── Test 16: workforce_skill_tier tier boundaries ──────────────────────

    #[test]
    fn skill_tier_boundaries() {
        // Exactly 10% educated -> tier 1
        let cohorts = EducationCohorts {
            uneducated: 90,
            graduated: 10,
            ..Default::default()
        };
        assert_eq!(workforce_skill_tier(&cohorts), 1);

        // Exactly 30% educated -> tier 2
        let cohorts = EducationCohorts {
            uneducated: 70,
            graduated: 30,
            ..Default::default()
        };
        assert_eq!(workforce_skill_tier(&cohorts), 2);

        // Exactly 60% educated -> tier 3
        let cohorts = EducationCohorts {
            uneducated: 40,
            graduated: 60,
            ..Default::default()
        };
        assert_eq!(workforce_skill_tier(&cohorts), 3);
    }

    // ─── Test 17: EducationStage ordering ───────────────────────────────────

    #[test]
    fn stage_ordering() {
        assert!(EducationStage::Uneducated < EducationStage::Elementary);
        assert!(EducationStage::Elementary < EducationStage::HighSchool);
        assert!(EducationStage::HighSchool < EducationStage::University);
        assert!(EducationStage::University < EducationStage::Graduated);
    }

    // ─── Test 18: Pipeline enrolls only up to capacity ──────────────────────

    #[test]
    fn pipeline_enrolls_only_up_to_capacity() {
        let pipeline = DefaultEducationPipeline::default();
        let mut cohorts = EducationCohorts {
            uneducated: 200,
            ..Default::default()
        };
        let update = pipeline.advance_cohorts(&mut cohorts, 30, 1);
        assert_eq!(update.newly_enrolled, 30);
        assert_eq!(cohorts.uneducated, 170);
        assert_eq!(cohorts.elementary, 30);
    }

    // ─── Test 19: Non-advancement tick preserves cohort counts ──────────────

    #[test]
    fn non_advancement_tick_preserves_cohorts() {
        let pipeline = DefaultEducationPipeline::default();
        let mut cohorts = EducationCohorts {
            elementary: 50,
            high_school: 30,
            university: 20,
            graduated: 10,
            ..Default::default()
        };
        // Tick 1 is not a multiple of any ticks_per_stage, so only enrollment runs
        // (and there are no uneducated to enroll)
        let update = pipeline.advance_cohorts(&mut cohorts, 100, 1);
        assert_eq!(cohorts.elementary, 50);
        assert_eq!(cohorts.high_school, 30);
        assert_eq!(cohorts.university, 20);
        assert_eq!(cohorts.graduated, 10);
        assert_eq!(update.newly_enrolled, 0);
        assert_eq!(update.newly_graduated, 0);
    }
}
