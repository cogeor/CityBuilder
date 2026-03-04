//! Scheduling strategy trait and implementations.
//!
//! Controls how frequently expensive simulation systems run and what fraction
//! of the map is scanned each tick. Two built-in strategies are provided:
//!
//! - [`FixedStrategy`]: constant scan fraction with round-robin system activation.
//! - [`AdaptiveStrategy`]: adjusts scan fraction based on entity count.

use crate::core_types::Tick;

// ─── Trait ──────────────────────────────────────────────────────────────────

/// Strategy for how frequently expensive systems run.
pub trait SchedulingStrategy {
    /// Returns the fraction of the map to scan this tick (0.0-1.0).
    fn scan_fraction(&self, tick: Tick) -> f32;

    /// Returns whether an expensive system should run this tick.
    fn should_run_expensive(&self, tick: Tick, system_id: u8) -> bool;

    /// Returns the name of this strategy.
    fn name(&self) -> &str;
}

// ─── FixedStrategy ──────────────────────────────────────────────────────────

/// Fixed scheduling: always same fraction, round-robin system activation.
///
/// Every tick returns the same scan fraction. Expensive systems are activated
/// in a round-robin pattern: system `system_id` runs when
/// `tick % num_systems == system_id`.
#[derive(Debug, Clone)]
pub struct FixedStrategy {
    scan_fraction: f32,
    num_systems: u8,
}

impl FixedStrategy {
    /// Create a new fixed strategy.
    ///
    /// `scan_fraction` is clamped to `[0.0, 1.0]`.
    /// If `num_systems` is 0, `should_run_expensive` always returns false.
    pub fn new(scan_fraction: f32, num_systems: u8) -> Self {
        Self {
            scan_fraction: scan_fraction.clamp(0.0, 1.0),
            num_systems,
        }
    }
}

impl SchedulingStrategy for FixedStrategy {
    #[inline]
    fn scan_fraction(&self, _tick: Tick) -> f32 {
        self.scan_fraction
    }

    #[inline]
    fn should_run_expensive(&self, tick: Tick, system_id: u8) -> bool {
        if self.num_systems == 0 {
            return false;
        }
        (tick % self.num_systems as u64) == system_id as u64
    }

    #[inline]
    fn name(&self) -> &str {
        "fixed"
    }
}

// ─── AdaptiveStrategy ───────────────────────────────────────────────────────

/// Adaptive scheduling: adjusts scan fraction based on entity count.
///
/// When the entity count is at or below `entity_threshold`, the full
/// `base_fraction` is used. Above the threshold the fraction is scaled down
/// proportionally: `base_fraction * (threshold / entity_count)`.
///
/// Expensive system activation uses the same round-robin scheme as
/// [`FixedStrategy`].
#[derive(Debug, Clone)]
pub struct AdaptiveStrategy {
    base_fraction: f32,
    entity_threshold: u32,
    num_systems: u8,
}

impl AdaptiveStrategy {
    /// Create a new adaptive strategy.
    ///
    /// `base_fraction` is clamped to `[0.0, 1.0]`.
    /// `entity_threshold` is the entity count below which the base fraction
    /// is used unmodified.
    pub fn new(base_fraction: f32, entity_threshold: u32, num_systems: u8) -> Self {
        Self {
            base_fraction: base_fraction.clamp(0.0, 1.0),
            entity_threshold,
            num_systems,
        }
    }

    /// Compute scan fraction based on current entity count.
    ///
    /// Returns `base_fraction` when `entity_count <= entity_threshold`.
    /// Above the threshold, returns `base_fraction * (threshold / entity_count)`,
    /// clamped to `[0.0, 1.0]`.
    pub fn compute_fraction(&self, entity_count: u32) -> f32 {
        if entity_count == 0 || entity_count <= self.entity_threshold {
            return self.base_fraction;
        }
        let ratio = self.entity_threshold as f32 / entity_count as f32;
        (self.base_fraction * ratio).clamp(0.0, 1.0)
    }
}

impl SchedulingStrategy for AdaptiveStrategy {
    /// Returns the base scan fraction.
    ///
    /// Note: to get the entity-count-adjusted fraction, call
    /// [`compute_fraction`](AdaptiveStrategy::compute_fraction) directly.
    #[inline]
    fn scan_fraction(&self, _tick: Tick) -> f32 {
        self.base_fraction
    }

    #[inline]
    fn should_run_expensive(&self, tick: Tick, system_id: u8) -> bool {
        if self.num_systems == 0 {
            return false;
        }
        (tick % self.num_systems as u64) == system_id as u64
    }

    #[inline]
    fn name(&self) -> &str {
        "adaptive"
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── FixedStrategy ───────────────────────────────────────────────────

    #[test]
    fn fixed_returns_constant_scan_fraction() {
        let s = FixedStrategy::new(0.125, 4);
        // Fraction should be the same regardless of tick
        assert_eq!(s.scan_fraction(0), 0.125);
        assert_eq!(s.scan_fraction(1), 0.125);
        assert_eq!(s.scan_fraction(100), 0.125);
        assert_eq!(s.scan_fraction(u64::MAX), 0.125);
    }

    #[test]
    fn fixed_round_robins_expensive_systems() {
        let s = FixedStrategy::new(0.125, 4);

        // With 4 systems, system_id 0 runs at ticks 0,4,8,...
        assert!(s.should_run_expensive(0, 0));
        assert!(!s.should_run_expensive(0, 1));
        assert!(!s.should_run_expensive(0, 2));
        assert!(!s.should_run_expensive(0, 3));

        assert!(!s.should_run_expensive(1, 0));
        assert!(s.should_run_expensive(1, 1));

        assert!(!s.should_run_expensive(2, 0));
        assert!(s.should_run_expensive(2, 2));

        assert!(!s.should_run_expensive(3, 0));
        assert!(s.should_run_expensive(3, 3));

        // Cycle repeats
        assert!(s.should_run_expensive(4, 0));
        assert!(s.should_run_expensive(5, 1));
    }

    #[test]
    fn fixed_name() {
        let s = FixedStrategy::new(0.5, 2);
        assert_eq!(s.name(), "fixed");
    }

    #[test]
    fn fixed_clamps_fraction_above_one() {
        let s = FixedStrategy::new(1.5, 1);
        assert_eq!(s.scan_fraction(0), 1.0);
    }

    #[test]
    fn fixed_clamps_fraction_below_zero() {
        let s = FixedStrategy::new(-0.5, 1);
        assert_eq!(s.scan_fraction(0), 0.0);
    }

    #[test]
    fn fixed_zero_systems_never_runs_expensive() {
        let s = FixedStrategy::new(0.125, 0);
        for tick in 0..16 {
            for id in 0..4 {
                assert!(
                    !s.should_run_expensive(tick, id),
                    "should_run_expensive should always be false with 0 systems"
                );
            }
        }
    }

    #[test]
    fn fixed_fraction_boundary_zero() {
        let s = FixedStrategy::new(0.0, 4);
        assert_eq!(s.scan_fraction(0), 0.0);
    }

    #[test]
    fn fixed_fraction_boundary_one() {
        let s = FixedStrategy::new(1.0, 4);
        assert_eq!(s.scan_fraction(0), 1.0);
    }

    // ── AdaptiveStrategy ────────────────────────────────────────────────

    #[test]
    fn adaptive_uses_base_fraction_below_threshold() {
        let s = AdaptiveStrategy::new(0.25, 1000, 4);
        // Below threshold: should return base fraction
        assert_eq!(s.compute_fraction(500), 0.25);
        assert_eq!(s.compute_fraction(999), 0.25);
        assert_eq!(s.compute_fraction(1000), 0.25); // at threshold
    }

    #[test]
    fn adaptive_reduces_fraction_above_threshold() {
        let s = AdaptiveStrategy::new(0.5, 1000, 4);
        // 2000 entities = 2x threshold => fraction halved
        let f = s.compute_fraction(2000);
        assert!((f - 0.25).abs() < 1e-6, "Expected ~0.25, got {f}");

        // 4000 entities = 4x threshold => fraction quartered
        let f = s.compute_fraction(4000);
        assert!((f - 0.125).abs() < 1e-6, "Expected ~0.125, got {f}");
    }

    #[test]
    fn adaptive_scan_fraction_returns_base() {
        let s = AdaptiveStrategy::new(0.25, 1000, 4);
        // The trait method returns base_fraction (not entity-adjusted)
        assert_eq!(s.scan_fraction(0), 0.25);
        assert_eq!(s.scan_fraction(100), 0.25);
    }

    #[test]
    fn adaptive_round_robins_expensive_systems() {
        let s = AdaptiveStrategy::new(0.25, 1000, 3);
        // Same round-robin as fixed, with 3 systems
        assert!(s.should_run_expensive(0, 0));
        assert!(s.should_run_expensive(1, 1));
        assert!(s.should_run_expensive(2, 2));
        assert!(s.should_run_expensive(3, 0));
        assert!(!s.should_run_expensive(0, 1));
    }

    #[test]
    fn adaptive_name() {
        let s = AdaptiveStrategy::new(0.25, 1000, 4);
        assert_eq!(s.name(), "adaptive");
    }

    #[test]
    fn adaptive_zero_systems_never_runs_expensive() {
        let s = AdaptiveStrategy::new(0.25, 1000, 0);
        for tick in 0..16 {
            for id in 0..4 {
                assert!(
                    !s.should_run_expensive(tick, id),
                    "should_run_expensive should always be false with 0 systems"
                );
            }
        }
    }

    #[test]
    fn adaptive_compute_fraction_zero_entities() {
        let s = AdaptiveStrategy::new(0.5, 1000, 4);
        // 0 entities should return base fraction (not divide by zero)
        assert_eq!(s.compute_fraction(0), 0.5);
    }

    #[test]
    fn adaptive_clamps_fraction() {
        let s = AdaptiveStrategy::new(1.5, 1000, 4);
        // Constructor clamps to 1.0
        assert_eq!(s.scan_fraction(0), 1.0);
        assert_eq!(s.compute_fraction(500), 1.0);
    }

    // ── Trait object usage ──────────────────────────────────────────────

    #[test]
    fn trait_object_dispatch() {
        let strategies: Vec<Box<dyn SchedulingStrategy>> = vec![
            Box::new(FixedStrategy::new(0.125, 4)),
            Box::new(AdaptiveStrategy::new(0.25, 1000, 4)),
        ];

        assert_eq!(strategies[0].name(), "fixed");
        assert_eq!(strategies[1].name(), "adaptive");
        assert!(strategies[0].scan_fraction(0) > 0.0);
        assert!(strategies[1].scan_fraction(0) > 0.0);
    }

    // ── Single system edge case ─────────────────────────────────────────

    #[test]
    fn single_system_always_runs() {
        let s = FixedStrategy::new(0.5, 1);
        for tick in 0..10 {
            assert!(
                s.should_run_expensive(tick, 0),
                "System 0 should always run with num_systems=1"
            );
        }
    }
}
