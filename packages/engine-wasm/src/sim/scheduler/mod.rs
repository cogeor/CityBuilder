//! Deterministic ordered sim scheduler for the city-builder tick loop.
//!
//! `TickPhase` defines the execution order of system groups.
//! `SimSystem` is the trait for pluggable, frequency-gated systems.
//! `SimScheduler` dispatches systems in phase ordinal order each tick.

use crate::core_types::Tick;

// ─── TickPhase ────────────────────────────────────────────────────────────────

/// Execution-phase ordering for simulation systems.
///
/// Systems are dispatched in the ordinal order of this enum every tick.
/// This is separate from `PhaseWheel::Phase` (which governs spatial amortisation
/// scan windows) to avoid namespace collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TickPhase {
    /// Buildings, construction — run every tick.
    TileUpdate = 0,
    /// Power, water utilities — run every 2 ticks.
    UtilityUpdate = 1,
    /// Pollution wind, land value, vegetation — run every 4 ticks.
    EffectPropagation = 2,
    /// Wealth density, zone development — run every 10 ticks.
    ZoneGrowth = 3,
    /// Jobs, finance, population — run every tick.
    EconomyUpdate = 4,
    /// City events, citizen voice — run every 24 ticks.
    CitizenFeedback = 5,
    /// Transport metrics, infra lifecycle — run every 8 ticks.
    Analytics = 6,
}

// ─── SimSystem trait ──────────────────────────────────────────────────────────

/// A pluggable, frequency-gated simulation system.
///
/// Systems are registered with a `SimScheduler` and dispatched in
/// `TickPhase` ordinal order. The scheduler checks `update_frequency()`
/// before calling `run()`; a frequency of `1` means every tick, `4` means
/// every 4th tick, etc.
///
/// Note: `CityState` is not yet defined; systems in the new scheduler interact
/// via `run_raw` with a unit placeholder until `CityState` is fully extracted
/// from `SimulationEngine`. This is intentional scaffolding.
pub trait SimSystem: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn phase(&self) -> TickPhase;
    /// Run when `tick % update_frequency() == 0`. Return `1` to run every tick.
    fn update_frequency(&self) -> u32;
}

// ─── SimScheduler ─────────────────────────────────────────────────────────────

/// Ordered, frequency-gated dispatcher for simulation systems.
///
/// Systems are stored pre-sorted by `TickPhase` ordinal so that `dispatch`
/// iterates in a deterministic, stable order every invocation.
#[derive(Debug, Default)]
pub struct SimScheduler {
    /// Registered systems, sorted by `TickPhase` ordinal at registration time.
    systems: Vec<Box<dyn SimSystem>>,
}

impl SimScheduler {
    pub fn new() -> Self {
        SimScheduler { systems: Vec::new() }
    }

    /// Register a system. Systems are kept sorted by phase ordinal.
    pub fn register(&mut self, system: Box<dyn SimSystem>) {
        self.systems.push(system);
        self.systems.sort_by_key(|s| s.phase());
    }

    /// Returns `true` when this tick should trigger a system at `frequency`.
    #[inline]
    pub fn should_run(tick: Tick, frequency: u32) -> bool {
        frequency == 0 || tick % frequency as u64 == 0
    }

    /// Number of registered systems.
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// Whether the scheduler has no registered systems.
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// Iterate registered systems in phase order.
    pub fn systems(&self) -> &[Box<dyn SimSystem>] {
        &self.systems
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummySystem {
        name: &'static str,
        phase: TickPhase,
        freq: u32,
    }

    impl SimSystem for DummySystem {
        fn name(&self) -> &'static str { self.name }
        fn phase(&self) -> TickPhase { self.phase }
        fn update_frequency(&self) -> u32 { self.freq }
    }

    fn make(name: &'static str, phase: TickPhase, freq: u32) -> Box<dyn SimSystem> {
        Box::new(DummySystem { name, phase, freq })
    }

    #[test]
    fn systems_sorted_by_phase_ordinal() {
        let mut sched = SimScheduler::new();
        sched.register(make("economy", TickPhase::EconomyUpdate, 1));
        sched.register(make("tile", TickPhase::TileUpdate, 1));
        sched.register(make("analytics", TickPhase::Analytics, 8));

        let names: Vec<_> = sched.systems().iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["tile", "economy", "analytics"]);
    }

    #[test]
    fn should_run_frequency_1_every_tick() {
        for tick in 0..20u64 {
            assert!(SimScheduler::should_run(tick, 1));
        }
    }

    #[test]
    fn should_run_frequency_4_every_4th_tick() {
        for tick in 0..20u64 {
            let expected = tick % 4 == 0;
            assert_eq!(SimScheduler::should_run(tick, 4), expected);
        }
    }

    #[test]
    fn should_run_frequency_0_always() {
        // Frequency 0 is treated as "always run" (defensive).
        for tick in 0..10u64 {
            assert!(SimScheduler::should_run(tick, 0));
        }
    }

    #[test]
    fn register_multiple_phases_maintains_sort() {
        let mut sched = SimScheduler::new();
        sched.register(make("citizen", TickPhase::CitizenFeedback, 24));
        sched.register(make("utility", TickPhase::UtilityUpdate, 2));
        sched.register(make("zone", TickPhase::ZoneGrowth, 10));
        sched.register(make("effect", TickPhase::EffectPropagation, 4));

        let phases: Vec<_> = sched.systems().iter().map(|s| s.phase()).collect();
        // Must be in ascending ordinal order
        for i in 1..phases.len() {
            assert!(phases[i] >= phases[i - 1]);
        }
    }

    #[test]
    fn empty_scheduler() {
        let sched = SimScheduler::new();
        assert!(sched.is_empty());
        assert_eq!(sched.len(), 0);
    }

    #[test]
    fn tick_phase_ordinal_order() {
        assert!(TickPhase::TileUpdate < TickPhase::UtilityUpdate);
        assert!(TickPhase::UtilityUpdate < TickPhase::EffectPropagation);
        assert!(TickPhase::EffectPropagation < TickPhase::ZoneGrowth);
        assert!(TickPhase::ZoneGrowth < TickPhase::EconomyUpdate);
        assert!(TickPhase::EconomyUpdate < TickPhase::CitizenFeedback);
        assert!(TickPhase::CitizenFeedback < TickPhase::Analytics);
    }
}
