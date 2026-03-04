//! Seasonal weather effects with swappable trait.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring = 0,
    Summer = 1,
    Autumn = 2,
    Winter = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClimateZone {
    Temperate,    // 4 seasons
    Tropical,     // 2 seasons (wet/dry)
    Arid,         // mild seasons
}

#[derive(Debug, Clone)]
pub struct SeasonModifiers {
    pub power_demand_mult: f32,
    pub road_maintenance_mult: f32,
    pub happiness_mult: f32,
    pub growth_mult: f32,
    pub commercial_mult: f32,
    pub park_effectiveness_mult: f32,
}

impl Default for SeasonModifiers {
    fn default() -> Self {
        Self {
            power_demand_mult: 1.0,
            road_maintenance_mult: 1.0,
            happiness_mult: 1.0,
            growth_mult: 1.0,
            commercial_mult: 1.0,
            park_effectiveness_mult: 1.0,
        }
    }
}

/// Trait for pluggable seasonal effects.
pub trait ISeasonalEffects {
    fn get_season(&self, tick: u64, ticks_per_day: u64) -> Season;
    fn get_modifiers(&self, season: Season) -> SeasonModifiers;
    fn name(&self) -> &str;
}

/// Default temperate 4-season model.
pub struct TemperateSeasons {
    pub days_per_season: u32,
}

impl Default for TemperateSeasons {
    fn default() -> Self {
        Self { days_per_season: 90 }  // ~360 day year
    }
}

impl ISeasonalEffects for TemperateSeasons {
    fn get_season(&self, tick: u64, ticks_per_day: u64) -> Season {
        let day = tick / ticks_per_day.max(1);
        let season_day = (day % (self.days_per_season as u64 * 4)) / self.days_per_season as u64;
        match season_day {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        }
    }

    fn get_modifiers(&self, season: Season) -> SeasonModifiers {
        match season {
            Season::Spring => SeasonModifiers {
                growth_mult: 1.1,
                park_effectiveness_mult: 1.2,
                ..Default::default()
            },
            Season::Summer => SeasonModifiers {
                commercial_mult: 1.15,
                park_effectiveness_mult: 1.3,
                happiness_mult: 1.05,
                ..Default::default()
            },
            Season::Autumn => SeasonModifiers {
                road_maintenance_mult: 1.1,
                ..Default::default()
            },
            Season::Winter => SeasonModifiers {
                power_demand_mult: 1.3,
                road_maintenance_mult: 1.2,
                happiness_mult: 0.95,
                growth_mult: 0.8,
                park_effectiveness_mult: 0.5,
                ..Default::default()
            },
        }
    }

    fn name(&self) -> &str { "temperate_4season" }
}

/// Tropical 2-season model (wet/dry mapped to Summer/Winter).
pub struct TropicalSeasons {
    pub days_per_season: u32,
}

impl Default for TropicalSeasons {
    fn default() -> Self {
        Self { days_per_season: 180 }
    }
}

impl ISeasonalEffects for TropicalSeasons {
    fn get_season(&self, tick: u64, ticks_per_day: u64) -> Season {
        let day = tick / ticks_per_day.max(1);
        if (day % (self.days_per_season as u64 * 2)) < self.days_per_season as u64 {
            Season::Summer  // dry season
        } else {
            Season::Winter  // wet season
        }
    }

    fn get_modifiers(&self, season: Season) -> SeasonModifiers {
        match season {
            Season::Summer => SeasonModifiers {
                commercial_mult: 1.1,
                happiness_mult: 1.05,
                ..Default::default()
            },
            Season::Winter => SeasonModifiers {
                road_maintenance_mult: 1.15,
                happiness_mult: 0.98,
                ..Default::default()
            },
            _ => SeasonModifiers::default(),
        }
    }

    fn name(&self) -> &str { "tropical_2season" }
}

/// Get current season name.
pub fn season_name(season: Season) -> &'static str {
    match season {
        Season::Spring => "Spring",
        Season::Summer => "Summer",
        Season::Autumn => "Autumn",
        Season::Winter => "Winter",
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Test 1: Temperate spring at tick 0 ─────────────────────────────

    #[test]
    fn temperate_spring_at_tick_zero() {
        let model = TemperateSeasons::default();
        let season = model.get_season(0, 100);
        assert_eq!(season, Season::Spring);
    }

    // ─── Test 2: Temperate summer at appropriate tick ───────────────────

    #[test]
    fn temperate_summer_at_appropriate_tick() {
        let model = TemperateSeasons::default();
        // 90 days per season, 100 ticks per day => summer starts at tick 9000
        let season = model.get_season(9000, 100);
        assert_eq!(season, Season::Summer);
    }

    // ─── Test 3: Temperate winter modifiers correct (power +30%) ────────

    #[test]
    fn temperate_winter_modifiers_correct() {
        let model = TemperateSeasons::default();
        let mods = model.get_modifiers(Season::Winter);
        assert!((mods.power_demand_mult - 1.3).abs() < f32::EPSILON);
        assert!((mods.road_maintenance_mult - 1.2).abs() < f32::EPSILON);
        assert!((mods.happiness_mult - 0.95).abs() < f32::EPSILON);
        assert!((mods.growth_mult - 0.8).abs() < f32::EPSILON);
        assert!((mods.park_effectiveness_mult - 0.5).abs() < f32::EPSILON);
    }

    // ─── Test 4: Temperate cycles through all 4 seasons ─────────────────

    #[test]
    fn temperate_cycles_through_all_four_seasons() {
        let model = TemperateSeasons::default();
        let ticks_per_day: u64 = 100;
        let days_per_season = model.days_per_season as u64;

        let spring = model.get_season(0, ticks_per_day);
        let summer = model.get_season(days_per_season * ticks_per_day, ticks_per_day);
        let autumn = model.get_season(days_per_season * 2 * ticks_per_day, ticks_per_day);
        let winter = model.get_season(days_per_season * 3 * ticks_per_day, ticks_per_day);

        assert_eq!(spring, Season::Spring);
        assert_eq!(summer, Season::Summer);
        assert_eq!(autumn, Season::Autumn);
        assert_eq!(winter, Season::Winter);

        // After a full year, back to spring
        let spring_again = model.get_season(days_per_season * 4 * ticks_per_day, ticks_per_day);
        assert_eq!(spring_again, Season::Spring);
    }

    // ─── Test 5: Tropical alternates between 2 seasons ──────────────────

    #[test]
    fn tropical_alternates_between_two_seasons() {
        let model = TropicalSeasons::default();
        let ticks_per_day: u64 = 100;
        let days_per_season = model.days_per_season as u64;

        let dry = model.get_season(0, ticks_per_day);
        let wet = model.get_season(days_per_season * ticks_per_day, ticks_per_day);
        let dry_again = model.get_season(days_per_season * 2 * ticks_per_day, ticks_per_day);

        assert_eq!(dry, Season::Summer);     // dry season mapped to Summer
        assert_eq!(wet, Season::Winter);     // wet season mapped to Winter
        assert_eq!(dry_again, Season::Summer);
    }

    // ─── Test 6: Winter power demand multiplier > 1.0 ───────────────────

    #[test]
    fn winter_power_demand_above_one() {
        let model = TemperateSeasons::default();
        let mods = model.get_modifiers(Season::Winter);
        assert!(mods.power_demand_mult > 1.0);
    }

    // ─── Test 7: Summer commercial bonus ────────────────────────────────

    #[test]
    fn summer_commercial_bonus() {
        let model = TemperateSeasons::default();
        let mods = model.get_modifiers(Season::Summer);
        assert!(mods.commercial_mult > 1.0);
        assert!((mods.commercial_mult - 1.15).abs() < f32::EPSILON);
    }

    // ─── Test 8: Default modifiers are all 1.0 ─────────────────────────

    #[test]
    fn default_modifiers_are_all_one() {
        let mods = SeasonModifiers::default();
        assert!((mods.power_demand_mult - 1.0).abs() < f32::EPSILON);
        assert!((mods.road_maintenance_mult - 1.0).abs() < f32::EPSILON);
        assert!((mods.happiness_mult - 1.0).abs() < f32::EPSILON);
        assert!((mods.growth_mult - 1.0).abs() < f32::EPSILON);
        assert!((mods.commercial_mult - 1.0).abs() < f32::EPSILON);
        assert!((mods.park_effectiveness_mult - 1.0).abs() < f32::EPSILON);
    }

    // ─── Test 9: season_name returns correct strings ────────────────────

    #[test]
    fn season_name_returns_correct_strings() {
        assert_eq!(season_name(Season::Spring), "Spring");
        assert_eq!(season_name(Season::Summer), "Summer");
        assert_eq!(season_name(Season::Autumn), "Autumn");
        assert_eq!(season_name(Season::Winter), "Winter");
    }

    // ─── Test 10: Temperate model name ──────────────────────────────────

    #[test]
    fn temperate_model_name() {
        let model = TemperateSeasons::default();
        assert_eq!(model.name(), "temperate_4season");
    }

    // ─── Test 11: Tropical model name ───────────────────────────────────

    #[test]
    fn tropical_model_name() {
        let model = TropicalSeasons::default();
        assert_eq!(model.name(), "tropical_2season");
    }

    // ─── Test 12: get_season handles ticks_per_day = 0 gracefully ───────

    #[test]
    fn get_season_handles_zero_ticks_per_day() {
        let temperate = TemperateSeasons::default();
        // Should not panic; ticks_per_day.max(1) prevents division by zero
        let season = temperate.get_season(1000, 0);
        // With ticks_per_day=0, max(1) makes it 1, so day=1000
        // 1000 % 360 = 280, 280 / 90 = 3 => Winter
        assert_eq!(season, Season::Winter);

        let tropical = TropicalSeasons::default();
        let season = tropical.get_season(1000, 0);
        // day=1000, 1000 % 360 = 280, 280 < 180 is false => Winter (wet)
        assert_eq!(season, Season::Winter);
    }

    // ─── Test 13: Tropical wet season modifiers ─────────────────────────

    #[test]
    fn tropical_wet_season_modifiers() {
        let model = TropicalSeasons::default();
        let mods = model.get_modifiers(Season::Winter);
        assert!((mods.road_maintenance_mult - 1.15).abs() < f32::EPSILON);
        assert!((mods.happiness_mult - 0.98).abs() < f32::EPSILON);
    }

    // ─── Test 14: Tropical spring/autumn returns defaults ───────────────

    #[test]
    fn tropical_unused_seasons_return_defaults() {
        let model = TropicalSeasons::default();
        let spring_mods = model.get_modifiers(Season::Spring);
        let autumn_mods = model.get_modifiers(Season::Autumn);

        assert!((spring_mods.power_demand_mult - 1.0).abs() < f32::EPSILON);
        assert!((autumn_mods.power_demand_mult - 1.0).abs() < f32::EPSILON);
    }

    // ─── Test 15: ClimateZone enum values ───────────────────────────────

    #[test]
    fn climate_zone_enum_values() {
        let t = ClimateZone::Temperate;
        let tr = ClimateZone::Tropical;
        let a = ClimateZone::Arid;
        assert_eq!(t, ClimateZone::Temperate);
        assert_eq!(tr, ClimateZone::Tropical);
        assert_eq!(a, ClimateZone::Arid);
        assert_ne!(t, tr);
    }
}
