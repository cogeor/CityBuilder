//! SimSystem — trait for simulation systems that run each tick.
//!
//! Each system implements this trait and is registered into a [`Schedule`]
//! via the [`App`]. Systems receive a [`SimContext`] with mutable access
//! to the world state.

use crate::resource::ResourceMap;
use crate::types::Tick;

/// A simulation system that processes world state each tick.
///
/// # Example
/// ```ignore
/// struct PopulationSystem;
///
/// impl SimSystem for PopulationSystem {
///     fn name(&self) -> &str { "population" }
///     fn tick(&mut self, ctx: &mut SimContext) {
///         // update population based on housing capacity
///     }
/// }
/// ```
pub trait SimSystem: Send + Sync {
    /// Human-readable name for debugging and profiling.
    fn name(&self) -> &str;

    /// Called each frame within the system's schedule phase.
    fn tick(&mut self, ctx: &mut SimContext);
}

/// Context passed to systems each tick.
///
/// Provides mutable access to the world state and read access to
/// shared registries. The engine ensures no two systems in the same
/// phase run concurrently (sequential execution within each phase).
pub struct SimContext<'a> {
    /// Current simulation tick number.
    pub tick: Tick,

    /// Type-erased resource storage (registries, configs, etc.)
    pub resources: &'a mut ResourceMap,
}

/// Anything that can be converted into a boxed SimSystem.
pub trait IntoSimSystem {
    fn into_system(self) -> Box<dyn SimSystem>;
}

impl<S: SimSystem + 'static> IntoSimSystem for S {
    fn into_system(self) -> Box<dyn SimSystem> {
        Box::new(self)
    }
}
