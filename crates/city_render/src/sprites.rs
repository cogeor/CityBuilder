//! Procedural sprite atlas — generates simple building pixel art at runtime.
//!
//! Atlas layout: 8 sprites in a horizontal strip, each SPRITE_W × SPRITE_H pixels.
//! Sprite IDs:
//!   0 = empty (transparent)
//!   1 = small house (green roof, beige walls)
//!   2 = shop (blue awning, grey walls)
//!   3 = factory (brown, smokestack)
//!   4 = civic building (purple, dome)

/// Width of each sprite cell in the atlas.
pub const SPRITE_W: u32 = 64;
/// Height of each sprite cell in the atlas (tall for vertical buildings).
pub const SPRITE_H: u32 = 96;
/// Number of sprite slots in the atlas.
pub const SPRITE_COUNT: u32 = 8;
/// Total atlas width in pixels.
pub const ATLAS_W: u32 = SPRITE_W * SPRITE_COUNT;
/// Total atlas height in pixels.
pub const ATLAS_H: u32 = SPRITE_H;

/// Generate the sprite atlas as RGBA8 pixel data.
pub fn generate_atlas() -> Vec<u8> {
    let mut pixels = vec![0u8; (ATLAS_W * ATLAS_H * 4) as usize];

    draw_house(&mut pixels, 1);
    draw_shop(&mut pixels, 2);
    draw_factory(&mut pixels, 3);
    draw_civic(&mut pixels, 4);

    pixels
}

/// UV coordinates for a sprite ID: (u_min, v_min, u_max, v_max).
pub fn sprite_uvs(sprite_id: u32) -> [f32; 4] {
    assert!(
        sprite_id < SPRITE_COUNT,
        "sprite_id {} out of range (max {})",
        sprite_id,
        SPRITE_COUNT - 1
    );
    let u0 = sprite_id as f32 / SPRITE_COUNT as f32;
    let u1 = (sprite_id + 1) as f32 / SPRITE_COUNT as f32;
    [u0, 0.0, u1, 1.0]
}

/// Load atlas bytes from a raw RGBA file. Returns None if the file doesn't exist or can't be read.
fn load_atlas_raw(path: &str) -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read(path).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        None
    }
}

/// Load atlas from file if it exists with the correct size, otherwise generate procedurally.
pub fn generate_or_load_atlas() -> Vec<u8> {
    let atlas_path = "plugins/base.world/content/spriteset_buildings.rgba";
    let expected = (ATLAS_W * ATLAS_H * 4) as usize;
    if let Some(data) = load_atlas_raw(atlas_path) {
        if data.len() == expected {
            return data;
        }
        eprintln!(
            "Warning: {} has wrong size (expected {}, got {}), using procedural atlas",
            atlas_path,
            expected,
            data.len()
        );
    }
    generate_atlas()
}

// ─── Drawing helpers ─────────────────────────────────────────────────────────

fn set_pixel(pixels: &mut [u8], x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
    if x >= ATLAS_W || y >= ATLAS_H {
        return;
    }
    let idx = ((y * ATLAS_W + x) * 4) as usize;
    pixels[idx] = r;
    pixels[idx + 1] = g;
    pixels[idx + 2] = b;
    pixels[idx + 3] = a;
}

fn fill_rect(pixels: &mut [u8], x0: u32, y0: u32, w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) {
    for dy in 0..h {
        for dx in 0..w {
            set_pixel(pixels, x0 + dx, y0 + dy, r, g, b, a);
        }
    }
}

fn fill_rect_outline(pixels: &mut [u8], x0: u32, y0: u32, w: u32, h: u32, r: u8, g: u8, b: u8) {
    for dx in 0..w {
        set_pixel(pixels, x0 + dx, y0, r, g, b, 255);
        set_pixel(pixels, x0 + dx, y0 + h - 1, r, g, b, 255);
    }
    for dy in 0..h {
        set_pixel(pixels, x0, y0 + dy, r, g, b, 255);
        set_pixel(pixels, x0 + w - 1, y0 + dy, r, g, b, 255);
    }
}

fn fill_iso_diamond(pixels: &mut [u8], cx: u32, cy: u32, half_w: u32, half_h: u32, r: u8, g: u8, b: u8, a: u8) {
    for dy in 0..half_h * 2 {
        let y = cy + dy - half_h;
        let dist_from_center = if dy < half_h { half_h - dy } else { dy - half_h };
        let row_half_width = half_w * (half_h - dist_from_center) / half_h;
        for dx in 0..row_half_width * 2 {
            let x = cx + dx - row_half_width;
            set_pixel(pixels, x, y, r, g, b, a);
        }
    }
}

// ─── Sprite definitions ──────────────────────────────────────────────────────

/// Sprite slot offset
fn sx(slot: u32) -> u32 {
    slot * SPRITE_W
}

fn draw_house(pixels: &mut [u8], slot: u32) {
    let ox = sx(slot);
    // Isometric base (diamond at bottom)
    fill_iso_diamond(pixels, ox + 32, SPRITE_H - 8, 28, 8, 80, 120, 60, 255);

    // Walls — isometric box: left face and right face
    let wall_bottom = SPRITE_H - 16;
    let wall_top = 30;
    let wall_height = wall_bottom - wall_top;

    // Left wall (darker)
    for dy in 0..wall_height {
        let y = wall_top + dy;
        let progress = dy as f32 / wall_height as f32;
        let x_start = ox + 4 + (progress * 28.0) as u32;
        let x_end = ox + 32;
        for x in x_start..x_end {
            set_pixel(pixels, x, y, 210, 190, 150, 255);
        }
    }
    // Right wall (lighter)
    for dy in 0..wall_height {
        let y = wall_top + dy;
        let progress = dy as f32 / wall_height as f32;
        let x_end = ox + 60 - (progress * 28.0) as u32;
        let x_start = ox + 32;
        for x in x_start..x_end {
            set_pixel(pixels, x, y, 230, 215, 175, 255);
        }
    }

    // Roof — triangle/peak
    let roof_color = (90, 160, 75); // green roof
    for dy in 0..20 {
        let y = wall_top - 10 + dy;
        let half = 30 - dy * 30 / 20;
        for dx in 0..half * 2 {
            let x = ox + 32 + dx - half as u32;
            set_pixel(pixels, x, y, roof_color.0, roof_color.1, roof_color.2, 255);
        }
    }

    // Windows
    fill_rect(pixels, ox + 16, wall_top + 12, 6, 6, 180, 220, 240, 255);
    fill_rect(pixels, ox + 42, wall_top + 12, 6, 6, 180, 220, 240, 255);
    // Door
    fill_rect(pixels, ox + 28, wall_bottom - 14, 8, 14, 120, 80, 50, 255);
}

fn draw_shop(pixels: &mut [u8], slot: u32) {
    let ox = sx(slot);
    // Base diamond
    fill_iso_diamond(pixels, ox + 32, SPRITE_H - 8, 28, 8, 100, 100, 110, 255);

    let wall_bottom = SPRITE_H - 16;
    let wall_top = 35;
    let wall_height = wall_bottom - wall_top;

    // Left wall
    for dy in 0..wall_height {
        let y = wall_top + dy;
        let progress = dy as f32 / wall_height as f32;
        let x_start = ox + 6 + (progress * 26.0) as u32;
        for x in x_start..ox + 32 {
            set_pixel(pixels, x, y, 190, 190, 200, 255);
        }
    }
    // Right wall
    for dy in 0..wall_height {
        let y = wall_top + dy;
        let progress = dy as f32 / wall_height as f32;
        let x_end = ox + 58 - (progress * 26.0) as u32;
        for x in ox + 32..x_end {
            set_pixel(pixels, x, y, 210, 210, 220, 255);
        }
    }

    // Flat roof line
    fill_rect(pixels, ox + 6, wall_top - 3, 52, 3, 70, 70, 80, 255);

    // Awning (blue)
    fill_rect(pixels, ox + 6, wall_top, 52, 8, 60, 100, 180, 255);

    // Shop window (large)
    fill_rect(pixels, ox + 12, wall_top + 12, 40, 16, 160, 210, 230, 255);
    fill_rect_outline(pixels, ox + 12, wall_top + 12, 40, 16, 50, 50, 60);
    // Door
    fill_rect(pixels, ox + 28, wall_bottom - 14, 8, 14, 100, 70, 50, 255);
}

fn draw_factory(pixels: &mut [u8], slot: u32) {
    let ox = sx(slot);
    // Base diamond
    fill_iso_diamond(pixels, ox + 32, SPRITE_H - 8, 28, 8, 90, 80, 70, 255);

    let wall_bottom = SPRITE_H - 16;
    let wall_top = 28;
    let wall_height = wall_bottom - wall_top;

    // Main body (wide, industrial)
    for dy in 0..wall_height {
        let y = wall_top + dy;
        let progress = dy as f32 / wall_height as f32;
        let x_start = ox + 4 + (progress * 28.0) as u32;
        let x_end = ox + 60 - (progress * 28.0) as u32;
        for x in x_start..ox + 32 {
            set_pixel(pixels, x, y, 160, 140, 100, 255);
        }
        for x in ox + 32..x_end {
            set_pixel(pixels, x, y, 180, 160, 120, 255);
        }
    }

    // Flat roof
    fill_rect(pixels, ox + 4, wall_top - 2, 56, 2, 100, 90, 70, 255);

    // Smokestack
    fill_rect(pixels, ox + 46, 10, 8, wall_top - 10, 120, 110, 100, 255);
    // Smoke puff
    fill_rect(pixels, ox + 44, 6, 12, 6, 180, 180, 180, 150);

    // Industrial windows (row)
    for i in 0..4 {
        fill_rect(pixels, ox + 10 + i * 12, wall_top + 10, 8, 10, 200, 210, 180, 255);
    }
    // Loading door
    fill_rect(pixels, ox + 22, wall_bottom - 18, 16, 18, 100, 90, 70, 255);
}

fn draw_civic(pixels: &mut [u8], slot: u32) {
    let ox = sx(slot);
    // Base diamond
    fill_iso_diamond(pixels, ox + 32, SPRITE_H - 8, 28, 8, 120, 100, 140, 255);

    let wall_bottom = SPRITE_H - 16;
    let wall_top = 25;
    let wall_height = wall_bottom - wall_top;

    // Walls (marble-ish white)
    for dy in 0..wall_height {
        let y = wall_top + dy;
        let progress = dy as f32 / wall_height as f32;
        let x_start = ox + 6 + (progress * 26.0) as u32;
        let x_end = ox + 58 - (progress * 26.0) as u32;
        for x in x_start..ox + 32 {
            set_pixel(pixels, x, y, 220, 210, 230, 255);
        }
        for x in ox + 32..x_end {
            set_pixel(pixels, x, y, 235, 228, 240, 255);
        }
    }

    // Dome
    for dy in 0..16 {
        let y = wall_top - 14 + dy;
        let progress = dy as f32 / 16.0;
        let half = ((1.0 - (progress * 2.0 - 1.0).powi(2)).sqrt() * 18.0) as u32;
        for dx in 0..half * 2 {
            let x = ox + 32 + dx - half;
            set_pixel(pixels, x, y, 140, 120, 170, 255);
        }
    }

    // Columns
    for col in 0..4 {
        let cx = ox + 12 + col * 12;
        fill_rect(pixels, cx, wall_top + 4, 4, wall_height - 4, 200, 195, 210, 255);
    }

    // Door
    fill_rect(pixels, ox + 26, wall_bottom - 16, 12, 16, 140, 120, 100, 255);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_has_correct_size() {
        let atlas = generate_atlas();
        assert_eq!(atlas.len(), (ATLAS_W * ATLAS_H * 4) as usize);
    }

    #[test]
    fn empty_sprite_is_transparent() {
        let atlas = generate_atlas();
        // Slot 0 should be all transparent
        for y in 0..SPRITE_H {
            for x in 0..SPRITE_W {
                let idx = ((y * ATLAS_W + x) * 4 + 3) as usize;
                assert_eq!(atlas[idx], 0, "slot 0 pixel ({},{}) should be transparent", x, y);
            }
        }
    }

    #[test]
    fn house_sprite_has_opaque_pixels() {
        let atlas = generate_atlas();
        let mut opaque = 0u32;
        for y in 0..SPRITE_H {
            for x in 0..SPRITE_W {
                let idx = ((y * ATLAS_W + (SPRITE_W + x)) * 4 + 3) as usize;
                if atlas[idx] > 0 { opaque += 1; }
            }
        }
        assert!(opaque > 100, "house sprite should have many opaque pixels, got {}", opaque);
    }

    #[test]
    fn sprite_uvs_correct() {
        let uvs = sprite_uvs(1);
        assert_eq!(uvs[0], 1.0 / SPRITE_COUNT as f32);
        assert_eq!(uvs[2], 2.0 / SPRITE_COUNT as f32);
    }

    #[test]
    fn sprite_uvs_bounds_check() {
        for i in 0..SPRITE_COUNT {
            let uvs = sprite_uvs(i);
            assert!(uvs[0] >= 0.0 && uvs[2] <= 1.0);
        }
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn sprite_uvs_out_of_range_panics() {
        sprite_uvs(SPRITE_COUNT);
    }

    #[test]
    fn generate_or_load_atlas_produces_correct_size() {
        let atlas = generate_or_load_atlas();
        assert_eq!(atlas.len(), (ATLAS_W * ATLAS_H * 4) as usize);
    }
}
