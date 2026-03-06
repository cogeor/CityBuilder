use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldVars {
    pub beds_per_1000: f32,
    pub outpatient_visits_per_capita: f32,
    pub police_officers_per_1000: f32,
    pub fire_staff_per_1000: f32,
    pub spending_mobility: f32,
    pub needs_mobility_min: f32,
    pub freight_trips_per_1000_per_day: f32,
    pub congestion_slope: f32,
    pub grow_min_threshold: f32,
    pub abandon_util_threshold: f32,
    pub abandon_days: f32,
    pub target_jobs_housing_ratio: f32,
}

impl Default for WorldVars {
    fn default() -> Self {
        Self {
            beds_per_1000: 4.2,
            outpatient_visits_per_capita: 2.6,
            police_officers_per_1000: 2.4,
            fire_staff_per_1000: 1.4,
            spending_mobility: 0.17,
            needs_mobility_min: 0.6,
            freight_trips_per_1000_per_day: 350.0,
            congestion_slope: 0.2,
            grow_min_threshold: 0.55,
            abandon_util_threshold: 0.35,
            abandon_days: 90.0,
            target_jobs_housing_ratio: 1.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_baselines() {
        let v = WorldVars::default();
        assert_eq!(v.beds_per_1000, 4.2_f32);
        assert_eq!(v.outpatient_visits_per_capita, 2.6_f32);
        assert_eq!(v.police_officers_per_1000, 2.4_f32);
        assert_eq!(v.fire_staff_per_1000, 1.4_f32);
        assert_eq!(v.spending_mobility, 0.17_f32);
        assert_eq!(v.needs_mobility_min, 0.6_f32);
        assert_eq!(v.freight_trips_per_1000_per_day, 350.0_f32);
        assert_eq!(v.congestion_slope, 0.2_f32);
        assert_eq!(v.grow_min_threshold, 0.55_f32);
        assert_eq!(v.abandon_util_threshold, 0.35_f32);
        assert_eq!(v.abandon_days, 90.0_f32);
        assert_eq!(v.target_jobs_housing_ratio, 1.1_f32);
    }
}
