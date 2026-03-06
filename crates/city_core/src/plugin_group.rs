//! PluginGroup — bundles related plugins together.
//!
//! Mirrors `bevy_app::PluginGroup`. Allows grouping plugins like
//! `CityBuilderPlugins` (similar to Bevy's `DefaultPlugins`).

use crate::plugin::Plugin;
use crate::App;

/// A set of [`Plugin`]s that can be added together.
///
/// # Example
/// ```ignore
/// pub struct CityBuilderPlugins;
///
/// impl PluginGroup for CityBuilderPlugins {
///     fn build(self) -> PluginGroupBuilder {
///         PluginGroupBuilder::new()
///             .add(TerrainPlugin)
///             .add(ZoningPlugin)
///     }
/// }
/// ```
pub trait PluginGroup {
    fn build(self) -> PluginGroupBuilder;
}

/// Ordered list of boxed plugins built by a [`PluginGroup`].
pub struct PluginGroupBuilder {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginGroupBuilder {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Add a plugin to the group.
    pub fn add<P: Plugin>(mut self, plugin: P) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Consume the builder, applying all plugins to the app.
    pub fn finish(self, app: &mut App) {
        for plugin in self.plugins {
            app.add_boxed_plugin(plugin);
        }
    }
}

impl Default for PluginGroupBuilder {
    fn default() -> Self {
        Self::new()
    }
}
