use city_core::Tick;
use crate::types::*;

#[inline]
pub const fn ticks_to_game_seconds(ticks: Tick) -> u64 {
    ticks * GAME_SECONDS_PER_TICK_NUM as u64 / GAME_SECONDS_PER_TICK_DEN as u64
}

#[inline]
pub const fn ticks_to_game_minutes(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_MINUTE
}

#[inline]
pub const fn ticks_to_game_hours(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_HOUR
}

#[inline]
pub const fn ticks_to_game_days(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_DAY
}

#[inline]
pub const fn ticks_to_game_months(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_MONTH
}

#[inline]
pub const fn ticks_to_game_years(ticks: Tick) -> u64 {
    ticks / TICKS_PER_GAME_YEAR
}

#[inline]
pub const fn day_tick(tick: Tick) -> u64 {
    tick % TICKS_PER_GAME_DAY
}

#[inline]
pub const fn game_hour(tick: Tick) -> u32 {
    (day_tick(tick) / TICKS_PER_GAME_HOUR) as u32
}

#[inline]
pub const fn game_minute(tick: Tick) -> u32 {
    ((day_tick(tick) % TICKS_PER_GAME_HOUR) / TICKS_PER_GAME_MINUTE) as u32
}

#[inline]
pub const fn is_hour_boundary(tick: Tick) -> bool {
    day_tick(tick) % TICKS_PER_GAME_HOUR == 0
}

#[inline]
pub const fn is_day_boundary(tick: Tick) -> bool {
    tick % TICKS_PER_GAME_DAY == 0
}

#[inline]
pub const fn is_month_boundary(tick: Tick) -> bool {
    tick % TICKS_PER_GAME_MONTH == 0
}

#[inline]
pub const fn real_seconds_for_ticks(ticks: Tick, speed_multiplier: u32) -> u64 {
    ticks / (SIM_TICKS_PER_REAL_SECOND * speed_multiplier) as u64
}

pub fn format_time(tick: Tick) -> String {
    let h = game_hour(tick);
    let m = game_minute(tick);
    format!("{:02}:{:02}", h, m)
}

pub fn format_day(tick: Tick) -> String {
    format!("Day {}", ticks_to_game_days(tick) + 1)
}

#[inline]
pub const fn cents_per_tick_to_per_month(rate: MoneyCents) -> MoneyCents {
    rate * TICKS_PER_GAME_MONTH as i64
}

#[inline]
pub const fn cents_per_tick_to_per_year(rate: MoneyCents) -> MoneyCents {
    rate * TICKS_PER_GAME_YEAR as i64
}

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
        assert_eq!(ticks_to_game_seconds(1), 1);
        assert_eq!(ticks_to_game_seconds(5), 6);
        assert_eq!(ticks_to_game_seconds(50), 60);
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
        assert_eq!(game_hour(36_000), 12);
        assert_eq!(game_hour(69_000), 23);
        assert_eq!(game_hour(72_000), 0);
    }

    #[test]
    fn game_minute_values() {
        assert_eq!(game_minute(0), 0);
        assert_eq!(game_minute(50), 1);
        assert_eq!(game_minute(2_950), 59);
        assert_eq!(game_minute(3_000), 0);
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
        assert_eq!(real_seconds_for_ticks(72_000, 1), 3_600);
        assert_eq!(real_seconds_for_ticks(72_000, 2), 1_800);
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
        assert_eq!(cents_per_tick_to_per_month(10), 21_600_000);
        assert_eq!(cents_per_tick_to_per_year(10), 259_200_000);
        assert_eq!(cents_per_tick_to_per_day(10), 720_000);
    }
}
