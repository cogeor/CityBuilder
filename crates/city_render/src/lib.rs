//! city_render — GPU rendering pipeline for the isometric city builder.
//!
//! Uses wgpu for cross-platform GPU rendering and winit for windowing.

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

pub mod instance;
pub mod projection;
pub mod renderer;
pub mod sprites;
pub mod tile_visuals;
