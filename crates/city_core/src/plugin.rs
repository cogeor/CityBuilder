//! Plugin trait — the core building block of the engine.
//!
//! Mirrors `bevy_app::Plugin`. Every feature is a plugin that configures
//! an [`App`] with systems, resources, and registrations.

use crate::App;
use std::any::Any;

/// A collection of engine logic and configuration.
///
/// Plugins configure an [`App`]. When an `App` registers a plugin,
/// the plugin's [`build`](Plugin::build) function is called immediately.
///
/// # Lifecycle
/// 1. `build()` — called immediately on registration
/// 2. `finish()` — called after all plugins are built
/// 3. `cleanup()` — called after finish, for handoff to threads etc.
pub trait Plugin: Any + Send + Sync {
    /// Configure the [`App`] to which this plugin is added.
    fn build(&self, app: &mut App);

    /// Called after all plugins are built. For late initialization.
    fn finish(&self, _app: &mut App) {}

    /// Runs after finish. For cleanup or thread handoff.
    fn cleanup(&self, _app: &mut App) {}

    /// Name for debugging and duplicate detection.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// If false, the plugin can be added multiple times.
    fn is_unique(&self) -> bool {
        true
    }
}
