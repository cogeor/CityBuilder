//! Isometric projection — tile coordinates to screen pixels.

/// Tile width in pixels.
pub const TILE_W: f32 = 64.0;
/// Tile height in pixels (half of width for standard isometric).
pub const TILE_H: f32 = 32.0;

/// Convert tile (x, y) to screen-space center point.
///
/// Standard isometric projection:
///   screen_x = (x - y) * TILE_W / 2
///   screen_y = (x + y) * TILE_H / 2
#[inline]
pub fn tile_to_screen(x: i16, y: i16) -> (f32, f32) {
    let fx = x as f32;
    let fy = y as f32;
    (
        (fx - fy) * TILE_W / 2.0,
        (fx + fy) * TILE_H / 2.0,
    )
}

/// Compute the center of the map in screen space (for initial camera position).
pub fn map_center_screen(width: u16, height: u16) -> (f32, f32) {
    // Center is at tile (width/2, height/2)
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    (
        (cx - cy) * TILE_W / 2.0,
        (cx + cy) * TILE_H / 2.0,
    )
}

/// Compute z-order for a tile (back-to-front painter ordering).
/// Tiles with higher x+y are in front.
#[inline]
pub fn tile_z_order(x: i16, y: i16, map_size: u16) -> f32 {
    let max_sum = (map_size as f32) * 2.0;
    let sum = (x + y) as f32;
    // Normalize to 0..1, with 0 = furthest back
    sum / max_sum
}

/// Terrain color palette — one color per terrain ID.
pub fn terrain_color(terrain_id: u8) -> [f32; 4] {
    match terrain_id {
        0 => [0.35, 0.65, 0.25, 1.0],  // Grass — rich green
        1 => [0.15, 0.40, 0.70, 1.0],  // Water — deep blue
        2 => [0.85, 0.75, 0.50, 1.0],  // Sand — warm beige
        3 => [0.15, 0.45, 0.15, 1.0],  // Forest — dark green
        4 => [0.50, 0.45, 0.40, 1.0],  // Rock — grey-brown
        _ => [0.80, 0.20, 0.80, 1.0],  // Unknown — magenta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_projects_to_zero() {
        let (sx, sy) = tile_to_screen(0, 0);
        assert_eq!(sx, 0.0);
        assert_eq!(sy, 0.0);
    }

    #[test]
    fn isometric_x_goes_right_down() {
        let (sx, sy) = tile_to_screen(1, 0);
        assert_eq!(sx, 32.0);  // TILE_W / 2
        assert_eq!(sy, 16.0);  // TILE_H / 2
    }

    #[test]
    fn isometric_y_goes_left_down() {
        let (sx, sy) = tile_to_screen(0, 1);
        assert_eq!(sx, -32.0);
        assert_eq!(sy, 16.0);
    }
}
