//! Day/night cycle gameplay effects with swappable trait.

/// Multipliers applied to simulation systems based on time of day.
#[derive(Debug, Clone, Copy)]
pub struct DiurnalModifiers {
    pub crime_mult: f32,
    pub commercial_revenue_mult: f32,
    pub power_demand_mult: f32,
    pub noise_mult: f32,
    pub traffic_mult: f32,
}

impl Default for DiurnalModifiers {
    fn default() -> Self {
        Self {
            crime_mult: 1.0,
            commercial_revenue_mult: 1.0,
            power_demand_mult: 1.0,
            noise_mult: 1.0,
            traffic_mult: 1.0,
        }
    }
}

/// Trait for pluggable day/night cycle effects.
pub trait IDiurnalEffects {
    fn get_multipliers(&self, game_hour: u8) -> DiurnalModifiers;
    fn name(&self) -> &str;
}

/// Default implementation with smooth hour-based curves.
///
/// Period definitions:
/// - Night      (22:00-06:00): crime 1.4, commercial 0.2, power 0.8, noise 0.3
/// - Daytime    (08:00-16:00): all multipliers 1.0 (baseline)
/// - Rush hours (07-09, 17-19): traffic 1.5
///
/// Transition hours use linear interpolation between adjacent period values.
pub struct DefaultDiurnalEffects;

impl Default for DefaultDiurnalEffects {
    fn default() -> Self {
        Self
    }
}

/// Linear interpolation between two f32 values.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Night-period target values.
const NIGHT_CRIME: f32 = 1.4;
const NIGHT_COMMERCIAL: f32 = 0.2;
const NIGHT_POWER: f32 = 0.8;
const NIGHT_NOISE: f32 = 0.3;

/// Day-period baseline values.
const DAY_CRIME: f32 = 1.0;
const DAY_COMMERCIAL: f32 = 1.0;
const DAY_POWER: f32 = 1.0;
const DAY_NOISE: f32 = 1.0;

/// Rush-hour traffic multiplier.
const RUSH_TRAFFIC: f32 = 1.5;
/// Normal traffic multiplier.
const NORMAL_TRAFFIC: f32 = 1.0;

impl IDiurnalEffects for DefaultDiurnalEffects {
    fn get_multipliers(&self, game_hour: u8) -> DiurnalModifiers {
        let hour = game_hour.min(23);

        // Determine crime, commercial, power, noise based on time period.
        //
        // Core night: 23:00 - 05:00 (full night values)
        // Transition to night: 21:00 - 22:59 (lerp day -> night)
        //   hour 21: t=0.0 (day), hour 22: t=0.5
        // Transition to day: 05:00 - 06:59 (lerp night -> day)
        //   hour 05: t=0.0 (night), hour 06: t=0.5
        // Core day: 07:00 - 20:00 (full day values)
        //
        // This gives us the night window 22:00-06:00 specified in requirements,
        // with smooth 2-hour transition ramps on each side.
        let (crime, commercial, power, noise) = match hour {
            // Core night: 23:00-04:59
            23 | 0..=4 => (NIGHT_CRIME, NIGHT_COMMERCIAL, NIGHT_POWER, NIGHT_NOISE),
            // Transition night -> day: 05:00-06:59
            5 => {
                let t = 0.0; // start of transition, still fully night
                (
                    lerp(NIGHT_CRIME, DAY_CRIME, t),
                    lerp(NIGHT_COMMERCIAL, DAY_COMMERCIAL, t),
                    lerp(NIGHT_POWER, DAY_POWER, t),
                    lerp(NIGHT_NOISE, DAY_NOISE, t),
                )
            }
            6 => {
                let t = 0.5;
                (
                    lerp(NIGHT_CRIME, DAY_CRIME, t),
                    lerp(NIGHT_COMMERCIAL, DAY_COMMERCIAL, t),
                    lerp(NIGHT_POWER, DAY_POWER, t),
                    lerp(NIGHT_NOISE, DAY_NOISE, t),
                )
            }
            // Core day: 07:00-20:59
            7..=20 => (DAY_CRIME, DAY_COMMERCIAL, DAY_POWER, DAY_NOISE),
            // Transition day -> night: 21:00-22:59
            21 => {
                let t = 0.0; // start of transition, still fully day
                (
                    lerp(DAY_CRIME, NIGHT_CRIME, t),
                    lerp(DAY_COMMERCIAL, NIGHT_COMMERCIAL, t),
                    lerp(DAY_POWER, NIGHT_POWER, t),
                    lerp(DAY_NOISE, NIGHT_NOISE, t),
                )
            }
            22 => {
                let t = 0.5;
                (
                    lerp(DAY_CRIME, NIGHT_CRIME, t),
                    lerp(DAY_COMMERCIAL, NIGHT_COMMERCIAL, t),
                    lerp(DAY_POWER, NIGHT_POWER, t),
                    lerp(DAY_NOISE, NIGHT_NOISE, t),
                )
            }
            _ => unreachable!(), // hour is clamped to 0..=23
        };

        // Traffic multiplier: rush hours 7-9 and 17-19 get +50%.
        // Transition: hours 6 and 10 ramp in/out of morning rush;
        // hours 16 and 20 ramp in/out of evening rush.
        let traffic = match hour {
            // Morning rush: 7-9
            7..=9 => RUSH_TRAFFIC,
            // Evening rush: 17-19
            17..=19 => RUSH_TRAFFIC,
            // Transition into morning rush (hour 6)
            6 => lerp(NORMAL_TRAFFIC, RUSH_TRAFFIC, 0.5),
            // Transition out of morning rush (hour 10)
            10 => lerp(RUSH_TRAFFIC, NORMAL_TRAFFIC, 0.5),
            // Transition into evening rush (hour 16)
            16 => lerp(NORMAL_TRAFFIC, RUSH_TRAFFIC, 0.5),
            // Transition out of evening rush (hour 20)
            20 => lerp(RUSH_TRAFFIC, NORMAL_TRAFFIC, 0.5),
            // All other hours: normal
            _ => NORMAL_TRAFFIC,
        };

        DiurnalModifiers {
            crime_mult: crime,
            commercial_revenue_mult: commercial,
            power_demand_mult: power,
            noise_mult: noise,
            traffic_mult: traffic,
        }
    }

    fn name(&self) -> &str {
        "default_diurnal"
    }
}

/// Human-readable description of the time-of-day period for a given hour.
pub fn hour_description(hour: u8) -> &'static str {
    match hour.min(23) {
        0..=4 => "night",
        5..=6 => "dawn",
        7..=9 => "morning_rush",
        10..=11 => "late_morning",
        12..=13 => "midday",
        14..=16 => "afternoon",
        17..=19 => "evening_rush",
        20 => "evening",
        21..=22 => "dusk",
        23 => "night",
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> DefaultDiurnalEffects {
        DefaultDiurnalEffects::default()
    }

    // ---- 1. Midnight modifiers correct ----
    #[test]
    fn midnight_modifiers_correct() {
        let m = model().get_multipliers(0);
        assert!((m.crime_mult - 1.4).abs() < f32::EPSILON);
        assert!((m.commercial_revenue_mult - 0.2).abs() < f32::EPSILON);
        assert!((m.power_demand_mult - 0.8).abs() < f32::EPSILON);
        assert!((m.noise_mult - 0.3).abs() < f32::EPSILON);
    }

    // ---- 2. Noon modifiers correct ----
    #[test]
    fn noon_modifiers_correct() {
        let m = model().get_multipliers(12);
        assert!((m.crime_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.commercial_revenue_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.power_demand_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.noise_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.traffic_mult - 1.0).abs() < f32::EPSILON);
    }

    // ---- 3. Rush hour traffic multiplier (morning) ----
    #[test]
    fn rush_hour_traffic_morning() {
        for hour in 7..=9 {
            let m = model().get_multipliers(hour);
            assert!(
                (m.traffic_mult - 1.5).abs() < f32::EPSILON,
                "hour {} should have traffic_mult=1.5, got {}",
                hour,
                m.traffic_mult
            );
        }
    }

    // ---- 4. Rush hour traffic multiplier (evening) ----
    #[test]
    fn rush_hour_traffic_evening() {
        for hour in 17..=19 {
            let m = model().get_multipliers(hour);
            assert!(
                (m.traffic_mult - 1.5).abs() < f32::EPSILON,
                "hour {} should have traffic_mult=1.5, got {}",
                hour,
                m.traffic_mult
            );
        }
    }

    // ---- 5. Night crime multiplier ----
    #[test]
    fn night_crime_multiplier() {
        // Core night hours: 23, 0, 1, 2, 3, 4
        for hour in [23, 0, 1, 2, 3, 4] {
            let m = model().get_multipliers(hour);
            assert!(
                (m.crime_mult - 1.4).abs() < f32::EPSILON,
                "hour {} should have crime_mult=1.4, got {}",
                hour,
                m.crime_mult
            );
        }
    }

    // ---- 6. Night commercial revenue reduction ----
    #[test]
    fn night_commercial_revenue_reduction() {
        for hour in [23, 0, 1, 2, 3, 4] {
            let m = model().get_multipliers(hour);
            assert!(
                (m.commercial_revenue_mult - 0.2).abs() < f32::EPSILON,
                "hour {} should have commercial_revenue_mult=0.2, got {}",
                hour,
                m.commercial_revenue_mult
            );
        }
    }

    // ---- 7. Night power demand reduction ----
    #[test]
    fn night_power_demand_reduction() {
        for hour in [23, 0, 1, 2, 3, 4] {
            let m = model().get_multipliers(hour);
            assert!(
                (m.power_demand_mult - 0.8).abs() < f32::EPSILON,
                "hour {} should have power_demand_mult=0.8, got {}",
                hour,
                m.power_demand_mult
            );
        }
    }

    // ---- 8. Night noise reduction ----
    #[test]
    fn night_noise_reduction() {
        for hour in [23, 0, 1, 2, 3, 4] {
            let m = model().get_multipliers(hour);
            assert!(
                (m.noise_mult - 0.3).abs() < f32::EPSILON,
                "hour {} should have noise_mult=0.3, got {}",
                hour,
                m.noise_mult
            );
        }
    }

    // ---- 9. Transition hours interpolation (dawn) ----
    #[test]
    fn transition_dawn_interpolation() {
        let m5 = model().get_multipliers(5);
        let m6 = model().get_multipliers(6);

        // Hour 5: still fully night values (t=0.0 in night->day transition)
        assert!((m5.crime_mult - 1.4).abs() < f32::EPSILON);

        // Hour 6: midpoint between night and day
        let expected_crime = lerp(NIGHT_CRIME, DAY_CRIME, 0.5);
        assert!(
            (m6.crime_mult - expected_crime).abs() < f32::EPSILON,
            "hour 6 crime_mult should be {}, got {}",
            expected_crime,
            m6.crime_mult
        );

        let expected_commercial = lerp(NIGHT_COMMERCIAL, DAY_COMMERCIAL, 0.5);
        assert!(
            (m6.commercial_revenue_mult - expected_commercial).abs() < f32::EPSILON,
            "hour 6 commercial_revenue_mult should be {}, got {}",
            expected_commercial,
            m6.commercial_revenue_mult
        );
    }

    // ---- 10. Transition hours interpolation (dusk) ----
    #[test]
    fn transition_dusk_interpolation() {
        let m21 = model().get_multipliers(21);
        let m22 = model().get_multipliers(22);

        // Hour 21: still fully day values (t=0.0 in day->night transition)
        assert!((m21.crime_mult - 1.0).abs() < f32::EPSILON);

        // Hour 22: midpoint between day and night
        let expected_crime = lerp(DAY_CRIME, NIGHT_CRIME, 0.5);
        assert!(
            (m22.crime_mult - expected_crime).abs() < f32::EPSILON,
            "hour 22 crime_mult should be {}, got {}",
            expected_crime,
            m22.crime_mult
        );
    }

    // ---- 11. Hour description correctness ----
    #[test]
    fn hour_description_correctness() {
        assert_eq!(hour_description(0), "night");
        assert_eq!(hour_description(3), "night");
        assert_eq!(hour_description(5), "dawn");
        assert_eq!(hour_description(6), "dawn");
        assert_eq!(hour_description(7), "morning_rush");
        assert_eq!(hour_description(9), "morning_rush");
        assert_eq!(hour_description(10), "late_morning");
        assert_eq!(hour_description(12), "midday");
        assert_eq!(hour_description(14), "afternoon");
        assert_eq!(hour_description(17), "evening_rush");
        assert_eq!(hour_description(19), "evening_rush");
        assert_eq!(hour_description(20), "evening");
        assert_eq!(hour_description(21), "dusk");
        assert_eq!(hour_description(23), "night");
    }

    // ---- 12. Boundary hours (0 and 23) ----
    #[test]
    fn boundary_hours() {
        let m0 = model().get_multipliers(0);
        let m23 = model().get_multipliers(23);

        // Both are full night
        assert!((m0.crime_mult - 1.4).abs() < f32::EPSILON);
        assert!((m23.crime_mult - 1.4).abs() < f32::EPSILON);
        assert!((m0.commercial_revenue_mult - 0.2).abs() < f32::EPSILON);
        assert!((m23.commercial_revenue_mult - 0.2).abs() < f32::EPSILON);
    }

    // ---- 13. All hours produce valid multipliers (no NaN, all positive) ----
    #[test]
    fn all_hours_valid_multipliers() {
        let effects = model();
        for hour in 0..=23 {
            let m = effects.get_multipliers(hour);
            assert!(!m.crime_mult.is_nan(), "hour {} crime_mult is NaN", hour);
            assert!(!m.commercial_revenue_mult.is_nan(), "hour {} commercial NaN", hour);
            assert!(!m.power_demand_mult.is_nan(), "hour {} power NaN", hour);
            assert!(!m.noise_mult.is_nan(), "hour {} noise NaN", hour);
            assert!(!m.traffic_mult.is_nan(), "hour {} traffic NaN", hour);

            assert!(m.crime_mult > 0.0, "hour {} crime_mult not positive", hour);
            assert!(m.commercial_revenue_mult > 0.0, "hour {} commercial not positive", hour);
            assert!(m.power_demand_mult > 0.0, "hour {} power not positive", hour);
            assert!(m.noise_mult > 0.0, "hour {} noise not positive", hour);
            assert!(m.traffic_mult > 0.0, "hour {} traffic not positive", hour);
        }
    }

    // ---- 14. Default implementation is deterministic ----
    #[test]
    fn default_implementation_deterministic() {
        let a = model();
        let b = model();
        for hour in 0..=23 {
            let ma = a.get_multipliers(hour);
            let mb = b.get_multipliers(hour);
            assert!((ma.crime_mult - mb.crime_mult).abs() < f32::EPSILON);
            assert!((ma.commercial_revenue_mult - mb.commercial_revenue_mult).abs() < f32::EPSILON);
            assert!((ma.power_demand_mult - mb.power_demand_mult).abs() < f32::EPSILON);
            assert!((ma.noise_mult - mb.noise_mult).abs() < f32::EPSILON);
            assert!((ma.traffic_mult - mb.traffic_mult).abs() < f32::EPSILON);
        }
    }

    // ---- 15. Default modifiers are all 1.0 ----
    #[test]
    fn default_modifiers_all_one() {
        let m = DiurnalModifiers::default();
        assert!((m.crime_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.commercial_revenue_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.power_demand_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.noise_mult - 1.0).abs() < f32::EPSILON);
        assert!((m.traffic_mult - 1.0).abs() < f32::EPSILON);
    }

    // ---- 16. Model name correct ----
    #[test]
    fn model_name_correct() {
        let effects = model();
        assert_eq!(effects.name(), "default_diurnal");
    }

    // ---- 17. Hour clamped for out-of-range values ----
    #[test]
    fn hour_clamped_for_out_of_range() {
        // hour > 23 should be clamped and not panic
        let m = model().get_multipliers(255);
        // 255.min(23) = 23 => night
        assert!((m.crime_mult - 1.4).abs() < f32::EPSILON);

        let desc = hour_description(255);
        assert_eq!(desc, "night");
    }

    // ---- 18. Traffic transitions are smooth ----
    #[test]
    fn traffic_transitions_smooth() {
        let effects = model();

        // Hour 6 should be between normal and rush
        let m6 = effects.get_multipliers(6);
        assert!(
            m6.traffic_mult > NORMAL_TRAFFIC && m6.traffic_mult < RUSH_TRAFFIC,
            "hour 6 traffic_mult {} should be between {} and {}",
            m6.traffic_mult,
            NORMAL_TRAFFIC,
            RUSH_TRAFFIC
        );

        // Hour 10 should be between rush and normal
        let m10 = effects.get_multipliers(10);
        assert!(
            m10.traffic_mult > NORMAL_TRAFFIC && m10.traffic_mult < RUSH_TRAFFIC,
            "hour 10 traffic_mult {} should be between {} and {}",
            m10.traffic_mult,
            NORMAL_TRAFFIC,
            RUSH_TRAFFIC
        );

        // Hour 16 ramp into evening rush
        let m16 = effects.get_multipliers(16);
        assert!(
            m16.traffic_mult > NORMAL_TRAFFIC && m16.traffic_mult < RUSH_TRAFFIC,
            "hour 16 traffic_mult {} should be between {} and {}",
            m16.traffic_mult,
            NORMAL_TRAFFIC,
            RUSH_TRAFFIC
        );

        // Hour 20 ramp out of evening rush
        let m20 = effects.get_multipliers(20);
        assert!(
            m20.traffic_mult > NORMAL_TRAFFIC && m20.traffic_mult < RUSH_TRAFFIC,
            "hour 20 traffic_mult {} should be between {} and {}",
            m20.traffic_mult,
            NORMAL_TRAFFIC,
            RUSH_TRAFFIC
        );
    }

    // ---- 19. Midday has no traffic rush ----
    #[test]
    fn midday_normal_traffic() {
        let m = model().get_multipliers(12);
        assert!(
            (m.traffic_mult - 1.0).abs() < f32::EPSILON,
            "midday traffic should be 1.0, got {}",
            m.traffic_mult
        );
    }
}
