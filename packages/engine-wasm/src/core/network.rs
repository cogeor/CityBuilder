//! Road graph and network topology.
//!
//! Nodes are tile positions. Edges represent road segments with typed
//! properties (speed limit, capacity, cost). Supports connected component
//! detection for utility/transport networks.

use crate::core_types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Road classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RoadType {
    Local = 0,
    Collector = 1,
    Arterial = 2,
    Highway = 3,
}

impl RoadType {
    /// Speed limit in km/h.
    pub const fn speed_kmh(self) -> u32 {
        match self {
            RoadType::Local => 30,
            RoadType::Collector => 50,
            RoadType::Arterial => 70,
            RoadType::Highway => 100,
        }
    }

    /// Lane capacity (vehicles per game-hour).
    pub const fn capacity(self) -> u32 {
        match self {
            RoadType::Local => 200,
            RoadType::Collector => 500,
            RoadType::Arterial => 1000,
            RoadType::Highway => 2000,
        }
    }

    /// Construction cost in cents per tile.
    pub const fn cost_cents(self) -> MoneyCents {
        match self {
            RoadType::Local => 1000,
            RoadType::Collector => 2500,
            RoadType::Arterial => 5000,
            RoadType::Highway => 10000,
        }
    }
}

/// A road segment connecting two adjacent tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadSegment {
    pub road_type: RoadType,
}

/// Which cardinal directions a road tile connects to, encoded as a 4-bit NSEW mask.
///
/// Bit layout: `N=bit3  S=bit2  E=bit1  W=bit0`
///
/// The discriminant equals the mask value, making `ROAD_TABLE` a zero-cost array lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RoadTileVariant {
    Empty = 0b0000, //  0 – no connections
    W     = 0b0001, //  1 – west stub
    E     = 0b0010, //  2 – east stub
    EW    = 0b0011, //  3 – straight east–west
    S     = 0b0100, //  4 – south stub
    SW    = 0b0101, //  5 – south + west corner
    SE    = 0b0110, //  6 – south + east corner
    SEW   = 0b0111, //  7 – T south/east/west
    N     = 0b1000, //  8 – north stub
    NW    = 0b1001, //  9 – north + west corner
    NE    = 0b1010, // 10 – north + east corner
    NEW   = 0b1011, // 11 – T north/east/west
    NS    = 0b1100, // 12 – straight north–south
    NSW   = 0b1101, // 13 – T north/south/west
    NSE   = 0b1110, // 14 – T north/south/east
    Cross = 0b1111, // 15 – four-way intersection
}

/// Lookup table: index is the 4-bit NSEW mask, value is the corresponding variant.
///
/// ```
/// use engine_wasm::core::network::{ROAD_TABLE, RoadTileVariant};
/// assert_eq!(ROAD_TABLE[0b1111], RoadTileVariant::Cross);
/// assert_eq!(ROAD_TABLE[0b1100], RoadTileVariant::NS);
/// ```
pub const ROAD_TABLE: [RoadTileVariant; 16] = [
    RoadTileVariant::Empty, //  0
    RoadTileVariant::W,     //  1
    RoadTileVariant::E,     //  2
    RoadTileVariant::EW,    //  3
    RoadTileVariant::S,     //  4
    RoadTileVariant::SW,    //  5
    RoadTileVariant::SE,    //  6
    RoadTileVariant::SEW,   //  7
    RoadTileVariant::N,     //  8
    RoadTileVariant::NW,    //  9
    RoadTileVariant::NE,    // 10
    RoadTileVariant::NEW,   // 11
    RoadTileVariant::NS,    // 12
    RoadTileVariant::NSW,   // 13
    RoadTileVariant::NSE,   // 14
    RoadTileVariant::Cross, // 15
];

/// Key for an edge between two tile coordinates (normalized so a < b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EdgeKey {
    a: TileCoord,
    b: TileCoord,
}

impl EdgeKey {
    fn new(p1: TileCoord, p2: TileCoord) -> Self {
        // Normalize: smaller coordinate first
        if (p1.y, p1.x) <= (p2.y, p2.x) {
            EdgeKey { a: p1, b: p2 }
        } else {
            EdgeKey { a: p2, b: p1 }
        }
    }
}

/// Road network graph.
#[derive(Debug)]
pub struct RoadGraph {
    /// Adjacency list: for each node, its neighbors.
    adjacency: HashMap<TileCoord, Vec<(TileCoord, RoadType)>>,
    /// Edge data keyed by normalized coordinate pair.
    edges: HashMap<EdgeKey, RoadSegment>,
    /// Set of all road nodes.
    nodes: HashSet<TileCoord>,
}

impl RoadGraph {
    pub fn new() -> Self {
        RoadGraph {
            adjacency: HashMap::new(),
            edges: HashMap::new(),
            nodes: HashSet::new(),
        }
    }

    /// Add a road segment between two adjacent tiles.
    /// Returns true if the segment was added (false if it already exists).
    pub fn add_segment(&mut self, a: TileCoord, b: TileCoord, road_type: RoadType) -> bool {
        let key = EdgeKey::new(a, b);
        if self.edges.contains_key(&key) {
            return false;
        }
        self.edges.insert(key, RoadSegment { road_type });
        self.adjacency.entry(a).or_default().push((b, road_type));
        self.adjacency.entry(b).or_default().push((a, road_type));
        self.nodes.insert(a);
        self.nodes.insert(b);
        true
    }

    /// Remove a road segment. Returns the removed segment or None.
    pub fn remove_segment(&mut self, a: TileCoord, b: TileCoord) -> Option<RoadSegment> {
        let key = EdgeKey::new(a, b);
        let segment = self.edges.remove(&key)?;

        // Remove from adjacency
        if let Some(neighbors) = self.adjacency.get_mut(&a) {
            neighbors.retain(|(n, _)| *n != b);
            if neighbors.is_empty() {
                self.adjacency.remove(&a);
                self.nodes.remove(&a);
            }
        }
        if let Some(neighbors) = self.adjacency.get_mut(&b) {
            neighbors.retain(|(n, _)| *n != a);
            if neighbors.is_empty() {
                self.adjacency.remove(&b);
                self.nodes.remove(&b);
            }
        }

        Some(segment)
    }

    /// Check if a road exists at/through a tile.
    pub fn has_road_at(&self, pos: TileCoord) -> bool {
        self.nodes.contains(&pos)
    }

    /// Get all neighbors of a node.
    pub fn neighbors(&self, pos: TileCoord) -> &[(TileCoord, RoadType)] {
        match self.adjacency.get(&pos) {
            Some(v) => v,
            None => &[],
        }
    }

    /// Get the segment between two nodes.
    pub fn get_segment(&self, a: TileCoord, b: TileCoord) -> Option<&RoadSegment> {
        self.edges.get(&EdgeKey::new(a, b))
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Find connected components using BFS.
    /// Returns a list of components, each being a set of tile coordinates.
    pub fn connected_components(&self) -> Vec<HashSet<TileCoord>> {
        let mut visited: HashSet<TileCoord> = HashSet::new();
        let mut components = Vec::new();

        for &node in &self.nodes {
            if visited.contains(&node) {
                continue;
            }
            let mut component = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(node);
            visited.insert(node);

            while let Some(current) = queue.pop_front() {
                component.insert(current);
                for (neighbor, _) in self.neighbors(current) {
                    if visited.insert(*neighbor) {
                        queue.push_back(*neighbor);
                    }
                }
            }
            components.push(component);
        }
        components
    }

    /// Check if two nodes are in the same connected component.
    pub fn are_connected(&self, a: TileCoord, b: TileCoord) -> bool {
        if !self.nodes.contains(&a) || !self.nodes.contains(&b) {
            return false;
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(a);
        visited.insert(a);

        while let Some(current) = queue.pop_front() {
            if current == b {
                return true;
            }
            for (neighbor, _) in self.neighbors(current) {
                if visited.insert(*neighbor) {
                    queue.push_back(*neighbor);
                }
            }
        }
        false
    }

    /// Iterate over all nodes.
    pub fn iter_nodes(&self) -> impl Iterator<Item = &TileCoord> {
        self.nodes.iter()
    }

    /// Return the 4-bit NSEW connection mask for `pos`.
    ///
    /// Bit layout: `N=bit3  S=bit2  E=bit1  W=bit0`
    ///
    /// A bit is set when the corresponding neighbor tile is present in the road graph.
    /// Combine with `ROAD_TABLE` for tile-sprite selection:
    ///
    /// ```ignore
    /// let variant = ROAD_TABLE[graph.connection_bits(pos) as usize];
    /// ```
    pub fn connection_bits(&self, pos: TileCoord) -> u8 {
        let north = TileCoord::new(pos.x,     pos.y - 1);
        let south = TileCoord::new(pos.x,     pos.y + 1);
        let east  = TileCoord::new(pos.x + 1, pos.y    );
        let west  = TileCoord::new(pos.x - 1, pos.y    );

        let mut bits: u8 = 0;
        if self.has_road_at(north) { bits |= 0b1000; } // N = bit 3
        if self.has_road_at(south) { bits |= 0b0100; } // S = bit 2
        if self.has_road_at(east)  { bits |= 0b0010; } // E = bit 1
        if self.has_road_at(west)  { bits |= 0b0001; } // W = bit 0
        bits
    }

    /// Return `true` if any cardinal neighbor of `(x, y)` has a road node.
    ///
    /// Used by zone/building placement to verify road access without requiring
    /// the queried tile itself to be a road tile.
    pub fn has_road_access(&self, x: i16, y: i16) -> bool {
        let north = TileCoord::new(x,     y - 1);
        let south = TileCoord::new(x,     y + 1);
        let east  = TileCoord::new(x + 1, y    );
        let west  = TileCoord::new(x - 1, y    );

        self.has_road_at(north)
            || self.has_road_at(south)
            || self.has_road_at(east)
            || self.has_road_at(west)
    }
}

impl Default for RoadGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tc(x: i16, y: i16) -> TileCoord {
        TileCoord::new(x, y)
    }

    // ── add_segment creates edge and nodes ──────────────────────────────

    #[test]
    fn add_segment_creates_edge_and_nodes() {
        let mut g = RoadGraph::new();
        let added = g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        assert!(added);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.node_count(), 2);
        assert!(g.has_road_at(tc(0, 0)));
        assert!(g.has_road_at(tc(1, 0)));
    }

    // ── add_segment duplicate returns false ─────────────────────────────

    #[test]
    fn add_segment_duplicate_returns_false() {
        let mut g = RoadGraph::new();
        assert!(g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local));
        assert!(!g.add_segment(tc(0, 0), tc(1, 0), RoadType::Arterial));
        // Also test reversed order
        assert!(!g.add_segment(tc(1, 0), tc(0, 0), RoadType::Highway));
        assert_eq!(g.edge_count(), 1);
    }

    // ── remove_segment removes edge and orphaned nodes ──────────────────

    #[test]
    fn remove_segment_removes_edge_and_orphaned_nodes() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        let removed = g.remove_segment(tc(0, 0), tc(1, 0));
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().road_type, RoadType::Local);
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.node_count(), 0);
        assert!(!g.has_road_at(tc(0, 0)));
        assert!(!g.has_road_at(tc(1, 0)));
    }

    // ── remove_segment keeps non-orphaned nodes ─────────────────────────

    #[test]
    fn remove_segment_keeps_non_orphaned_nodes() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        g.add_segment(tc(1, 0), tc(2, 0), RoadType::Local);
        g.remove_segment(tc(0, 0), tc(1, 0));
        // (1,0) is still connected to (2,0), so it stays
        assert!(!g.has_road_at(tc(0, 0)));
        assert!(g.has_road_at(tc(1, 0)));
        assert!(g.has_road_at(tc(2, 0)));
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    }

    // ── remove_segment returns None for non-existent ────────────────────

    #[test]
    fn remove_segment_returns_none_for_nonexistent() {
        let mut g = RoadGraph::new();
        assert!(g.remove_segment(tc(0, 0), tc(1, 0)).is_none());
    }

    // ── has_road_at ─────────────────────────────────────────────────────

    #[test]
    fn has_road_at_empty_graph() {
        let g = RoadGraph::new();
        assert!(!g.has_road_at(tc(5, 5)));
    }

    #[test]
    fn has_road_at_with_roads() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(3, 4), tc(3, 5), RoadType::Collector);
        assert!(g.has_road_at(tc(3, 4)));
        assert!(g.has_road_at(tc(3, 5)));
        assert!(!g.has_road_at(tc(3, 6)));
    }

    // ── neighbors returns correct list ──────────────────────────────────

    #[test]
    fn neighbors_returns_correct_list() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(5, 5), tc(5, 6), RoadType::Local);
        g.add_segment(tc(5, 5), tc(6, 5), RoadType::Arterial);

        let neighbors = g.neighbors(tc(5, 5));
        assert_eq!(neighbors.len(), 2);

        let neighbor_coords: HashSet<TileCoord> =
            neighbors.iter().map(|(c, _)| *c).collect();
        assert!(neighbor_coords.contains(&tc(5, 6)));
        assert!(neighbor_coords.contains(&tc(6, 5)));
    }

    #[test]
    fn neighbors_empty_for_unknown_node() {
        let g = RoadGraph::new();
        assert!(g.neighbors(tc(0, 0)).is_empty());
    }

    // ── connected_components with single component ──────────────────────

    #[test]
    fn connected_components_single() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        g.add_segment(tc(1, 0), tc(2, 0), RoadType::Local);
        g.add_segment(tc(2, 0), tc(3, 0), RoadType::Local);

        let components = g.connected_components();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 4);
    }

    // ── connected_components with two disconnected components ────────────

    #[test]
    fn connected_components_two_disconnected() {
        let mut g = RoadGraph::new();
        // Component 1
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        // Component 2
        g.add_segment(tc(10, 10), tc(11, 10), RoadType::Highway);

        let components = g.connected_components();
        assert_eq!(components.len(), 2);

        // Each component has 2 nodes
        let mut sizes: Vec<usize> = components.iter().map(|c| c.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![2, 2]);
    }

    // ── are_connected positive and negative cases ───────────────────────

    #[test]
    fn are_connected_positive() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        g.add_segment(tc(1, 0), tc(2, 0), RoadType::Local);
        assert!(g.are_connected(tc(0, 0), tc(2, 0)));
    }

    #[test]
    fn are_connected_negative() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        g.add_segment(tc(10, 10), tc(11, 10), RoadType::Local);
        assert!(!g.are_connected(tc(0, 0), tc(10, 10)));
    }

    #[test]
    fn are_connected_nonexistent_node() {
        let g = RoadGraph::new();
        assert!(!g.are_connected(tc(0, 0), tc(1, 0)));
    }

    #[test]
    fn are_connected_same_node() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        assert!(g.are_connected(tc(0, 0), tc(0, 0)));
    }

    // ── edge_count and node_count ───────────────────────────────────────

    #[test]
    fn edge_count_and_node_count() {
        let mut g = RoadGraph::new();
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.node_count(), 0);

        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.node_count(), 2);

        g.add_segment(tc(1, 0), tc(2, 0), RoadType::Local);
        assert_eq!(g.edge_count(), 2);
        assert_eq!(g.node_count(), 3);

        g.add_segment(tc(0, 0), tc(0, 1), RoadType::Collector);
        assert_eq!(g.edge_count(), 3);
        assert_eq!(g.node_count(), 4);
    }

    // ── road type properties (speed, capacity, cost) ────────────────────

    #[test]
    fn road_type_speed() {
        assert_eq!(RoadType::Local.speed_kmh(), 30);
        assert_eq!(RoadType::Collector.speed_kmh(), 50);
        assert_eq!(RoadType::Arterial.speed_kmh(), 70);
        assert_eq!(RoadType::Highway.speed_kmh(), 100);
    }

    #[test]
    fn road_type_capacity() {
        assert_eq!(RoadType::Local.capacity(), 200);
        assert_eq!(RoadType::Collector.capacity(), 500);
        assert_eq!(RoadType::Arterial.capacity(), 1000);
        assert_eq!(RoadType::Highway.capacity(), 2000);
    }

    #[test]
    fn road_type_cost() {
        assert_eq!(RoadType::Local.cost_cents(), 1000);
        assert_eq!(RoadType::Collector.cost_cents(), 2500);
        assert_eq!(RoadType::Arterial.cost_cents(), 5000);
        assert_eq!(RoadType::Highway.cost_cents(), 10000);
    }

    // ── remove bridge disconnects components ────────────────────────────

    #[test]
    fn remove_bridge_disconnects_components() {
        let mut g = RoadGraph::new();
        // Build: A -- B -- C -- D
        //              (bridge)
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        g.add_segment(tc(1, 0), tc(2, 0), RoadType::Local); // bridge
        g.add_segment(tc(2, 0), tc(3, 0), RoadType::Local);

        assert!(g.are_connected(tc(0, 0), tc(3, 0)));
        assert_eq!(g.connected_components().len(), 1);

        // Remove the bridge
        g.remove_segment(tc(1, 0), tc(2, 0));

        assert!(!g.are_connected(tc(0, 0), tc(3, 0)));
        assert_eq!(g.connected_components().len(), 2);
    }

    // ── get_segment ─────────────────────────────────────────────────────

    #[test]
    fn get_segment_existing() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Arterial);
        let seg = g.get_segment(tc(0, 0), tc(1, 0));
        assert!(seg.is_some());
        assert_eq!(seg.unwrap().road_type, RoadType::Arterial);

        // Reversed order should also work
        let seg2 = g.get_segment(tc(1, 0), tc(0, 0));
        assert!(seg2.is_some());
        assert_eq!(seg2.unwrap().road_type, RoadType::Arterial);
    }

    #[test]
    fn get_segment_nonexistent() {
        let g = RoadGraph::new();
        assert!(g.get_segment(tc(0, 0), tc(1, 0)).is_none());
    }

    // ── iter_nodes ──────────────────────────────────────────────────────

    #[test]
    fn iter_nodes_returns_all_nodes() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        g.add_segment(tc(1, 0), tc(2, 0), RoadType::Local);

        let nodes: HashSet<TileCoord> = g.iter_nodes().copied().collect();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.contains(&tc(0, 0)));
        assert!(nodes.contains(&tc(1, 0)));
        assert!(nodes.contains(&tc(2, 0)));
    }

    // ── default ─────────────────────────────────────────────────────────

    #[test]
    fn default_creates_empty_graph() {
        let g = RoadGraph::default();
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.node_count(), 0);
    }

    // ── RoadTileVariant & ROAD_TABLE ────────────────────────────────────

    #[test]
    fn road_table_spot_checks() {
        assert_eq!(ROAD_TABLE[0],  RoadTileVariant::Empty);
        assert_eq!(ROAD_TABLE[3],  RoadTileVariant::EW);
        assert_eq!(ROAD_TABLE[12], RoadTileVariant::NS);
        assert_eq!(ROAD_TABLE[15], RoadTileVariant::Cross);
    }

    #[test]
    fn road_table_all_variants_reachable() {
        // Every variant appears exactly once in the table.
        use std::collections::HashSet;
        let set: HashSet<u8> = ROAD_TABLE.iter().map(|v| *v as u8).collect();
        assert_eq!(set.len(), 16);
    }

    // ── connection_bits ─────────────────────────────────────────────────

    #[test]
    fn connection_bits_isolated_tile() {
        let mut g = RoadGraph::new();
        // A single node has no neighbors, so bits should be 0.
        g.add_segment(tc(5, 5), tc(5, 6), RoadType::Local);
        // tc(5,5) has a neighbor at tc(5,6) which is South (y+1).
        assert_eq!(g.connection_bits(tc(5, 5)), 0b0100); // S bit
    }

    #[test]
    fn connection_bits_straight_ew() {
        let mut g = RoadGraph::new();
        // West -- Center -- East
        g.add_segment(tc(4, 0), tc(5, 0), RoadType::Local);
        g.add_segment(tc(5, 0), tc(6, 0), RoadType::Local);
        // Center sees East (bit1) and West (bit0).
        assert_eq!(g.connection_bits(tc(5, 0)), 0b0011); // EW
        assert_eq!(ROAD_TABLE[g.connection_bits(tc(5, 0)) as usize], RoadTileVariant::EW);
    }

    #[test]
    fn connection_bits_cross() {
        let mut g = RoadGraph::new();
        let center = tc(5, 5);
        g.add_segment(center, tc(5, 4), RoadType::Local); // N
        g.add_segment(center, tc(5, 6), RoadType::Local); // S
        g.add_segment(center, tc(6, 5), RoadType::Local); // E
        g.add_segment(center, tc(4, 5), RoadType::Local); // W
        assert_eq!(g.connection_bits(center), 0b1111);
        assert_eq!(ROAD_TABLE[g.connection_bits(center) as usize], RoadTileVariant::Cross);
    }

    #[test]
    fn connection_bits_empty_graph() {
        let g = RoadGraph::new();
        assert_eq!(g.connection_bits(tc(0, 0)), 0);
    }

    // ── has_road_access ─────────────────────────────────────────────────

    #[test]
    fn has_road_access_adjacent_north() {
        let mut g = RoadGraph::new();
        // Road one tile north of the queried position.
        g.add_segment(tc(3, 3), tc(4, 3), RoadType::Local);
        // Queried tile is (3,4): north neighbor (3,3) has a road.
        assert!(g.has_road_access(3, 4));
    }

    #[test]
    fn has_road_access_no_neighbors() {
        let mut g = RoadGraph::new();
        g.add_segment(tc(0, 0), tc(1, 0), RoadType::Local);
        // Position (10,10) has no road neighbors.
        assert!(!g.has_road_access(10, 10));
    }

    #[test]
    fn has_road_access_empty_graph() {
        let g = RoadGraph::new();
        assert!(!g.has_road_access(0, 0));
    }

    #[test]
    fn has_road_access_all_four_directions() {
        let mut g = RoadGraph::new();
        // Build roads in all four cardinal directions from (5,5).
        g.add_segment(tc(5, 4), tc(5, 3), RoadType::Local); // north
        g.add_segment(tc(5, 6), tc(5, 7), RoadType::Local); // south
        g.add_segment(tc(6, 5), tc(7, 5), RoadType::Local); // east
        g.add_segment(tc(4, 5), tc(3, 5), RoadType::Local); // west
        assert!(g.has_road_access(5, 5));
    }
}
