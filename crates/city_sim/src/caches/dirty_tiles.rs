/// Per-tile dirty bitset for analysis map invalidation.
///
/// Internally stores one bit per tile in `Vec<u64>` words.
#[derive(Debug, Clone)]
pub struct DirtyTileSet {
    words: Vec<u64>,
    width: u16,
    height: u16,
}

impl DirtyTileSet {
    pub fn new(width: u16, height: u16) -> Self {
        let tiles = width as usize * height as usize;
        let word_count = (tiles + 63) / 64;
        DirtyTileSet {
            words: vec![0u64; word_count],
            width,
            height,
        }
    }

    #[inline]
    pub fn mark(&mut self, x: i16, y: i16) {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return;
        }
        let t = y as usize * self.width as usize + x as usize;
        self.words[t / 64] |= 1u64 << (t % 64);
    }

    pub fn mark_region(&mut self, x: i16, y: i16, w: u8, h: u8) {
        for dy in 0..h as i16 {
            for dx in 0..w as i16 {
                self.mark(x + dx, y + dy);
            }
        }
    }

    pub fn mark_manhattan(&mut self, cx: i16, cy: i16, radius: u8) {
        let r = radius as i16;
        for dy in -r..=r {
            let rem = r - dy.abs();
            for dx in -rem..=rem {
                self.mark(cx + dx, cy + dy);
            }
        }
    }

    #[inline]
    pub fn is_set(&self, x: i16, y: i16) -> bool {
        if x < 0 || y < 0 || x >= self.width as i16 || y >= self.height as i16 {
            return false;
        }
        let t = y as usize * self.width as usize + x as usize;
        (self.words[t / 64] >> (t % 64)) & 1 != 0
    }

    pub fn any(&self) -> bool {
        self.words.iter().any(|&w| w != 0)
    }

    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    pub fn iter_dirty_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(wi, &word)| {
            DirtyBitsIter { word, base: wi * 64 }
        })
    }

    #[inline]
    pub fn width(&self) -> u16 { self.width }

    #[inline]
    pub fn height(&self) -> u16 { self.height }
}

struct DirtyBitsIter {
    word: u64,
    base: usize,
}

impl Iterator for DirtyBitsIter {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.word == 0 {
            return None;
        }
        let bit = self.word.trailing_zeros() as usize;
        self.word &= self.word - 1;
        Some(self.base + bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_all_clean() {
        let dts = DirtyTileSet::new(8, 8);
        for y in 0..8i16 {
            for x in 0..8i16 {
                assert!(!dts.is_set(x, y));
            }
        }
        assert!(!dts.any());
    }

    #[test]
    fn mark_single_tile() {
        let mut dts = DirtyTileSet::new(16, 16);
        dts.mark(5, 7);
        assert!(dts.is_set(5, 7));
        assert!(!dts.is_set(4, 7));
        assert!(!dts.is_set(6, 7));
        assert!(dts.any());
    }

    #[test]
    fn out_of_bounds_no_panic() {
        let mut dts = DirtyTileSet::new(8, 8);
        dts.mark(-1, 0);
        dts.mark(0, -1);
        dts.mark(8, 0);
        dts.mark(0, 8);
        assert!(!dts.any());
        assert!(!dts.is_set(-1, 0));
        assert!(!dts.is_set(100, 100));
    }

    #[test]
    fn clear_resets_all() {
        let mut dts = DirtyTileSet::new(4, 4);
        dts.mark(0, 0);
        dts.mark(3, 3);
        assert!(dts.any());
        dts.clear();
        assert!(!dts.any());
    }

    #[test]
    fn mark_region_covers_all_tiles() {
        let mut dts = DirtyTileSet::new(10, 10);
        dts.mark_region(2, 3, 3, 2);
        for dy in 0..2i16 {
            for dx in 0..3i16 {
                assert!(dts.is_set(2 + dx, 3 + dy));
            }
        }
        assert!(!dts.is_set(5, 3));
        assert!(!dts.is_set(2, 5));
    }

    #[test]
    fn iter_dirty_yields_all_marked() {
        let mut dts = DirtyTileSet::new(8, 8);
        dts.mark(0, 0);
        dts.mark(7, 7);
        dts.mark(3, 4);
        let expected: Vec<usize> = vec![0, 4 * 8 + 3, 7 * 8 + 7];
        let mut got: Vec<usize> = dts.iter_dirty_indices().collect();
        got.sort_unstable();
        assert_eq!(got, expected);
    }

    #[test]
    fn mark_manhattan_covers_diamond() {
        let mut dts = DirtyTileSet::new(10, 10);
        dts.mark_manhattan(5, 5, 2);
        let count = dts.iter_dirty_indices().count();
        assert_eq!(count, 13);
        assert!(dts.is_set(5, 5));
        assert!(dts.is_set(3, 5));
        assert!(dts.is_set(7, 5));
        assert!(!dts.is_set(3, 3));
    }

    #[test]
    fn word_boundary_tile_63_and_64() {
        let mut dts = DirtyTileSet::new(128, 128);
        dts.mark(63, 0);
        assert!(dts.is_set(63, 0));
        assert!(!dts.is_set(64, 0));
        dts.mark(64, 0);
        assert!(dts.is_set(64, 0));
        let indices: Vec<usize> = dts.iter_dirty_indices().collect();
        assert!(indices.contains(&63));
        assert!(indices.contains(&64));
    }
}
