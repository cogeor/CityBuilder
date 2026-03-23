//! App — the central builder and runtime, mirrors `bevy_app::App`.
//!
//! Plugins register systems, resources, and other state via the App.
//! The engine runs systems grouped by [`Schedule`] phase.

use crate::plugin::Plugin;
use crate::plugin_group::PluginGroup;
use crate::resource::ResourceMap;
use crate::schedule::Schedule;
use crate::system::{IntoSimSystem, SimSystem};
use std::collections::BTreeMap;

/// The central application builder and runtime.
///
/// # Example
/// ```ignore
/// let mut app = App::new();
/// app.add_plugin(TerrainPlugin);
/// app.add_plugin_group(CityBuilderPlugins);
/// ```
pub struct App {
    /// Systems grouped by schedule phase, in insertion order.
    systems: BTreeMap<Schedule, Vec<Box<dyn SimSystem>>>,

    /// Type-erased resource storage.
    pub resources: ResourceMap,

    /// Names of registered plugins (for duplicate detection).
    plugin_names: Vec<String>,

    /// Stored plugins for finish/cleanup lifecycle hooks.
    plugins: Vec<Box<dyn Plugin>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            systems: BTreeMap::new(),
            resources: ResourceMap::new(),
            plugin_names: Vec::new(),
            plugins: Vec::new(),
        }
    }

    /// Add a single plugin.
    pub fn add_plugins<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        self.add_boxed_plugin(Box::new(plugin));
        self
    }

    /// Add a plugin group (bundle of plugins).
    pub fn add_plugin_group<G: PluginGroup>(&mut self, group: G) -> &mut Self {
        group.build().finish(self);
        self
    }

    /// Register a system into a specific schedule phase.
    pub fn add_systems<S: IntoSimSystem>(
        &mut self,
        schedule: Schedule,
        system: S,
    ) -> &mut Self {
        self.systems
            .entry(schedule)
            .or_default()
            .push(system.into_system());
        self
    }

    /// Insert a resource, replacing any existing one of the same type.
    pub fn insert_resource<R: Send + Sync + 'static>(&mut self, resource: R) -> &mut Self {
        self.resources.insert(resource);
        self
    }

    /// Initialize a resource with its Default value if not already present.
    pub fn init_resource<R: Default + Send + Sync + 'static>(&mut self) -> &mut Self {
        if !self.resources.contains::<R>() {
            self.resources.insert(R::default());
        }
        self
    }

    /// Get a reference to a resource by type.
    pub fn get_resource<R: Send + Sync + 'static>(&self) -> Option<&R> {
        self.resources.get::<R>()
    }

    /// Get a mutable reference to a resource by type.
    pub fn get_resource_mut<R: Send + Sync + 'static>(&mut self) -> Option<&mut R> {
        self.resources.get_mut::<R>()
    }

    /// Remove a resource by type, returning it if it existed.
    pub fn remove_resource<R: Send + Sync + 'static>(&mut self) -> Option<R> {
        self.resources.remove::<R>()
    }

    // ── Internal ────────────────────────────────────────────────────────

    /// Add a boxed plugin. Called by add_plugins and PluginGroupBuilder.
    pub fn add_boxed_plugin(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();

        if plugin.is_unique() && self.plugin_names.contains(&name) {
            panic!("Plugin '{}' already added — duplicates not allowed", name);
        }

        plugin.build(self);
        self.plugin_names.push(name);
        self.plugins.push(plugin);
    }

    /// Call `finish` on all stored plugins. Run after all plugins are built.
    pub fn finish_plugins(&mut self) {
        let plugins = std::mem::take(&mut self.plugins);
        for p in &plugins {
            p.finish(self);
        }
        self.plugins = plugins;
    }

    /// Call `cleanup` on all stored plugins. Run after finish.
    pub fn cleanup_plugins(&mut self) {
        let plugins = std::mem::take(&mut self.plugins);
        for p in &plugins {
            p.cleanup(self);
        }
        self.plugins = plugins;
    }

    /// Get all systems for a given schedule, consuming them.
    pub fn take_systems(&mut self) -> BTreeMap<Schedule, Vec<Box<dyn SimSystem>>> {
        std::mem::take(&mut self.systems)
    }

    /// How many systems are registered total.
    pub fn system_count(&self) -> usize {
        self.systems.values().map(|v| v.len()).sum()
    }

    /// How many plugins have been registered.
    pub fn plugin_count(&self) -> usize {
        self.plugin_names.len()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::SimContext;

    struct TestSystem;
    impl SimSystem for TestSystem {
        fn name(&self) -> &str { "test" }
        fn tick(&mut self, _ctx: &mut SimContext) {}
    }

    struct TestPlugin;
    impl crate::Plugin for TestPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Schedule::Tick, TestSystem);
        }
    }

    #[test]
    fn plugin_adds_system() {
        let mut app = App::new();
        app.add_plugins(TestPlugin);
        assert_eq!(app.system_count(), 1);
        assert_eq!(app.plugin_count(), 1);
    }

    #[test]
    fn plugin_group() {
        struct PluginA;
        impl crate::Plugin for PluginA {
            fn build(&self, app: &mut App) {
                app.add_systems(Schedule::PreTick, TestSystem);
            }
        }

        struct PluginB;
        impl crate::Plugin for PluginB {
            fn build(&self, app: &mut App) {
                app.add_systems(Schedule::PostTick, TestSystem);
            }
        }

        struct TestGroup;
        impl PluginGroup for TestGroup {
            fn build(self) -> crate::PluginGroupBuilder {
                crate::PluginGroupBuilder::new()
                    .add(PluginA)
                    .add(PluginB)
            }
        }

        let mut app = App::new();
        app.add_plugin_group(TestGroup);
        assert_eq!(app.system_count(), 2);
        assert_eq!(app.plugin_count(), 2);
    }

    #[test]
    #[should_panic(expected = "already added")]
    fn duplicate_unique_plugin_panics() {
        let mut app = App::new();
        app.add_plugins(TestPlugin);
        app.add_plugins(TestPlugin);
    }

    #[test]
    fn resources() {
        let mut app = App::new();
        app.insert_resource(42u32);
        assert_eq!(*app.get_resource::<u32>().unwrap(), 42);
    }

    #[test]
    fn finish_plugins_called() {
        struct FinishPlugin;
        impl crate::Plugin for FinishPlugin {
            fn build(&self, _app: &mut App) {}
            fn finish(&self, app: &mut App) {
                // Insert a sentinel resource to prove finish was called
                app.insert_resource(99u64);
            }
        }

        let mut app = App::new();
        app.add_plugins(FinishPlugin);
        assert!(app.get_resource::<u64>().is_none(), "finish not yet called");
        app.finish_plugins();
        assert_eq!(*app.get_resource::<u64>().unwrap(), 99, "finish should have inserted resource");
    }

    #[test]
    fn cleanup_plugins_called() {
        struct CleanupPlugin;
        impl crate::Plugin for CleanupPlugin {
            fn build(&self, _app: &mut App) {}
            fn cleanup(&self, app: &mut App) {
                app.insert_resource(77u8);
            }
        }

        let mut app = App::new();
        app.add_plugins(CleanupPlugin);
        app.cleanup_plugins();
        assert_eq!(*app.get_resource::<u8>().unwrap(), 77);
    }
}
