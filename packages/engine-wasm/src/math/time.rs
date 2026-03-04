//! Time model helpers for the simulation engine.
//!
//! Single source of truth: 1 real hour = 1 in-game day.
//! Fixed 20 Hz tick rate, each tick = 1.2 game-seconds.
//!
//! All constants are re-exported from core_types. This module provides
//! conversion helpers that operate on Tick values.

use crate::core_types::*;

/// Convert ticks to game-seconds (integer result, truncated).
/// Formula: ticks * 12 / 10
#[inline]
pub const fn ticks_to_game_seconds(ticks: Tick) -> u64 {
    ticks * GAME_SECONDS_PER_TICK_NUM as u64 / GAME_SECONDS_PER_TICK_DEN as u64
}

/// Convert ticks to game-minutes (integer, truncated).
#[inline]
pub const fn ticks_to_game_minutes(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_MINUTE
}

/// Convert ticks to game-hours (integer, truncated).
#[inline]
pub const fn ticks_to_game_hours(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_HOUR
}

/// Convert ticks to game-days (integer, truncated).
#[inline]
pub const fn ticks_to_game_days(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_DAY
}

/// Convert ticks to game-months (integer, truncated).
#[inline]
pub const fn ticks_to_game_months(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_MONTH
}

/// Convert ticks to game-years (integer, truncated).
#[inline]
pub const fn ticks_to_game_years(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_YEAR
}

/// Get the current day-tick (position within the current day, 0..71999).
#[inline]
pub const fn day_tick(tick: Tick) -> u64 {
    tick % TICKS_PER_GAME_DAY
}

/// Get the current game-hour (0..23) from a tick counter.
#[inline]
pub const fn game_hour(tick: Tick) -> u32 {
    (day_tick(tick) / TICKS_PER_GAME_HOUR) as u32
}

/// Get the current game-minute within the hour (0..59).
#[inline]
pub const fn game_minute(tick: Tick) -> u32 {
    ((day_tick(tick) % TICKS_PER_GAME_HOUR) / TICKS_PER_GAME_MINUTE) as u32
}

/// Returns true if this tick is a game-hour boundary (for lighting recomputation).
#[inline]
pub const fn is_hour_boundary(tick: Tick) -> bool {
    day_tick(tick) % TICKS_PER_GAME_HOUR == 0
}

/// Returns true if this tick is a game-day boundary.
#[inline]
pub const fn is_day_boundary(tick: Tick) -> bool {
    tick % TICKS_PER_GAME_DAY == 0
}

/// Returns true if this tick is a game-month boundary.
#[inline]
pub const fn is_month_boundary(tick: Tick) -> bool {
    tick % TICKS_PER_GAME_MONTH == 0
}

/// Compute real-time seconds for a given number of ticks at a speed multiplier.
/// speed_multiplier: 1 = normal (20 ticks/sec), 2 = fast, 4 = ultra.
#[inline]
pub const fn real_seconds_for_ticks(ticks: Tick, speed_multiplier: u32) -> u64 {
    ticks / (SIM_TICKS_PER_REAL_SECOND * speed_multiplier) as u64
}

/// Format a tick as "HH:MM" for UI display.
pub fn format_time(tick: Tick) -> String {
    let h = game_hour(tick);
    let m = game_minute(tick);
    format!("{:02}:{:02}", h, m)
}

/// Format a tick as "Day N" for UI display.
pub fn format_day(tick: Tick) -> String {
    format!("Day {}", ticks_to_game_days(tick) + 1)
}

/// Convert a cents-per-tick rate to display as per-month.
#[inline]
pub const fn cents_per_tick_to_per_month(rate: MoneyCents) -> MoneyCents {
    rate * TICKS_PER_GAME_MONTH as i64
}

/// Convert a cents-per-tick rate to display as per-year.
#[inline]
pub const fn cents_per_tick_to_per_year(rate: MoneyCents) -> MoneyCents {
    rate * TICKS_PER_GAME_YEAR as i64
}

/// Convert a cents-per-tick rate to display as per-day.
#[inline]
pub const fn cents_per_tick_to_per_day(rate: MoneyCents) -> MoneyCents {
    rate * TICKS_PER_GAME_DAY as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_to_day_mapping() {
        assert_eq!(ticks_to_game_days(72_000), 1);
        assert_eq!(ticks_to_game_days(0), 0);
        assert_eq!(ticks_to_game_days(71_999), 0);
    }

    #[test]
    fn tick_to_hour_mapping() {
        assert_eq!(ticks_to_game_hours(3_000), 1);
        assert_eq!(ticks_to_game_hours(6_000), 2);
        assert_eq!(ticks_to_game_hours(72_000), 24);
    }

    #[test]
    fn tick_to_minute_mapping() {
        assert_eq!(ticks_to_game_minutes(50), 1);
        assert_eq!(ticks_to_game_minutes(100), 2);
        assert_eq!(ticks_to_game_minutes(3_000), 60);
    }

    #[test]
    fn game_seconds_conversion() {
        // 1 tick = 1.2 game seconds
        assert_eq!(ticks_to_game_seconds(1), 1); // truncated: 1.2 -> 1
        assert_eq!(ticks_to_game_seconds(5), 6); // 5 * 1.2 = 6.0
        assert_eq!(ticks_to_game_seconds(50), 60); // 50 * 1.2 = 60 = 1 min
    }

    #[test]
    fn day_tick_wraps() {
        assert_eq!(day_tick(0), 0);
        assert_eq!(day_tick(72_000), 0);
        assert_eq!(day_tick(72_001), 1);
        assert_eq!(day_tick(144_000), 0);
    }

    #[test]
    fn game_hour_values() {
        assert_eq!(game_hour(0), 0);
        assert_eq!(game_hour(3_000), 1);
        assert_eq!(game_hour(36_000), 12); // noon
        assert_eq!(game_hour(69_000), 23);
        assert_eq!(game_hour(72_000), 0); // midnight again
    }

    #[test]
    fn game_minute_values() {
        assert_eq!(game_minute(0), 0);
        assert_eq!(game_minute(50), 1);
        assert_eq!(game_minute(2_950), 59);
        assert_eq!(game_minute(3_000), 0); // hour boundary
    }

    #[test]
    fn hour_boundary_detection() {
        assert!(is_hour_boundary(0));
        assert!(is_hour_boundary(3_000));
        assert!(is_hour_boundary(6_000));
        assert!(!is_hour_boundary(1));
        assert!(!is_hour_boundary(3_001));
    }

    #[test]
    fn day_boundary_detection() {
        assert!(is_day_boundary(0));
        assert!(is_day_boundary(72_000));
        assert!(!is_day_boundary(1));
    }

    #[test]
    fn month_boundary_detection() {
        assert!(is_month_boundary(0));
        assert!(is_month_boundary(2_160_000));
        assert!(!is_month_boundary(1));
    }

    #[test]
    fn real_time_calculation() {
        // At 1x speed: 72000 ticks / 20 = 3600 real seconds = 1 hour
        assert_eq!(real_seconds_for_ticks(72_000, 1), 3_600);
        // At 2x speed: 72000 / 40 = 1800 seconds
        assert_eq!(real_seconds_for_ticks(72_000, 2), 1_800);
        // At 4x speed: 72000 / 80 = 900 seconds
        assert_eq!(real_seconds_for_ticks(72_000, 4), 900);
    }

    #[test]
    fn format_time_display() {
        assert_eq!(format_time(0), "00:00");
        assert_eq!(format_time(3_000), "01:00");
        assert_eq!(format_time(3_050), "01:01");
        assert_eq!(format_time(36_000), "12:00");
    }

    #[test]
    fn budget_display_conversion() {
        // 10 cents/tick -> per month
        assert_eq!(cents_per_tick_to_per_month(10), 21_600_000);
        // 10 cents/tick -> per year
        assert_eq!(cents_per_tick_to_per_year(10), 259_200_000);
        // 10 cents/tick -> per day
        assert_eq!(cents_per_tick_to_per_day(10), 720_000);
    }
}
