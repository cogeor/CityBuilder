//! GPU instance data for rendering.

use bytemuck::{Pod, Zeroable};

/// Per-instance data uploaded to GPU for each terrain tile.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuInstance {
    /// Screen position in pixels (isometric projected).
    pub screen_pos: [f32; 2],
    /// RGBA color (0.0-1.0).
    pub color: [f32; 4],
    /// Depth sort key (0.0-1.0, lower = further back).
    pub z_order: f32,
    /// Padding to align to 32 bytes.
    pub _pad: f32,
}

impl GpuInstance {
    pub fn new(screen_x: f32, screen_y: f32, color: [f32; 4], z_order: f32) -> Self {
        Self {
            screen_pos: [screen_x, screen_y],
            color,
            z_order,
            _pad: 0.0,
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
    pub fn ortho(width: f32, height: f32, cam_x: f32, cam_y: f32) -> Self {
        // Map screen pixels to clip space [-1, 1].
        // Origin at top-left, Y down.
        let projection = [
            [2.0 / width, 0.0,           0.0, 0.0],
            [0.0,         -2.0 / height,  0.0, 0.0],
            [0.0,         0.0,            1.0, 0.0],
            [-1.0,        1.0,            0.0, 1.0],
        ];
        Self {
            projection,
            camera_offset: [cam_x, cam_y],
            _padding: [0.0; 2],
        }
    }
}
