//! Stripe-walk iterator for zoned development.
//!
//! Replaces random (x,y) sampling with a deterministic scan that persists its
//! cursor across ticks. Advancing by a large prime avoids axis-aligned bias
//! while still covering every tile within one full pass.

use crate::tilemap::TileMap;
use crate::types::{ZoneDensity, ZoneType};

/// A persistent cursor that walks the tile map, returning zoned tiles that
/// match a given `(ZoneType, ZoneDensity)` pair.
#[derive(Debug, Clone)]
pub struct StripeWalkIter {
    map_width: u32,
    map_height: u32,
    cursor: u32,
    stride: u32,
}

impl StripeWalkIter {
    pub fn new(map_width: u16, map_height: u16) -> Self {
        Self {
            map_width: map_width as u32,
            map_height: map_height as u32,
            cursor: 0,
            stride: 7919,
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let result = iter.next_zoned(&map, ZoneType::Commercial, ZoneDensity::Low);
        assert!(result.is_none());
    }

    #[test]
    fn covers_all_tiles_on_small_map() {
        let map = zoned_map(4, 4, ZoneType::Industrial, ZoneDensity::Medium);
        let mut iter = StripeWalkIter::new(4, 4);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..16 {
            if let Some((x, y)) = iter.next_zoned(&map, ZoneType::Industrial, ZoneDensity::Medium) {
                seen.insert((x, y));
            }
        }
        assert_eq!(seen.len(), 16);
    }

    #[test]
    fn cursor_persists_across_calls() {
        let map = zoned_map(8, 8, ZoneType::Residential, ZoneDensity::High);
        let mut iter = StripeWalkIter::new(8, 8);
        let first = iter.next_zoned(&map, ZoneType::Residential, ZoneDensity::High);
        let second = iter.next_zoned(&map, ZoneType::Residential, ZoneDensity::High);
        assert!(first.is_some());
        assert!(second.is_some());
        assert_ne!(first, second);
    }
}
