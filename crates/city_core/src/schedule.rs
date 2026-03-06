//! Schedule — named execution phases for systems.
//!
//! Systems are registered into specific schedules that define when they run
//! during each tick. Phases execute in order from lowest to highest.

use serde::{Deserialize, Serialize};

/// Named execution phases, like Bevy's schedules.
///
/// Systems registered under a schedule run in the order they were added,
/// and schedules themselves execute in discriminant order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Schedule {
    /// Runs once at world creation / game start.
    Init = 0,
    /// Before the main simulation tick (input processing, command application).
    PreTick = 1,
    /// Main simulation systems (population, economy, transport, etc.).
    Tick = 2,
    /// After simulation (stats recording, event emission, dirty tracking).
    PostTick = 3,
    /// Render data extraction and preparation.
    Render = 4,
}
