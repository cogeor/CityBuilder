//! City-level zone demand valves, mirroring SimCity RValve/CValve/IValve.
//!
//! `ZoneDemand` carries city-wide pressure for residential, commercial, and
//! industrial growth. Positive = more of that zone type needed; negative = excess.
//! `compute_zone_demand` re-derives demand each VALVERATE ticks from population,
//! employment capacity, and current tax rates.

use crate::core::world_vars::WorldVars;
use crate::core_types::ZoneType;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Maximum residential demand (±clamp).
pub const RES_DEMAND_MAX: i32 = 2000;
/// Maximum commercial / industrial demand (±clamp).
pub const CI_DEMAND_MAX: i32 = 1500;
/// Tax penalty per percentage point above 0 (residential/commercial/industrial).
pub const TAX_PENALTY_PER_PCT: i32 = 80;

// ─── ZoneDemand ──────────────────────────────────────────────────────────────

/// City-wide demand pressure per zone type.
///
/// Positive values signal a need for new development; negative values signal
/// oversupply. Values are clamped to a safe range each update.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ZoneDemand {
    /// Residential demand, clamped to `[-2000, +2000]`.
    pub residential: i32,
    /// Commercial demand, clamped to `[-1500, +1500]`.
    pub commercial: i32,
    /// Industrial demand, clamped to `[-1500, +1500]`.
    pub industrial: i32,
}

impl ZoneDemand {
    /// Maximum demand across all zones (used when demand is unconstrained).
    pub const FULL: ZoneDemand = ZoneDemand {
        residential: RES_DEMAND_MAX,
        commercial: CI_DEMAND_MAX,
        industrial: CI_DEMAND_MAX,
    };

    /// Returns the demand pressure for a specific zone type.
    /// Civic/Park/Transport always return max (demand is not gated).
    pub fn for_zone(&self, zone: ZoneType) -> i32 {
        match zone {
            ZoneType::Residential => self.residential,
            ZoneType::Commercial  => self.commercial,
            ZoneType::Industrial  => self.industrial,
            ZoneType::Civic | ZoneType::Park | ZoneType::Transport | ZoneType::None => {
                CI_DEMAND_MAX
            }
        }
    }

    /// Returns `true` if there is positive demand for the given zone type.
    pub fn has_demand_for(&self, zone: ZoneType) -> bool {
        self.for_zone(zone) > 0
    }
}

// ─── compute_zone_demand ─────────────────────────────────────────────────────

/// Derive updated zone demand from current city metrics.
///
/// Parameters
/// - `res_pop`: current residential population
/// - `res_cap`: current residential housing capacity
/// - `com_jobs`: current commercial job capacity
/// - `ind_jobs`: current industrial job capacity
/// - `tax_res_pct`: residential tax rate (0-20)
/// - `tax_com_pct`: commercial tax rate (0-20)
/// - `tax_ind_pct`: industrial tax rate (0-20)
/// - `prev`: previous demand (used for smooth delta clamping)
pub fn compute_zone_demand(
    res_pop:     u32,
    res_cap:     u32,
    com_jobs:    u32,
    ind_jobs:    u32,
    tax_res_pct: u8,
    tax_com_pct: u8,
    tax_ind_pct: u8,
    prev:        ZoneDemand,
) -> ZoneDemand {
    // ── Residential pressure ────────────────────────────────────────────
    // Positive when population exceeds housing capacity (housing deficit).
    let res_base: i32 = if res_cap == 0 {
        RES_DEMAND_MAX / 2
    } else {
        // Employment ratio proxy: population / capacity
        let ratio_x1000 = (res_pop as i64 * 1000 / res_cap.max(1) as i64) as i32;
        // Exceeds 1000 (ratio > 1.0) → positive pressure; below → negative
        (ratio_x1000 - 1000) * 2
    };
    let res_tax_pen: i32 = tax_res_pct as i32 * TAX_PENALTY_PER_PCT;
    let res_raw = (res_base - res_tax_pen).clamp(-RES_DEMAND_MAX, RES_DEMAND_MAX);

    // ── Commercial pressure ─────────────────────────────────────────────
    // Scales with population relative to commercial capacity.
    let total_pop = res_pop.max(1);
    let com_ratio_x1000 = (total_pop as i64 * 1000 / (com_jobs.max(1)) as i64) as i32;
    let com_base = ((com_ratio_x1000 - 1000) * 2).clamp(-CI_DEMAND_MAX, CI_DEMAND_MAX);
    let com_tax_pen: i32 = tax_com_pct as i32 * TAX_PENALTY_PER_PCT;
    let com_raw = (com_base - com_tax_pen).clamp(-CI_DEMAND_MAX, CI_DEMAND_MAX);

    // ── Industrial pressure ─────────────────────────────────────────────
    let ind_ratio_x1000 = (total_pop as i64 * 1000 / (ind_jobs.max(1)) as i64) as i32;
    let ind_base = ((ind_ratio_x1000 - 1000) * 2).clamp(-CI_DEMAND_MAX, CI_DEMAND_MAX);
    let ind_tax_pen: i32 = tax_ind_pct as i32 * TAX_PENALTY_PER_PCT;
    let ind_raw = (ind_base - ind_tax_pen).clamp(-CI_DEMAND_MAX, CI_DEMAND_MAX);

    // Smooth delta: move at most 200 points per update cycle.
    const MAX_DELTA: i32 = 200;
    let res = (prev.residential + (res_raw - prev.residential).clamp(-MAX_DELTA, MAX_DELTA))
        .clamp(-RES_DEMAND_MAX, RES_DEMAND_MAX);
    let com = (prev.commercial + (com_raw - prev.commercial).clamp(-MAX_DELTA, MAX_DELTA))
        .clamp(-CI_DEMAND_MAX, CI_DEMAND_MAX);
    let ind = (prev.industrial + (ind_raw - prev.industrial).clamp(-MAX_DELTA, MAX_DELTA))
        .clamp(-CI_DEMAND_MAX, CI_DEMAND_MAX);

    ZoneDemand { residential: res, commercial: com, industrial: ind }
}

// ─── AbandonmentState ─────────────────────────────────────────────────────────

/// Tracks abandonment countdown for a zone or building slot.
///
/// Increment `age_ticks` each tick when utilization is below
/// `WorldVars::abandon_util_threshold`. Reset to 0 when above threshold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AbandonmentState {
    /// Ticks spent below `abandon_util_threshold`. Zero when healthy.
    pub age_ticks: u32,
}

impl AbandonmentState {
    pub const fn new() -> Self { AbandonmentState { age_ticks: 0 } }

    /// Update state and return `true` if the slot should be marked abandoned.
    ///
    /// - `utilization`: fraction of capacity in use (0.0–1.0)
    /// - `ticks_per_day`: simulation ticks in one game-day (used to convert
    ///   `WorldVars::abandon_days` to ticks)
    pub fn tick(&mut self, utilization: f32, world_vars: &WorldVars, ticks_per_day: u32) -> bool {
        if utilization < world_vars.abandon_util_threshold {
            self.age_ticks += 1;
        } else {
            self.age_ticks = 0;
        }
        let threshold = (world_vars.abandon_days * ticks_per_day as f32) as u32;
        self.age_ticks >= threshold
    }
}

// ─── gate_growth ──────────────────────────────────────────────────────────────

/// Apply growth gating to a demand value based on current utilization.
///
/// Returns 0 if `utilization < world_vars.grow_min_threshold`, otherwise
/// scales `demand` by the job-housing ratio factor:
///   `scale = (jhr / world_vars.target_jobs_housing_ratio).clamp(0.0, 2.0)`
/// then converts back to i32 (clamped to original demand range).
///
/// This implements the design rule: zones do not grow when underutilised,
/// and grow faster when the job-housing balance matches the real-world target.
pub fn gate_growth(
    demand: i32,
    utilization: f32,
    jobs_housing_ratio: f32,
    world_vars: &WorldVars,
) -> i32 {
    if utilization < world_vars.grow_min_threshold {
        return 0;
    }
    let jhr_scale = (jobs_housing_ratio / world_vars.target_jobs_housing_ratio).clamp(0.0, 2.0);
    ((demand as f32 * jhr_scale) as i32).clamp(-RES_DEMAND_MAX, RES_DEMAND_MAX)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_prev() -> ZoneDemand {
        ZoneDemand::default()
    }

    #[test]
    fn residential_positive_when_overcrowded_low_tax() {
        // Pop = 2000, cap = 1000 → ratio 2.0 → positive pressure
        let d = compute_zone_demand(2000, 1000, 100, 100, 0, 5, 5, zero_prev());
        assert!(
            d.residential > 0,
            "Expected positive residential demand, got {}",
            d.residential
        );
    }

    #[test]
    fn residential_non_positive_when_unemployed_high_tax() {
        // Pop = 100, cap = 2000 → big surplus → negative
        let d = compute_zone_demand(100, 2000, 100, 100, 20, 5, 5, zero_prev());
        assert!(
            d.residential <= 0,
            "Expected residential demand <= 0 at max tax + surplus, got {}",
            d.residential
        );
    }

    #[test]
    fn residential_non_positive_at_max_tax() {
        // Equal pop/cap but 20% tax → heavy penalty
        let d = compute_zone_demand(1000, 1000, 100, 100, 20, 20, 20, zero_prev());
        assert!(
            d.residential <= 0,
            "Expected residential <= 0 at max tax, got {}",
            d.residential
        );
    }

    #[test]
    fn for_zone_returns_correct_slot() {
        let d = ZoneDemand { residential: 100, commercial: -50, industrial: 300 };
        assert_eq!(d.for_zone(ZoneType::Residential), 100);
        assert_eq!(d.for_zone(ZoneType::Commercial), -50);
        assert_eq!(d.for_zone(ZoneType::Industrial), 300);
        assert_eq!(d.for_zone(ZoneType::Civic), CI_DEMAND_MAX);
    }

    #[test]
    fn has_demand_for_positive_values() {
        let d = ZoneDemand { residential: 1, commercial: -1, industrial: 0 };
        assert!(d.has_demand_for(ZoneType::Residential));
        assert!(!d.has_demand_for(ZoneType::Commercial));
        assert!(!d.has_demand_for(ZoneType::Industrial));
    }

    #[test]
    fn smooth_delta_clamp() {
        let prev = ZoneDemand { residential: 0, commercial: 0, industrial: 0 };
        // Even if target would be very high, step is clamped to MAX_DELTA
        let d = compute_zone_demand(10000, 1, 1, 1, 0, 0, 0, prev);
        assert!(d.residential <= 200, "Step should be clamped to 200, got {}", d.residential);
    }

    #[test]
    fn gate_blocks_growth_below_threshold() {
        let wv = WorldVars::default(); // grow_min_threshold = 0.55
        // utilization 0.3 < 0.55 → demand zeroed
        assert_eq!(gate_growth(500, 0.3, 1.1, &wv), 0);
    }

    #[test]
    fn gate_allows_growth_above_threshold() {
        let wv = WorldVars::default();
        // utilization 0.7 >= 0.55, jhr == target → scale = 1.0 → demand unchanged
        let result = gate_growth(500, 0.7, 1.1, &wv);
        assert!(result > 0, "expected positive demand, got {}", result);
    }

    #[test]
    fn abandonment_triggers_after_threshold_ticks() {
        let wv = WorldVars::default(); // abandon_days=90, assume ticks_per_day=1
        let mut state = AbandonmentState::new();
        // tick 89 times at low utilization — not yet abandoned
        for _ in 0..89 {
            let abandoned = state.tick(0.1, &wv, 1);
            assert!(!abandoned);
        }
        // tick 90 — now abandoned
        let abandoned = state.tick(0.1, &wv, 1);
        assert!(abandoned, "should be abandoned after 90 ticks below threshold");
    }

    #[test]
    fn abandonment_resets_when_utilization_recovers() {
        let wv = WorldVars::default();
        let mut state = AbandonmentState::new();
        // Run 50 ticks below threshold
        for _ in 0..50 {
            state.tick(0.1, &wv, 1);
        }
        assert_eq!(state.age_ticks, 50);
        // Recover
        state.tick(0.9, &wv, 1);
        assert_eq!(state.age_ticks, 0, "age_ticks should reset on recovery");
    }
}
