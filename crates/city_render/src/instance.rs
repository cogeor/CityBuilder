//! GPU instance data for rendering.

use bytemuck::{Pod, Zeroable};

/// Per-instance data for terrain tiles (flat isometric diamonds).
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuInstance {
    /// Screen position in pixels (isometric projected).
    pub screen_pos: [f32; 2],
    /// Depth sort key (0.0-1.0, lower = further back).
    pub z_order: f32,
    /// Color index (maps to a flat color in the shader).
    pub color_id: u32,
}

impl GpuInstance {
    pub fn new(screen_x: f32, screen_y: f32, z_order: f32, color_id: u32) -> Self {
        Self {
            screen_pos: [screen_x, screen_y],
            z_order,
            color_id,
        }
    }
}

/// Per-instance data for building sprites (textured quads, taller than tiles).
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct SpriteInstance {
    /// Screen position in pixels (tile center).
    pub screen_pos: [f32; 2],
    /// Depth sort key (0.0-1.0).
    pub z_order: f32,
    /// Padding for alignment.
    pub _pad: f32,
    /// Atlas UV rect: (u0, v0, u1, v1).
    pub uv_rect: [f32; 4],
    /// Sprite size in world pixels (width, height).
    pub size: [f32; 2],
    /// Padding.
    pub _pad2: [f32; 2],
}

impl SpriteInstance {
    pub fn new(screen_x: f32, screen_y: f32, z_order: f32, uv_rect: [f32; 4], width: f32, height: f32) -> Self {
        Self {
            screen_pos: [screen_x, screen_y],
            z_order,
            _pad: 0.0,
            uv_rect,
            size: [width, height],
            _pad2: [0.0; 2],
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
    pub fn ortho_zoom(width: f32, height: f32, cam_x: f32, cam_y: f32, zoom: f32) -> Self {
        let vw = width * zoom;
        let vh = height * zoom;
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
