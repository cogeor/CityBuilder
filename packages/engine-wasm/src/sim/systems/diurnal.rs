//! Day/night cycle gameplay effects with swappable trait.

/// Multipliers applied to simulation systems based on time of day.
/// All fields are basis points: 10000 = 1.0x, 5000 = 0.5x, 15000 = 1.5x.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiurnalModifiers {
    pub crime_mult: u16,
    pub commercial_revenue_mult: u16,
    pub power_demand_mult: u16,
    pub noise_mult: u16,
    pub traffic_mult: u16,
}

impl Default for DiurnalModifiers {
    fn default() -> Self {
        Self {
            crime_mult: 10_000,
            commercial_revenue_mult: 10_000,
            power_demand_mult: 10_000,
            noise_mult: 10_000,
            traffic_mult: 10_000,
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

/// Linear interpolation between two basis-point values.
///
/// `t` is a basis-point fraction: 0 = 0%, 10000 = 100%.
/// Intermediate arithmetic uses u32 to prevent overflow.
pub(crate) fn lerp_bp(a: u16, b: u16, t: u16) -> u16 {
    let a32 = a as u32;
    let b32 = b as u32;
    let t32 = t as u32;
    if b32 >= a32 {
        (a32 + (b32 - a32) * t32 / 10_000) as u16
    } else {
        (a32 - (a32 - b32) * t32 / 10_000) as u16
    }
}

/// Night-period target values.
const NIGHT_CRIME: u16 = 14_000;
const NIGHT_COMMERCIAL: u16 = 2_000;
const NIGHT_POWER: u16 = 8_000;
const NIGHT_NOISE: u16 = 3_000;

/// Day-period baseline values.
const DAY_CRIME: u16 = 10_000;
const DAY_COMMERCIAL: u16 = 10_000;
const DAY_POWER: u16 = 10_000;
const DAY_NOISE: u16 = 10_000;

/// Rush-hour traffic multiplier.
const RUSH_TRAFFIC: u16 = 15_000;
/// Normal traffic multiplier.
const NORMAL_TRAFFIC: u16 = 10_000;

impl IDiurnalEffects for DefaultDiurnalEffects {
    fn get_multipliers(&self, game_hour: u8) -> DiurnalModifiers {
        let hour = game_hour.min(23);

        // Determine crime, commercial, power, noise based on time period.
        //
        // Core night: 23:00 - 05:00 (full night values)
        // Transition to night: 21:00 - 22:59 (lerp day -> night)
        //   hour 21: t=0 (day), hour 22: t=5000
        // Transition to day: 05:00 - 06:59 (lerp night -> day)
        //   hour 05: t=0 (night), hour 06: t=5000
        // Core day: 07:00 - 20:00 (full day values)
        //
        // This gives us the night window 22:00-06:00 specified in requirements,
        // with smooth 2-hour transition ramps on each side.
        let (crime, commercial, power, noise) = match hour {
            // Core night: 23:00-04:59
            23 | 0..=4 => (NIGHT_CRIME, NIGHT_COMMERCIAL, NIGHT_POWER, NIGHT_NOISE),
            // Transition night -> day: 05:00-06:59
            5 => {
                let t: u16 = 0; // start of transition, still fully night
                (
                    lerp_bp(NIGHT_CRIME, DAY_CRIME, t),
                    lerp_bp(NIGHT_COMMERCIAL, DAY_COMMERCIAL, t),
                    lerp_bp(NIGHT_POWER, DAY_POWER, t),
                    lerp_bp(NIGHT_NOISE, DAY_NOISE, t),
                )
            }
            6 => {
                let t: u16 = 5_000;
                (
                    lerp_bp(NIGHT_CRIME, DAY_CRIME, t),
                    lerp_bp(NIGHT_COMMERCIAL, DAY_COMMERCIAL, t),
                    lerp_bp(NIGHT_POWER, DAY_POWER, t),
                    lerp_bp(NIGHT_NOISE, DAY_NOISE, t),
                )
            }
            // Core day: 07:00-20:59
            7..=20 => (DAY_CRIME, DAY_COMMERCIAL, DAY_POWER, DAY_NOISE),
            // Transition day -> night: 21:00-22:59
            21 => {
                let t: u16 = 0; // start of transition, still fully day
                (
                    lerp_bp(DAY_CRIME, NIGHT_CRIME, t),
                    lerp_bp(DAY_COMMERCIAL, NIGHT_COMMERCIAL, t),
                    lerp_bp(DAY_POWER, NIGHT_POWER, t),
                    lerp_bp(DAY_NOISE, NIGHT_NOISE, t),
                )
            }
            22 => {
                let t: u16 = 5_000;
                (
                    lerp_bp(DAY_CRIME, NIGHT_CRIME, t),
                    lerp_bp(DAY_COMMERCIAL, NIGHT_COMMERCIAL, t),
                    lerp_bp(DAY_POWER, NIGHT_POWER, t),
                    lerp_bp(DAY_NOISE, NIGHT_NOISE, t),
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
            6 => lerp_bp(NORMAL_TRAFFIC, RUSH_TRAFFIC, 5_000),
            // Transition out of morning rush (hour 10)
            10 => lerp_bp(RUSH_TRAFFIC, NORMAL_TRAFFIC, 5_000),
            // Transition into evening rush (hour 16)
            16 => lerp_bp(NORMAL_TRAFFIC, RUSH_TRAFFIC, 5_000),
            // Transition out of evening rush (hour 20)
            20 => lerp_bp(RUSH_TRAFFIC, NORMAL_TRAFFIC, 5_000),
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
        assert_eq!(m.crime_mult, 14_000);
        assert_eq!(m.commercial_revenue_mult, 2_000);
        assert_eq!(m.power_demand_mult, 8_000);
        assert_eq!(m.noise_mult, 3_000);
    }

    // ---- 2. Noon modifiers correct ----
    #[test]
    fn noon_modifiers_correct() {
        let m = model().get_multipliers(12);
        assert_eq!(m.crime_mult, 10_000);
        assert_eq!(m.commercial_revenue_mult, 10_000);
        assert_eq!(m.power_demand_mult, 10_000);
        assert_eq!(m.noise_mult, 10_000);
        assert_eq!(m.traffic_mult, 10_000);
    }

    // ---- 3. Rush hour traffic multiplier (morning) ----
    #[test]
    fn rush_hour_traffic_morning() {
        for hour in 7..=9 {
            let m = model().get_multipliers(hour);
            assert_eq!(
                m.traffic_mult, 15_000,
                "hour {} should have traffic_mult=15000, got {}",
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
            assert_eq!(
                m.traffic_mult, 15_000,
                "hour {} should have traffic_mult=15000, got {}",
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
            assert_eq!(
                m.crime_mult, 14_000,
                "hour {} should have crime_mult=14000, got {}",
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
            assert_eq!(
                m.commercial_revenue_mult, 2_000,
                "hour {} should have commercial_revenue_mult=2000, got {}",
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
            assert_eq!(
                m.power_demand_mult, 8_000,
                "hour {} should have power_demand_mult=8000, got {}",
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
            assert_eq!(
                m.noise_mult, 3_000,
                "hour {} should have noise_mult=3000, got {}",
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

        // Hour 5: still fully night values (t=0 in night->day transition)
        assert_eq!(m5.crime_mult, NIGHT_CRIME);

        // Hour 6: midpoint between night and day
        let expected_crime = lerp_bp(NIGHT_CRIME, DAY_CRIME, 5_000);
        assert_eq!(
            m6.crime_mult,
            expected_crime,
            "hour 6 crime_mult should be {}, got {}",
            expected_crime,
            m6.crime_mult
        );

        let expected_commercial = lerp_bp(NIGHT_COMMERCIAL, DAY_COMMERCIAL, 5_000);
        assert_eq!(
            m6.commercial_revenue_mult,
            expected_commercial,
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

        // Hour 21: still fully day values (t=0 in day->night transition)
        assert_eq!(m21.crime_mult, DAY_CRIME);

        // Hour 22: midpoint between day and night
        let expected_crime = lerp_bp(DAY_CRIME, NIGHT_CRIME, 5_000);
        assert_eq!(
            m22.crime_mult,
            expected_crime,
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
        assert_eq!(m0.crime_mult, 14_000);
        assert_eq!(m23.crime_mult, 14_000);
        assert_eq!(m0.commercial_revenue_mult, 2_000);
        assert_eq!(m23.commercial_revenue_mult, 2_000);
    }

    // ---- 13. All hours produce valid multipliers (all positive) ----
    #[test]
    fn all_hours_valid_multipliers() {
        let effects = model();
        for hour in 0..=23_u8 {
            let m = effects.get_multipliers(hour);
            assert!(m.crime_mult > 0, "hour {} crime_mult is zero", hour);
            assert!(m.commercial_revenue_mult > 0, "hour {} commercial is zero", hour);
            assert!(m.power_demand_mult > 0, "hour {} power is zero", hour);
            assert!(m.noise_mult > 0, "hour {} noise is zero", hour);
            assert!(m.traffic_mult > 0, "hour {} traffic is zero", hour);
        }
    }

    // ---- 14. Default implementation is deterministic ----
    #[test]
    fn default_implementation_deterministic() {
        let a = model();
        let b = model();
        for hour in 0..=23_u8 {
            assert_eq!(a.get_multipliers(hour), b.get_multipliers(hour));
        }
    }

    // ---- 15. Default modifiers are all 10000 (1.0x) ----
    #[test]
    fn default_modifiers_all_one() {
        let m = DiurnalModifiers::default();
        assert_eq!(m.crime_mult, 10_000);
        assert_eq!(m.commercial_revenue_mult, 10_000);
        assert_eq!(m.power_demand_mult, 10_000);
        assert_eq!(m.noise_mult, 10_000);
        assert_eq!(m.traffic_mult, 10_000);
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
        assert_eq!(m.crime_mult, 14_000);

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
        assert_eq!(
            m.traffic_mult, 10_000,
            "midday traffic should be 10000, got {}",
            m.traffic_mult
        );
    }
}
