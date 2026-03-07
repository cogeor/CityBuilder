//! Tile visual asset system — data-driven pattern definitions for tile rendering.
//!
//! Each tile type maps to a `TileVisual` that defines how it looks on the GPU:
//! base color, optional stripe overlay, stripe angle and spacing.
//!
//! The `TileVisualRegistry` is the single source of truth for tile appearance.
//! Pattern data is uploaded to the GPU as a uniform buffer.

use bytemuck::{Pod, Zeroable};

/// Maximum number of distinct tile visual patterns.
/// Must match `MAX_PATTERNS` in the WGSL shader.
pub const MAX_PATTERNS: usize = 32;

/// GPU-side pattern definition — uploaded as a uniform array.
///
/// 48 bytes per pattern (3 × vec4<f32>), aligned for GPU uniform buffers.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuPattern {
    /// Base fill color (RGBA).
    pub base_color: [f32; 4],
    /// Stripe overlay color (RGBA). Alpha=0 means no stripes.
    pub stripe_color: [f32; 4],
    /// x: stripe angle in radians, y: stripe width in pixels, z: stripe spacing in pixels, w: unused
    pub stripe_params: [f32; 4],
}

impl GpuPattern {
    /// Solid color, no stripes.
    pub fn solid(r: f32, g: f32, b: f32) -> Self {
        Self {
            base_color: [r, g, b, 1.0],
            stripe_color: [0.0, 0.0, 0.0, 0.0],
            stripe_params: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Base color with diagonal stripes.
    pub fn striped(
        base: [f32; 3],
        stripe: [f32; 3],
        stripe_alpha: f32,
        angle_deg: f32,
        width: f32,
        spacing: f32,
    ) -> Self {
        Self {
            base_color: [base[0], base[1], base[2], 1.0],
            stripe_color: [stripe[0], stripe[1], stripe[2], stripe_alpha],
            stripe_params: [angle_deg.to_radians(), width, spacing, 0.0],
        }
    }
}

/// Identifies a tile's visual appearance for the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternId(pub u32);

/// Registry mapping tile categories to GPU pattern definitions.
///
/// Tile categories:
///   0  = Grass (default terrain)
///   1  = Water
///   2  = Sand
///   3  = Forest
///   4  = Rock
///   7  = Road
///  11  = Residential zone (empty)
///  12  = Commercial zone (empty)
///  13  = Industrial zone (empty)
///  14  = Civic zone
///  15  = Park zone
///  16  = Transport zone
///  21  = Residential building (occupied)
///  22  = Commercial building (occupied)
///  23  = Industrial building (occupied)
///  24  = Civic building
pub struct TileVisualRegistry {
    patterns: [GpuPattern; MAX_PATTERNS],
    count: usize,
}

impl TileVisualRegistry {
    /// Create the default registry with all standard tile patterns.
    pub fn new() -> Self {
        let mut patterns = [GpuPattern::solid(0.0, 0.0, 0.0); MAX_PATTERNS];
        let mut count = 0;

        let mut set = |id: usize, p: GpuPattern| {
            patterns[id] = p;
            if id >= count { count = id + 1; }
        };

        // ── Terrain ──────────────────────────────────────────────────
        // 0: Grass — solid rich green
        set(0, GpuPattern::solid(0.35, 0.65, 0.25));
        // 1: Water — blue with subtle horizontal wave stripes
        set(1, GpuPattern::striped(
            [0.15, 0.40, 0.70], [0.20, 0.50, 0.80], 0.4,
            0.0, 2.0, 8.0,
        ));
        // 2: Sand — warm beige with fine diagonal grain
        set(2, GpuPattern::striped(
            [0.85, 0.75, 0.50], [0.80, 0.70, 0.45], 0.3,
            30.0, 1.0, 4.0,
        ));
        // 3: Forest — dark green with vertical tree-line stripes
        set(3, GpuPattern::striped(
            [0.15, 0.45, 0.15], [0.10, 0.35, 0.10], 0.5,
            90.0, 2.0, 6.0,
        ));
        // 4: Rock — grey-brown with angular stripes
        set(4, GpuPattern::striped(
            [0.50, 0.45, 0.40], [0.45, 0.40, 0.35], 0.4,
            60.0, 2.0, 5.0,
        ));

        // ── Road ─────────────────────────────────────────────────────
        // 7: Road — dark grey with center dashed line
        set(7, GpuPattern::striped(
            [0.35, 0.35, 0.38], [0.55, 0.55, 0.55], 0.6,
            0.0, 1.0, 6.0,
        ));

        // ── Zones (empty, awaiting development) ──────────────────────
        // 11: Residential zone — light green with diagonal hatching
        set(11, GpuPattern::striped(
            [0.30, 0.60, 0.28], [0.40, 0.75, 0.35], 0.5,
            45.0, 2.0, 8.0,
        ));
        // 12: Commercial zone — light blue with diagonal hatching
        set(12, GpuPattern::striped(
            [0.25, 0.38, 0.65], [0.35, 0.50, 0.80], 0.5,
            -45.0, 2.0, 8.0,
        ));
        // 13: Industrial zone — amber with cross-hatching (horizontal)
        set(13, GpuPattern::striped(
            [0.65, 0.58, 0.20], [0.80, 0.70, 0.25], 0.5,
            0.0, 2.0, 8.0,
        ));
        // 14: Civic zone — purple with hatching
        set(14, GpuPattern::striped(
            [0.50, 0.35, 0.60], [0.60, 0.45, 0.70], 0.4,
            45.0, 2.0, 10.0,
        ));
        // 15: Park — bright green with grass-line stripes
        set(15, GpuPattern::striped(
            [0.25, 0.65, 0.25], [0.30, 0.75, 0.30], 0.4,
            90.0, 1.5, 5.0,
        ));
        // 16: Transport zone — grey
        set(16, GpuPattern::solid(0.55, 0.55, 0.55));

        // ── Buildings (occupied tiles) ───────────────────────────────
        // 21: Residential building — darker green, denser stripes
        set(21, GpuPattern::striped(
            [0.22, 0.50, 0.22], [0.35, 0.65, 0.30], 0.7,
            45.0, 3.0, 5.0,
        ));
        // 22: Commercial building — deeper blue, denser stripes
        set(22, GpuPattern::striped(
            [0.20, 0.30, 0.60], [0.30, 0.45, 0.75], 0.7,
            -45.0, 3.0, 5.0,
        ));
        // 23: Industrial building — darker amber, horizontal stripes
        set(23, GpuPattern::striped(
            [0.55, 0.48, 0.15], [0.70, 0.60, 0.20], 0.7,
            0.0, 3.0, 5.0,
        ));
        // 24: Civic building — solid deep purple
        set(24, GpuPattern::striped(
            [0.40, 0.28, 0.55], [0.55, 0.40, 0.65], 0.6,
            45.0, 3.0, 6.0,
        ));

        Self { patterns, count }
    }

    /// Get the GPU pattern array for uploading to a uniform buffer.
    pub fn as_gpu_array(&self) -> &[GpuPattern; MAX_PATTERNS] {
        &self.patterns
    }

    /// Number of defined patterns.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Look up a pattern by ID.
    pub fn get(&self, id: PatternId) -> &GpuPattern {
        &self.patterns[(id.0 as usize).min(MAX_PATTERNS - 1)]
    }
}

impl Default for TileVisualRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_standard_patterns() {
        let reg = TileVisualRegistry::new();
        assert!(reg.count() >= 20);
    }

    #[test]
    fn grass_is_solid_green() {
        let reg = TileVisualRegistry::new();
        let grass = reg.get(PatternId(0));
        assert!(grass.base_color[1] > grass.base_color[0]); // green > red
        assert_eq!(grass.stripe_color[3], 0.0); // no stripes
    }

    #[test]
    fn residential_zone_has_stripes() {
        let reg = TileVisualRegistry::new();
        let res = reg.get(PatternId(11));
        assert!(res.stripe_color[3] > 0.0); // has stripes
        assert!(res.stripe_params[1] > 0.0); // stripe width > 0
    }

    #[test]
    fn building_patterns_are_denser() {
        let reg = TileVisualRegistry::new();
        let zone = reg.get(PatternId(11));
        let bldg = reg.get(PatternId(21));
        // Building stripe spacing should be tighter than zone
        assert!(bldg.stripe_params[2] <= zone.stripe_params[2]);
    }

    #[test]
    fn gpu_pattern_is_48_bytes() {
        assert_eq!(std::mem::size_of::<GpuPattern>(), 48);
    }

    #[test]
    fn gpu_array_is_pod() {
        // Verify the array can be cast to bytes (bytemuck requirement)
        let reg = TileVisualRegistry::new();
        let _bytes: &[u8] = bytemuck::cast_slice(reg.as_gpu_array());
    }
}
