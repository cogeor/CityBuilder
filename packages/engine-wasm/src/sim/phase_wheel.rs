//! Phase wheel scheduler for amortized computation.
//!
//! 4-phase rotation (A/B/C/D) spreads heavy work across ticks.
//! Spatial scans process 1/8 of map per invocation.

use crate::core_types::*;

/// The four phases of the wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    /// Transport field update (1/8 map scan).
    Transport = 0,
    /// Power & utility recomputation.
    Utilities = 1,
    /// Economy aggregates update.
    Economy = 2,
    /// Heatmap / analysis refresh (1/8 map scan).
    Analysis = 3,
}

impl Phase {
    /// Get the phase for a given tick.
    pub const fn for_tick(tick: Tick) -> Phase {
        match tick % 4 {
            0 => Phase::Transport,
            1 => Phase::Utilities,
            2 => Phase::Economy,
            _ => Phase::Analysis,
        }
    }

    /// Get all phases as an array.
    pub const fn all() -> [Phase; 4] {
        [
            Phase::Transport,
            Phase::Utilities,
            Phase::Economy,
            Phase::Analysis,
        ]
    }
}

/// Phase wheel scheduler state.
#[derive(Debug)]
pub struct PhaseWheel {
    /// Current scan fraction denominator (1/N of map per scan).
    /// Starts at 8, adapts based on performance.
    scan_denominator: u32,
    /// Target tick time in microseconds.
    target_tick_us: u32,
    /// Minimum scan denominator (most work per tick).
    min_denominator: u32,
    /// Maximum scan denominator (least work per tick).
    max_denominator: u32,
}

impl PhaseWheel {
    /// Create a new phase wheel with default settings.
    pub fn new() -> Self {
        PhaseWheel {
            scan_denominator: 8,
            target_tick_us: 8000, // 8ms target
            min_denominator: 4,   // at most 1/4 of map
            max_denominator: 32,  // at least 1/32 of map
        }
    }

    /// Get the current phase for a tick.
    #[inline]
    pub fn current_phase(&self, tick: Tick) -> Phase {
        Phase::for_tick(tick)
    }

    /// Check if a specific phase should run its expensive computation this tick.
    #[inline]
    pub fn should_run_expensive(&self, tick: Tick, phase: Phase) -> bool {
        Phase::for_tick(tick) == phase
    }

    /// Compute the scan window for spatial operations.
    /// Returns (start_index, count) into the tile array.
    pub fn scan_window(&self, tick: Tick, total_tiles: u32) -> (usize, usize) {
        let cycle = (tick / 4) as u32;
        let segment = cycle % self.scan_denominator;
        let segment_size = total_tiles / self.scan_denominator;
        let start = segment * segment_size;
        let count = if segment == self.scan_denominator - 1 {
            // Last segment gets any remainder
            total_tiles - start
        } else {
            segment_size
        };
        (start as usize, count as usize)
    }

    /// Get the current scan fraction (1/N).
    #[inline]
    pub fn scan_fraction(&self) -> u32 {
        self.scan_denominator
    }

    /// Adapt scan fraction based on measured tick time.
    /// Call after each tick with the measured duration.
    pub fn adapt(&mut self, measured_tick_us: u32) {
        let threshold_high = self.target_tick_us * 120 / 100; // 120% of target
        let threshold_low = self.target_tick_us * 50 / 100; // 50% of target

        if measured_tick_us > threshold_high {
            // Too slow: halve scan work
            self.scan_denominator = (self.scan_denominator * 2).min(self.max_denominator);
        } else if measured_tick_us < threshold_low {
            // Lots of headroom: increase scan work
            self.scan_denominator = (self.scan_denominator / 2).max(self.min_denominator);
        }
    }

    /// Get the number of full cycles needed to scan the entire map.
    pub fn full_scan_cycles(&self) -> u32 {
        self.scan_denominator
    }

    /// Get the number of ticks for a full map scan (cycles * 4 phases).
    pub fn full_scan_ticks(&self) -> u32 {
        self.scan_denominator * 4
    }
}

impl Default for PhaseWheel {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Tests ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_for_tick_cycles_correctly() {
        assert_eq!(Phase::for_tick(0), Phase::Transport);
        assert_eq!(Phase::for_tick(1), Phase::Utilities);
        assert_eq!(Phase::for_tick(2), Phase::Economy);
        assert_eq!(Phase::for_tick(3), Phase::Analysis);
        // Wraps back to Transport at tick 4
        assert_eq!(Phase::for_tick(4), Phase::Transport);
        assert_eq!(Phase::for_tick(5), Phase::Utilities);
        assert_eq!(Phase::for_tick(6), Phase::Economy);
        assert_eq!(Phase::for_tick(7), Phase::Analysis);
    }

    #[test]
    fn phase_all_returns_all_four() {
        let all = Phase::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], Phase::Transport);
        assert_eq!(all[1], Phase::Utilities);
        assert_eq!(all[2], Phase::Economy);
        assert_eq!(all[3], Phase::Analysis);
    }

    #[test]
    fn scan_window_covers_entire_map_after_n_cycles() {
        let wheel = PhaseWheel::new();
        let total_tiles: u32 = 256 * 256; // 65536 tiles
        let denom = wheel.scan_fraction();

        let mut covered = vec![false; total_tiles as usize];

        // Each cycle corresponds to 4 ticks. We need `denom` cycles to scan
        // the full map. Cycle i starts at tick i*4.
        for cycle in 0..denom {
            let tick = (cycle as u64) * 4;
            let (start, count) = wheel.scan_window(tick, total_tiles);
            for i in start..start + count {
                covered[i] = true;
            }
        }

        assert!(covered.iter().all(|&c| c), "Not all tiles were covered");
    }

    #[test]
    fn scan_window_segments_dont_overlap() {
        let wheel = PhaseWheel::new();
        let total_tiles: u32 = 256 * 256;
        let denom = wheel.scan_fraction();

        let mut visit_count = vec![0u32; total_tiles as usize];

        for cycle in 0..denom {
            let tick = (cycle as u64) * 4;
            let (start, count) = wheel.scan_window(tick, total_tiles);
            for i in start..start + count {
                visit_count[i] += 1;
            }
        }

        // Every tile should be visited exactly once
        assert!(
            visit_count.iter().all(|&c| c == 1),
            "Some tiles were visited more than once or not at all"
        );
    }

    #[test]
    fn adapt_reduces_denominator_when_fast() {
        let mut wheel = PhaseWheel::new();
        assert_eq!(wheel.scan_fraction(), 8);

        // Simulate very fast tick (below 50% of 8000 = 4000)
        wheel.adapt(2000);
        assert_eq!(wheel.scan_fraction(), 4); // 8 / 2 = 4
    }

    #[test]
    fn adapt_increases_denominator_when_slow() {
        let mut wheel = PhaseWheel::new();
        assert_eq!(wheel.scan_fraction(), 8);

        // Simulate slow tick (above 120% of 8000 = 9600)
        wheel.adapt(12000);
        assert_eq!(wheel.scan_fraction(), 16); // 8 * 2 = 16
    }

    #[test]
    fn adapt_doesnt_go_below_min() {
        let mut wheel = PhaseWheel::new();

        // Drive denominator down to minimum
        for _ in 0..10 {
            wheel.adapt(1000); // Very fast
        }
        assert_eq!(wheel.scan_fraction(), 4); // min_denominator
    }

    #[test]
    fn adapt_doesnt_go_above_max() {
        let mut wheel = PhaseWheel::new();

        // Drive denominator up to maximum
        for _ in 0..10 {
            wheel.adapt(50000); // Very slow
        }
        assert_eq!(wheel.scan_fraction(), 32); // max_denominator
    }

    #[test]
    fn adapt_no_change_in_normal_range() {
        let mut wheel = PhaseWheel::new();

        // Tick time within normal range (50%-120% of 8000 = 4000-9600)
        wheel.adapt(6000);
        assert_eq!(wheel.scan_fraction(), 8); // unchanged
    }

    #[test]
    fn full_scan_ticks_default() {
        let wheel = PhaseWheel::new();
        assert_eq!(wheel.full_scan_ticks(), 32); // 8 * 4
    }

    #[test]
    fn full_scan_cycles_default() {
        let wheel = PhaseWheel::new();
        assert_eq!(wheel.full_scan_cycles(), 8);
    }

    #[test]
    fn should_run_expensive_matches_for_tick() {
        let wheel = PhaseWheel::new();

        for tick in 0..16u64 {
            let current = Phase::for_tick(tick);
            for phase in Phase::all() {
                assert_eq!(
                    wheel.should_run_expensive(tick, phase),
                    current == phase,
                    "Mismatch at tick={tick}, phase={phase:?}"
                );
            }
        }
    }

    #[test]
    fn current_phase_matches_for_tick() {
        let wheel = PhaseWheel::new();
        for tick in 0..100u64 {
            assert_eq!(wheel.current_phase(tick), Phase::for_tick(tick));
        }
    }

    #[test]
    fn default_impl_matches_new() {
        let a = PhaseWheel::new();
        let b = PhaseWheel::default();
        assert_eq!(a.scan_fraction(), b.scan_fraction());
        assert_eq!(a.full_scan_ticks(), b.full_scan_ticks());
    }

    #[test]
    fn scan_window_handles_uneven_map_size() {
        let wheel = PhaseWheel::new();
        // 100 tiles is not evenly divisible by 8
        let total_tiles: u32 = 100;
        let denom = wheel.scan_fraction();

        let mut covered = vec![false; total_tiles as usize];

        for cycle in 0..denom {
            let tick = (cycle as u64) * 4;
            let (start, count) = wheel.scan_window(tick, total_tiles);
            for i in start..start + count {
                assert!(i < total_tiles as usize, "Index out of bounds");
                covered[i] = true;
            }
        }

        // With integer division, the last segment picks up remainder.
        // Check that the last segment covers everything from its start to the end.
        let last_tick = ((denom - 1) as u64) * 4;
        let (last_start, last_count) = wheel.scan_window(last_tick, total_tiles);
        assert_eq!(last_start + last_count, total_tiles as usize);
    }

    #[test]
    fn phase_repr_values() {
        assert_eq!(Phase::Transport as u8, 0);
        assert_eq!(Phase::Utilities as u8, 1);
        assert_eq!(Phase::Economy as u8, 2);
        assert_eq!(Phase::Analysis as u8, 3);
    }
}
