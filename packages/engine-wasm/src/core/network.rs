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
}
