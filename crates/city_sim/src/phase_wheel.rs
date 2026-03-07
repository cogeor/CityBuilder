use city_core::Tick;

/// The four phases of the wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    Transport = 0,
    Utilities = 1,
    Economy = 2,
    Analysis = 3,
}

impl Phase {
    pub const fn for_tick(tick: Tick) -> Phase {
        match tick % 4 {
            0 => Phase::Transport,
            1 => Phase::Utilities,
            2 => Phase::Economy,
            _ => Phase::Analysis,
        }
    }

    pub const fn all() -> [Phase; 4] {
        [Phase::Transport, Phase::Utilities, Phase::Economy, Phase::Analysis]
    }
}

/// Phase wheel scheduler state.
#[derive(Debug)]
pub struct PhaseWheel {
    scan_denominator: u32,
    target_tick_us: u32,
    min_denominator: u32,
    max_denominator: u32,
}

impl PhaseWheel {
    pub fn new() -> Self {
        PhaseWheel {
            scan_denominator: 8,
            target_tick_us: 8000,
            min_denominator: 4,
            max_denominator: 32,
        }
    }

    #[inline]
    pub fn current_phase(&self, tick: Tick) -> Phase {
        Phase::for_tick(tick)
    }

    #[inline]
    pub fn should_run_expensive(&self, tick: Tick, phase: Phase) -> bool {
        Phase::for_tick(tick) == phase
    }

    pub fn scan_window(&self, tick: Tick, total_tiles: u32) -> (usize, usize) {
        let cycle = (tick / 4) as u32;
        let segment = cycle % self.scan_denominator;
        let segment_size = total_tiles / self.scan_denominator;
        let start = segment * segment_size;
        let count = if segment == self.scan_denominator - 1 {
            total_tiles - start
        } else {
            segment_size
        };
        (start as usize, count as usize)
    }

    #[inline]
    pub fn scan_fraction(&self) -> u32 {
        self.scan_denominator
    }

    pub fn adapt(&mut self, measured_tick_us: u32) {
        let threshold_high = self.target_tick_us * 120 / 100;
        let threshold_low = self.target_tick_us * 50 / 100;

        if measured_tick_us > threshold_high {
            self.scan_denominator = (self.scan_denominator * 2).min(self.max_denominator);
        } else if measured_tick_us < threshold_low {
            self.scan_denominator = (self.scan_denominator / 2).max(self.min_denominator);
        }
    }

    pub fn full_scan_cycles(&self) -> u32 {
        self.scan_denominator
    }

    pub fn full_scan_ticks(&self) -> u32 {
        self.scan_denominator * 4
    }
}

impl Default for PhaseWheel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_for_tick_cycles_correctly() {
        assert_eq!(Phase::for_tick(0), Phase::Transport);
        assert_eq!(Phase::for_tick(1), Phase::Utilities);
        assert_eq!(Phase::for_tick(2), Phase::Economy);
        assert_eq!(Phase::for_tick(3), Phase::Analysis);
        assert_eq!(Phase::for_tick(4), Phase::Transport);
        assert_eq!(Phase::for_tick(7), Phase::Analysis);
    }

    #[test]
    fn phase_all_returns_all_four() {
        let all = Phase::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], Phase::Transport);
        assert_eq!(all[3], Phase::Analysis);
    }

    #[test]
    fn scan_window_covers_entire_map() {
        let wheel = PhaseWheel::new();
        let total_tiles: u32 = 256 * 256;
        let denom = wheel.scan_fraction();
        let mut covered = vec![false; total_tiles as usize];
        for cycle in 0..denom {
            let tick = (cycle as u64) * 4;
            let (start, count) = wheel.scan_window(tick, total_tiles);
            for i in start..start + count {
                covered[i] = true;
            }
        }
        assert!(covered.iter().all(|&c| c));
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
        assert!(visit_count.iter().all(|&c| c == 1));
    }

    #[test]
    fn adapt_reduces_denominator_when_fast() {
        let mut wheel = PhaseWheel::new();
        wheel.adapt(2000);
        assert_eq!(wheel.scan_fraction(), 4);
    }

    #[test]
    fn adapt_increases_denominator_when_slow() {
        let mut wheel = PhaseWheel::new();
        wheel.adapt(12000);
        assert_eq!(wheel.scan_fraction(), 16);
    }

    #[test]
    fn adapt_doesnt_go_below_min() {
        let mut wheel = PhaseWheel::new();
        for _ in 0..10 { wheel.adapt(1000); }
        assert_eq!(wheel.scan_fraction(), 4);
    }

    #[test]
    fn adapt_doesnt_go_above_max() {
        let mut wheel = PhaseWheel::new();
        for _ in 0..10 { wheel.adapt(50000); }
        assert_eq!(wheel.scan_fraction(), 32);
    }

    #[test]
    fn adapt_no_change_in_normal_range() {
        let mut wheel = PhaseWheel::new();
        wheel.adapt(6000);
        assert_eq!(wheel.scan_fraction(), 8);
    }

    #[test]
    fn full_scan_ticks_default() {
        let wheel = PhaseWheel::new();
        assert_eq!(wheel.full_scan_ticks(), 32);
    }

    #[test]
    fn scan_window_handles_uneven_map_size() {
        let wheel = PhaseWheel::new();
        let total_tiles: u32 = 100;
        let denom = wheel.scan_fraction();
        let mut covered = vec![false; total_tiles as usize];
        for cycle in 0..denom {
            let tick = (cycle as u64) * 4;
            let (start, count) = wheel.scan_window(tick, total_tiles);
            for i in start..start + count {
                covered[i] = true;
            }
        }
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
