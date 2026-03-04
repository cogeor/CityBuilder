//! Multi-source distance field computation.
//!
//! Computes shortest travel time from every road-connected tile to the
//! nearest source tile. Uses Dijkstra's algorithm on the road graph.
//!
//! Edge weights are in 1/256 tile-cost fixed-point units.
//! Supports time-sliced computation (partial updates per invocation).

use crate::core::network::{RoadGraph, RoadType};
use crate::core_types::*;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Maximum distance value (unreachable).
pub const UNREACHABLE: u32 = u32::MAX;

/// A distance field: per-tile travel-time cost.
/// Indexed linearly: `y * width + x`.
#[derive(Debug)]
pub struct DistanceField {
    data: Vec<u32>,
    width: u16,
    height: u16,
}

impl DistanceField {
    /// Create a new field filled with UNREACHABLE.
    pub fn new(size: MapSize) -> Self {
        let count = size.area() as usize;
        DistanceField {
            data: vec![UNREACHABLE; count],
            width: size.width,
            height: size.height,
        }
    }

    /// Get distance at (x, y). Returns UNREACHABLE for out-of-bounds.
    #[inline]
    pub fn get(&self, x: i16, y: i16) -> u32 {
        if x < 0 || y < 0 || x as u16 >= self.width || y as u16 >= self.height {
            return UNREACHABLE;
        }
        self.data[(y as usize) * (self.width as usize) + (x as usize)]
    }

    /// Set distance at (x, y). No-op for out-of-bounds.
    #[inline]
    #[allow(dead_code)]
    fn set(&mut self, x: i16, y: i16, value: u32) {
        if x >= 0 && y >= 0 && (x as u16) < self.width && (y as u16) < self.height {
            self.data[(y as usize) * (self.width as usize) + (x as usize)] = value;
        }
    }

    /// Get distance by linear index.
    #[inline]
    fn get_linear(&self, idx: usize) -> u32 {
        self.data[idx]
    }

    /// Set distance by linear index.
    #[inline]
    fn set_linear(&mut self, idx: usize, value: u32) {
        self.data[idx] = value;
    }

    /// Convert tile coordinates to a linear index. Returns None if out of bounds.
    #[inline]
    fn coord_to_index(&self, x: i16, y: i16) -> Option<usize> {
        if x >= 0 && y >= 0 && (x as u16) < self.width && (y as u16) < self.height {
            Some((y as usize) * (self.width as usize) + (x as usize))
        } else {
            None
        }
    }

    /// Reset all values to UNREACHABLE.
    pub fn clear(&mut self) {
        self.data.fill(UNREACHABLE);
    }

    /// Width of the field in tiles.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Height of the field in tiles.
    pub fn height(&self) -> u16 {
        self.height
    }
}

/// Compute edge weight between two adjacent tiles.
///
/// Weight is in 1/256 tile-cost fixed-point units.
/// Formula: weight = (1 tile) / (speed in tiles/tick) * 256
///        = 65536 * 256 / speed_q16
fn edge_weight(road_type: RoadType) -> u32 {
    // Speed in tiles/tick (Q16.16 fixed point)
    let speed_q16 = crate::core::scale::speed_kmh_to_tiles_per_tick_q16(road_type.speed_kmh());
    if speed_q16 <= 0 {
        return UNREACHABLE;
    }
    // weight = 1.0 / speed * 256
    // In Q16.16: speed_q16 = speed * 65536
    // So: weight = 256 / (speed_q16 / 65536) = 65536 * 256 / speed_q16
    let w = (65536u64 * 256) / (speed_q16 as u64);
    w.min(UNREACHABLE as u64) as u32
}

/// Compute a distance field using Dijkstra from multiple source tiles.
///
/// `sources` are the starting tiles (e.g., all hospital positions).
/// The road graph provides the topology and edge weights.
///
/// After computation, each tile reachable via the road graph has a
/// distance value representing travel time to the nearest source.
/// Tiles not reachable remain at `UNREACHABLE`.
pub fn compute_distance_field(
    field: &mut DistanceField,
    sources: &[TileCoord],
    graph: &RoadGraph,
) {
    field.clear();

    // Priority queue: (distance, linear_index)
    // Using linear index avoids needing Ord on TileCoord.
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();

    // Initialize sources at distance 0
    for &src in sources {
        if let Some(idx) = field.coord_to_index(src.x, src.y) {
            field.set_linear(idx, 0);
            heap.push(Reverse((0, idx)));
        }
    }

    let w = field.width as usize;

    // Dijkstra's algorithm
    while let Some(Reverse((dist, idx))) = heap.pop() {
        // Skip if we have already found a shorter path
        if dist > field.get_linear(idx) {
            continue;
        }

        // Reconstruct TileCoord from linear index
        let pos = TileCoord::new((idx % w) as i16, (idx / w) as i16);

        for &(neighbor, road_type) in graph.neighbors(pos) {
            let weight = edge_weight(road_type);
            let new_dist = dist.saturating_add(weight);

            if let Some(n_idx) = field.coord_to_index(neighbor.x, neighbor.y) {
                if new_dist < field.get_linear(n_idx) {
                    field.set_linear(n_idx, new_dist);
                    heap.push(Reverse((new_dist, n_idx)));
                }
            }
        }
    }
}

/// Compute a partial distance field update (time-sliced).
///
/// Performs a full Dijkstra recompute but only copies results for tiles
/// in the scan window `[scan_start .. scan_start + scan_size]` (by linear
/// index). This allows spreading recomputation over multiple frames.
///
/// Returns the number of tiles updated.
pub fn compute_partial(
    field: &mut DistanceField,
    sources: &[TileCoord],
    graph: &RoadGraph,
    scan_start: usize,
    scan_size: usize,
) -> usize {
    let total = field.data.len();
    let end = (scan_start + scan_size).min(total);

    // Build a temporary full field
    let mut temp = DistanceField::new(MapSize::new(field.width, field.height));
    compute_distance_field(&mut temp, sources, graph);

    // Copy only the scan window
    let mut count = 0;
    for i in scan_start..end {
        field.data[i] = temp.data[i];
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network::RoadGraph;

    fn tc(x: i16, y: i16) -> TileCoord {
        TileCoord::new(x, y)
    }

    /// Helper: build a linear road of `count` tiles starting at (start_x, y)
    /// going rightward, using the given road type.
    fn build_linear_road(
        graph: &mut RoadGraph,
        start_x: i16,
        y: i16,
        count: i16,
        road_type: RoadType,
    ) {
        for i in 0..count - 1 {
            graph.add_segment(tc(start_x + i, y), tc(start_x + i + 1, y), road_type);
        }
    }

    // ── Single source, linear road: distances increase correctly ─────────

    #[test]
    fn single_source_linear_road() {
        let size = MapSize::new(8, 1);
        let mut graph = RoadGraph::new();
        build_linear_road(&mut graph, 0, 0, 8, RoadType::Local);

        let mut field = DistanceField::new(size);
        compute_distance_field(&mut field, &[tc(0, 0)], &graph);

        // Source tile has distance 0
        assert_eq!(field.get(0, 0), 0);

        // Each subsequent tile should have strictly increasing distance
        let w = edge_weight(RoadType::Local);
        for i in 1..8i16 {
            assert_eq!(
                field.get(i, 0),
                w * (i as u32),
                "tile ({}, 0) should be {} * {} = {}",
                i,
                w,
                i,
                w * (i as u32)
            );
        }
    }

    // ── Multiple sources: each tile gets distance to nearest source ──────

    #[test]
    fn multiple_sources_nearest_distance() {
        let size = MapSize::new(10, 1);
        let mut graph = RoadGraph::new();
        build_linear_road(&mut graph, 0, 0, 10, RoadType::Local);

        // Two sources: tile 0 and tile 9
        let mut field = DistanceField::new(size);
        compute_distance_field(&mut field, &[tc(0, 0), tc(9, 0)], &graph);

        let w = edge_weight(RoadType::Local);

        // Tile 0 and tile 9 are sources
        assert_eq!(field.get(0, 0), 0);
        assert_eq!(field.get(9, 0), 0);

        // Tile 4 is 4 edges from source 0, 5 edges from source 9 -> dist = 4*w
        assert_eq!(field.get(4, 0), 4 * w);

        // Tile 5 is 5 edges from source 0, 4 edges from source 9 -> dist = 4*w
        assert_eq!(field.get(5, 0), 4 * w);

        // Tile 1 is 1 edge from source 0 -> dist = w
        assert_eq!(field.get(1, 0), w);

        // Tile 8 is 1 edge from source 9 -> dist = w
        assert_eq!(field.get(8, 0), w);
    }

    // ── Unreachable tiles stay at UNREACHABLE ────────────────────────────

    #[test]
    fn unreachable_tiles_stay_unreachable() {
        let size = MapSize::new(8, 2);
        let mut graph = RoadGraph::new();
        // Road only on row 0
        build_linear_road(&mut graph, 0, 0, 8, RoadType::Local);

        let mut field = DistanceField::new(size);
        compute_distance_field(&mut field, &[tc(0, 0)], &graph);

        // Row 0 tiles are reachable
        assert_eq!(field.get(0, 0), 0);
        assert!(field.get(3, 0) < UNREACHABLE);

        // Row 1 tiles have no roads and are unreachable
        for x in 0..8i16 {
            assert_eq!(
                field.get(x, 1),
                UNREACHABLE,
                "tile ({}, 1) should be UNREACHABLE",
                x
            );
        }
    }

    // ── Edge weights differ by road type (highway faster than local) ─────

    #[test]
    fn edge_weights_differ_by_road_type() {
        let w_local = edge_weight(RoadType::Local);
        let w_collector = edge_weight(RoadType::Collector);
        let w_arterial = edge_weight(RoadType::Arterial);
        let w_highway = edge_weight(RoadType::Highway);

        // Faster roads should have lower edge weights
        assert!(
            w_highway < w_arterial,
            "highway {} should be < arterial {}",
            w_highway,
            w_arterial
        );
        assert!(
            w_arterial < w_collector,
            "arterial {} should be < collector {}",
            w_arterial,
            w_collector
        );
        assert!(
            w_collector < w_local,
            "collector {} should be < local {}",
            w_collector,
            w_local
        );

        // All weights should be positive
        assert!(w_local > 0);
        assert!(w_highway > 0);
    }

    #[test]
    fn highway_produces_shorter_distance_than_local() {
        // Build two separate 5-tile roads: one local, one highway
        let size = MapSize::new(10, 2);
        let mut graph = RoadGraph::new();
        build_linear_road(&mut graph, 0, 0, 5, RoadType::Local);
        build_linear_road(&mut graph, 0, 1, 5, RoadType::Highway);

        let mut field = DistanceField::new(size);
        // Source at (0,0) and (0,1) separately
        compute_distance_field(&mut field, &[tc(0, 0), tc(0, 1)], &graph);

        // Tile (4,0) via local road vs tile (4,1) via highway
        let dist_local = field.get(4, 0);
        let dist_highway = field.get(4, 1);

        assert!(
            dist_highway < dist_local,
            "highway dist {} should be < local dist {}",
            dist_highway,
            dist_local
        );
    }

    // ── Empty sources: all tiles unreachable ─────────────────────────────

    #[test]
    fn empty_sources_all_unreachable() {
        let size = MapSize::new(4, 4);
        let mut graph = RoadGraph::new();
        build_linear_road(&mut graph, 0, 0, 4, RoadType::Local);

        let mut field = DistanceField::new(size);
        compute_distance_field(&mut field, &[], &graph);

        for y in 0..4i16 {
            for x in 0..4i16 {
                assert_eq!(
                    field.get(x, y),
                    UNREACHABLE,
                    "tile ({}, {}) should be UNREACHABLE with no sources",
                    x,
                    y
                );
            }
        }
    }

    // ── Disconnected components: tiles in other component unreachable ────

    #[test]
    fn disconnected_components_unreachable() {
        let size = MapSize::new(10, 1);
        let mut graph = RoadGraph::new();
        // Component A: tiles 0..3
        build_linear_road(&mut graph, 0, 0, 4, RoadType::Local);
        // Component B: tiles 6..9 (no connection to A)
        build_linear_road(&mut graph, 6, 0, 4, RoadType::Local);

        let mut field = DistanceField::new(size);
        // Source only in component A
        compute_distance_field(&mut field, &[tc(0, 0)], &graph);

        // Component A tiles are reachable
        assert_eq!(field.get(0, 0), 0);
        assert!(field.get(3, 0) < UNREACHABLE);

        // Component B tiles are unreachable
        for x in 6..10i16 {
            assert_eq!(
                field.get(x, 0),
                UNREACHABLE,
                "tile ({}, 0) in disconnected component should be UNREACHABLE",
                x
            );
        }

        // Gap tiles (4, 5) with no roads are also unreachable
        assert_eq!(field.get(4, 0), UNREACHABLE);
        assert_eq!(field.get(5, 0), UNREACHABLE);
    }

    // ── compute_partial updates only scan window ─────────────────────────

    #[test]
    fn compute_partial_updates_only_scan_window() {
        let size = MapSize::new(8, 1);
        let mut graph = RoadGraph::new();
        build_linear_road(&mut graph, 0, 0, 8, RoadType::Local);

        let mut field = DistanceField::new(size);

        // Partially compute: only tiles [2..5)
        let count = compute_partial(&mut field, &[tc(0, 0)], &graph, 2, 3);
        assert_eq!(count, 3);

        let w = edge_weight(RoadType::Local);

        // Tiles 0, 1 are outside scan window -> still UNREACHABLE
        assert_eq!(field.get(0, 0), UNREACHABLE);
        assert_eq!(field.get(1, 0), UNREACHABLE);

        // Tiles 2, 3, 4 are in scan window -> correctly computed
        assert_eq!(field.get(2, 0), 2 * w);
        assert_eq!(field.get(3, 0), 3 * w);
        assert_eq!(field.get(4, 0), 4 * w);

        // Tiles 5, 6, 7 are outside scan window -> still UNREACHABLE
        assert_eq!(field.get(5, 0), UNREACHABLE);
        assert_eq!(field.get(6, 0), UNREACHABLE);
        assert_eq!(field.get(7, 0), UNREACHABLE);
    }

    #[test]
    fn compute_partial_scan_past_end() {
        let size = MapSize::new(4, 1);
        let mut graph = RoadGraph::new();
        build_linear_road(&mut graph, 0, 0, 4, RoadType::Local);

        let mut field = DistanceField::new(size);

        // Scan window extends past the end
        let count = compute_partial(&mut field, &[tc(0, 0)], &graph, 2, 100);
        // Should only update tiles 2, 3 (total 4 tiles, indices 2..4)
        assert_eq!(count, 2);

        assert_eq!(field.get(0, 0), UNREACHABLE);
        assert_eq!(field.get(1, 0), UNREACHABLE);

        let w = edge_weight(RoadType::Local);
        assert_eq!(field.get(2, 0), 2 * w);
        assert_eq!(field.get(3, 0), 3 * w);
    }

    // ── clear resets everything ──────────────────────────────────────────

    #[test]
    fn clear_resets_everything() {
        let size = MapSize::new(4, 4);
        let mut field = DistanceField::new(size);

        // Manually set some values
        field.set(0, 0, 0);
        field.set(1, 0, 100);
        field.set(2, 1, 200);

        assert_eq!(field.get(0, 0), 0);
        assert_eq!(field.get(1, 0), 100);
        assert_eq!(field.get(2, 1), 200);

        field.clear();

        // All values should be UNREACHABLE again
        for y in 0..4i16 {
            for x in 0..4i16 {
                assert_eq!(
                    field.get(x, y),
                    UNREACHABLE,
                    "tile ({}, {}) should be UNREACHABLE after clear",
                    x,
                    y
                );
            }
        }
    }

    // ── DistanceField::new initializes to UNREACHABLE ────────────────────

    #[test]
    fn new_field_all_unreachable() {
        let size = MapSize::new(3, 3);
        let field = DistanceField::new(size);

        for y in 0..3i16 {
            for x in 0..3i16 {
                assert_eq!(field.get(x, y), UNREACHABLE);
            }
        }
    }

    // ── Out-of-bounds access returns UNREACHABLE ─────────────────────────

    #[test]
    fn out_of_bounds_returns_unreachable() {
        let size = MapSize::new(4, 4);
        let field = DistanceField::new(size);

        assert_eq!(field.get(-1, 0), UNREACHABLE);
        assert_eq!(field.get(0, -1), UNREACHABLE);
        assert_eq!(field.get(4, 0), UNREACHABLE);
        assert_eq!(field.get(0, 4), UNREACHABLE);
        assert_eq!(field.get(100, 100), UNREACHABLE);
    }

    // ── width() and height() accessors ───────────────────────────────────

    #[test]
    fn width_and_height_accessors() {
        let field = DistanceField::new(MapSize::new(10, 20));
        assert_eq!(field.width(), 10);
        assert_eq!(field.height(), 20);
    }

    // ── Source not on road graph still sets distance 0 ───────────────────

    #[test]
    fn source_not_on_road_graph() {
        let size = MapSize::new(4, 4);
        let graph = RoadGraph::new(); // empty graph

        let mut field = DistanceField::new(size);
        compute_distance_field(&mut field, &[tc(1, 1)], &graph);

        // Source tile gets distance 0 even without roads
        assert_eq!(field.get(1, 1), 0);

        // Other tiles remain unreachable since there are no roads
        assert_eq!(field.get(0, 0), UNREACHABLE);
        assert_eq!(field.get(2, 2), UNREACHABLE);
    }

    // ── 2D grid with branching paths ─────────────────────────────────────

    #[test]
    fn two_dimensional_grid() {
        // 3x3 grid with roads forming a cross pattern:
        //   (1,0)
        //    |
        // (0,1) - (1,1) - (2,1)
        //    |
        //   (1,2)
        let size = MapSize::new(3, 3);
        let mut graph = RoadGraph::new();
        graph.add_segment(tc(1, 0), tc(1, 1), RoadType::Local);
        graph.add_segment(tc(0, 1), tc(1, 1), RoadType::Local);
        graph.add_segment(tc(1, 1), tc(2, 1), RoadType::Local);
        graph.add_segment(tc(1, 1), tc(1, 2), RoadType::Local);

        let mut field = DistanceField::new(size);
        compute_distance_field(&mut field, &[tc(1, 1)], &graph);

        let w = edge_weight(RoadType::Local);

        assert_eq!(field.get(1, 1), 0);
        assert_eq!(field.get(1, 0), w);
        assert_eq!(field.get(0, 1), w);
        assert_eq!(field.get(2, 1), w);
        assert_eq!(field.get(1, 2), w);

        // Corners not connected
        assert_eq!(field.get(0, 0), UNREACHABLE);
        assert_eq!(field.get(2, 0), UNREACHABLE);
        assert_eq!(field.get(0, 2), UNREACHABLE);
        assert_eq!(field.get(2, 2), UNREACHABLE);
    }
}
