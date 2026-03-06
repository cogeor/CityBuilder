//! Core modules: world state, entities, commands, scale, events, and region.

pub mod alloc_tracker;
pub mod archetype_traits;
pub mod archetypes;
pub mod buildings;
pub mod commands;
pub mod commands_spec;
pub mod diffs;
pub mod entity;
pub mod events;
pub mod mapgen;
pub mod math_util;
pub mod network;
pub mod region;
pub mod scale;
pub mod stats_recorder;
pub mod tile_constants;
pub mod tilemap;
pub mod world;
