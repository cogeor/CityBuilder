//! city_core — Core primitives, traits, and plugin framework.
//!
//! This crate provides the foundational types and the Bevy-inspired
//! Plugin/App/PluginGroup framework for the city builder engine.

pub mod app;
pub mod plugin;
pub mod plugin_group;
pub mod resource;
pub mod schedule;
pub mod system;
pub mod terrain;
pub mod types;
pub mod zone;

// Re-export key items at crate root for ergonomics.
pub use app::App;
pub use plugin::Plugin;
pub use plugin_group::{PluginGroup, PluginGroupBuilder};
pub use resource::Resource;
pub use schedule::Schedule;
pub use system::{SimContext, SimSystem};
pub use types::*;
