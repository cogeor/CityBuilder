//! Simulation speed control.
//!
//! `SimSpeed` determines how many simulation ticks are executed per frame.
//! When the speed is `Paused` the tick body is skipped entirely and the
//! world state is not mutated.

use serde::{Deserialize, Serialize};

/// The four discrete speed levels available to the player.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimSpeed {
    /// Simulation is frozen — no world state changes occur.
    Paused = 0,
    /// Slow playback: 1 simulation tick per frame.
    Slow = 1,
    /// Normal playback: 2 simulation ticks per frame.
    Normal = 2,
    /// Fast playback: 4 simulation ticks per frame.
    Fast = 3,
}

impl Default for SimSpeed {
    fn default() -> Self {
        SimSpeed::Normal
    }
}

impl SimSpeed {
    /// Returns the number of simulation ticks that should be executed per
    /// rendered frame at this speed level.
    ///
    /// * `Paused`  → 0 (tick body is skipped)
    /// * `Slow`    → 1
    /// * `Normal`  → 2
    /// * `Fast`    → 4
    pub fn ticks_per_frame(self) -> u32 {
        match self {
            SimSpeed::Paused => 0,
            SimSpeed::Slow => 1,
            SimSpeed::Normal => 2,
            SimSpeed::Fast => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_normal() {
        assert_eq!(SimSpeed::default(), SimSpeed::Normal);
    }

    #[test]
    fn ticks_per_frame_values() {
        assert_eq!(SimSpeed::Paused.ticks_per_frame(), 0);
        assert_eq!(SimSpeed::Slow.ticks_per_frame(), 1);
        assert_eq!(SimSpeed::Normal.ticks_per_frame(), 2);
        assert_eq!(SimSpeed::Fast.ticks_per_frame(), 4);
    }

    #[test]
    fn discriminant_values() {
        assert_eq!(SimSpeed::Paused as u8, 0);
        assert_eq!(SimSpeed::Slow as u8, 1);
        assert_eq!(SimSpeed::Normal as u8, 2);
        assert_eq!(SimSpeed::Fast as u8, 3);
    }
}
