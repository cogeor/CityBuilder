//! Stripe-walk iterator for zoned development.
//!
//! Replaces random (x,y) sampling with a deterministic scan that persists its
//! cursor across ticks. Advancing by a large prime avoids axis-aligned bias
//! while still covering every tile within one full pass, similar to SimCity's
//! scan-line development walk.

use crate::core::tilemap::TileMap;
use crate::core_types::{ZoneDensity, ZoneType};

/// A persistent cursor that walks the tile map, returning zoned tiles that
/// match a given `(ZoneType, ZoneDensity)` pair.
///
/// The cursor advances by a fixed prime stride so that successive calls fan
/// out across the map rather than clustering at the top-left corner.
/// After `map_width * map_height` steps, every tile has been visited exactly
/// once (since gcd(stride, map_area) == 1 for any prime stride < map_area).
#[derive(Debug, Clone)]
pub struct StripeWalkIter {
    map_width: u32,
    map_height: u32,
    /// Linear tile index of the next candidate.
    cursor: u32,
    /// Step size — must be coprime with `map_width * map_height`.
    /// 7919 is a prime large enough that it beats most realistic map areas.
    stride: u32,
}

impl StripeWalkIter {
    /// Create a new iterator for a map of the given dimensions.
    pub fn new(map_width: u16, map_height: u16) -> Self {
        Self {
            map_width: map_width as u32,
            map_height: map_height as u32,
            cursor: 0,
            stride: 7919,
        }
    }

    /// Advance the cursor and return the next tile position whose zone and
    /// density match the requested values.  Returns `None` if the entire map
    /// has been scanned without finding a matching tile.
    pub fn next_zoned(
        &mut self,
        tiles: &TileMap,
        zone: ZoneType,
        density: ZoneDensity,
    ) -> Option<(i16, i16)> {
        let total = self.map_width * self.map_height;
        if total == 0 {
            return None;
        }

        for _ in 0..total {
            self.cursor = (self.cursor + self.stride) % total;
            let x = self.cursor % self.map_width;
            let y = self.cursor / self.map_width;
            if let Some(tile) = tiles.get(x, y) {
                if tile.zone == zone && tile.density == density {
                    return Some((x as i16, y as i16));
                }
            }
        }
        None
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tilemap::TileMap;
    use crate::core_types::{ZoneDensity, ZoneType};

    fn zoned_map(w: u16, h: u16, zone: ZoneType, density: ZoneDensity) -> TileMap {
        let mut map = TileMap::new(w as u32, h as u32);
        for y in 0..h as u32 {
            for x in 0..w as u32 {
                map.set_zone(x, y, zone);
                map.set_density(x, y, density);
            }
        }
        map
    }

    #[test]
    fn finds_zoned_tile() {
        let map = zoned_map(8, 8, ZoneType::Residential, ZoneDensity::Low);
        let mut iter = StripeWalkIter::new(8, 8);
        let result = iter.next_zoned(&map, ZoneType::Residential, ZoneDensity::Low);
        assert!(result.is_some());
    }

    #[test]
    fn returns_none_when_no_match() {
        let map = zoned_map(8, 8, ZoneType::Residential, ZoneDensity::Low);
        let mut iter = StripeWalkIter::new(8, 8);
        // Ask for Commercial — none present
        let result = iter.next_zoned(&map, ZoneType::Commercial, ZoneDensity::Low);
        assert!(result.is_none());
    }

    #[test]
    fn covers_all_tiles_on_small_map() {
        // A 4x4 map — stride=7919, total=16. Since gcd(7919,16)=1, all 16 tiles
        // are visited in one full pass.
        let map = zoned_map(4, 4, ZoneType::Industrial, ZoneDensity::Medium);
        let mut iter = StripeWalkIter::new(4, 4);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..16 {
            if let Some((x, y)) = iter.next_zoned(&map, ZoneType::Industrial, ZoneDensity::Medium) {
                seen.insert((x, y));
            }
        }
        // Should have visited all 16 tiles
        assert_eq!(seen.len(), 16);
    }

    #[test]
    fn cursor_persists_across_calls() {
        let map = zoned_map(8, 8, ZoneType::Residential, ZoneDensity::High);
        let mut iter = StripeWalkIter::new(8, 8);
        let first = iter.next_zoned(&map, ZoneType::Residential, ZoneDensity::High);
        let second = iter.next_zoned(&map, ZoneType::Residential, ZoneDensity::High);
        // Cursor should have advanced — results should differ
        assert!(first.is_some());
        assert!(second.is_some());
        assert_ne!(first, second);
    }

    #[test]
    fn stripe_fill_guarantee() {
        // A 4-tile-wide zone on a 128x128 map should fill all available tiles
        // within 10 * num_tiles passes (stripe guarantee).
        let w: u16 = 32;
        let h: u16 = 4;
        let mut map = TileMap::new(128, 128);
        for y in 0..h as u32 {
            for x in 0..w as u32 {
                map.set_zone(x, y, ZoneType::Residential);
                map.set_density(x, y, ZoneDensity::Low);
            }
        }
        let mut iter = StripeWalkIter::new(128, 128);
        let mut found = std::collections::HashSet::new();
        let zone_count = w as usize * h as usize;
        // 10 development ticks worth of attempts = 10 * zone_count
        for _ in 0..(10 * zone_count) {
            if let Some((x, y)) = iter.next_zoned(&map, ZoneType::Residential, ZoneDensity::Low) {
                found.insert((x, y));
            }
        }
        assert_eq!(found.len(), zone_count, "all zone tiles should be visited");
    }
}
