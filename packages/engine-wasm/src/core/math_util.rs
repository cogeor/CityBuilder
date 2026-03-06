//! Shared spatial math utilities for the engine core.

/// Returns true when two axis-aligned rectangles overlap.
///
/// Each rectangle is given as its top-left corner (x, y) plus width and height
/// in tile units. Rectangles that only touch at an edge are *not* considered
/// overlapping (strict less-than comparisons).
pub fn rects_overlap(
    ax: i16,
    ay: i16,
    aw: i16,
    ah: i16,
    bx: i16,
    by: i16,
    bw: i16,
    bh: i16,
) -> bool {
    let a_right = ax + aw;
    let a_bottom = ay + ah;
    let b_right = bx + bw;
    let b_bottom = by + bh;
    ax < b_right && a_right > bx && ay < b_bottom && a_bottom > by
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_rects_return_true() {
        assert!(rects_overlap(0, 0, 2, 2, 1, 1, 2, 2));
    }

    #[test]
    fn adjacent_rects_do_not_overlap() {
        // Touching at right edge of A / left edge of B
        assert!(!rects_overlap(0, 0, 2, 2, 2, 0, 2, 2));
    }

    #[test]
    fn separated_rects_return_false() {
        assert!(!rects_overlap(0, 0, 1, 1, 5, 5, 1, 1));
    }

    #[test]
    fn identical_rects_overlap() {
        assert!(rects_overlap(3, 3, 2, 2, 3, 3, 2, 2));
    }

    #[test]
    fn contained_rect_overlaps() {
        // Small rect fully inside large rect
        assert!(rects_overlap(0, 0, 10, 10, 2, 2, 3, 3));
    }
}
