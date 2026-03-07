//! GPU instance data for rendering.

use bytemuck::{Pod, Zeroable};

/// Per-instance data uploaded to GPU for each terrain tile.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuInstance {
    /// Screen position in pixels (isometric projected).
    pub screen_pos: [f32; 2],
    /// Depth sort key (0.0-1.0, lower = further back).
    pub z_order: f32,
    /// Pattern ID — indexes into the pattern uniform array.
    pub pattern_id: u32,
}

impl GpuInstance {
    pub fn new(screen_x: f32, screen_y: f32, z_order: f32, pattern_id: u32) -> Self {
        Self {
            screen_pos: [screen_x, screen_y],
            z_order,
            pattern_id,
        }
    }
}

/// Uniform data for the vertex shader.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Uniforms {
    /// Orthographic projection matrix (column-major).
    pub projection: [[f32; 4]; 4],
    /// Camera offset in pixels.
    pub camera_offset: [f32; 2],
    /// Padding.
    pub _padding: [f32; 2],
}

impl Uniforms {
    /// Create an orthographic projection for the given screen size.
    ///
    /// `zoom` controls how many world-pixels map to one screen-pixel.
    /// zoom=1.0 is 1:1, zoom=10.0 means 10 world-pixels per screen-pixel.
    pub fn ortho_zoom(width: f32, height: f32, cam_x: f32, cam_y: f32, zoom: f32) -> Self {
        let vw = width * zoom;
        let vh = height * zoom;
        // Center-origin projection: (0,0) in world maps to center of screen
        let projection = [
            [2.0 / vw, 0.0,        0.0, 0.0],
            [0.0,      -2.0 / vh,  0.0, 0.0],
            [0.0,      0.0,        1.0, 0.0],
            [0.0,      0.0,        0.0, 1.0],
        ];
        Self {
            projection,
            camera_offset: [cam_x, cam_y],
            _padding: [0.0; 2],
        }
    }

    /// Create an orthographic projection (1:1 pixel mapping).
    pub fn ortho(width: f32, height: f32, cam_x: f32, cam_y: f32) -> Self {
        Self::ortho_zoom(width, height, cam_x, cam_y, 1.0)
    }
}
