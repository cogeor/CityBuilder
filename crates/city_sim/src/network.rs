//! Road network graph.
//!
//! Lightweight road network supporting connectivity queries.

use city_core::TileCoord;
use std::collections::{HashMap, HashSet, VecDeque};

/// Road network as an adjacency list.
#[derive(Debug, Default)]
pub struct RoadNetwork {
    /// Adjacency: for each tile with a road, the set of connected road tiles.
    adjacency: HashMap<TileCoord, HashSet<TileCoord>>,
}

impl RoadNetwork {
    pub fn new() -> Self { Self::default() }

    /// Add a road segment between two adjacent tiles.
    pub fn add_segment(&mut self, a: TileCoord, b: TileCoord) {
        self.adjacency.entry(a).or_default().insert(b);
        self.adjacency.entry(b).or_default().insert(a);
    }

    /// Remove a road segment between two tiles.
    pub fn remove_segment(&mut self, a: TileCoord, b: TileCoord) {
        if let Some(set) = self.adjacency.get_mut(&a) {
            set.remove(&b);
            if set.is_empty() { self.adjacency.remove(&a); }
        }
        if let Some(set) = self.adjacency.get_mut(&b) {
            set.remove(&a);
            if set.is_empty() { self.adjacency.remove(&b); }
        }
    }

    /// Check if a tile has any road connections.
    pub fn has_road(&self, coord: TileCoord) -> bool {
        self.adjacency.contains_key(&coord)
    }

    /// Get neighbors of a road tile.
    pub fn neighbors(&self, coord: TileCoord) -> Option<&HashSet<TileCoord>> {
        self.adjacency.get(&coord)
    }

    /// BFS connectivity test: can we reach `to` from `from` via roads?
    pub fn is_connected(&self, from: TileCoord, to: TileCoord) -> bool {
        if from == to { return true; }
        if !self.has_road(from) || !self.has_road(to) { return false; }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(from);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = self.adjacency.get(&current) {
                for &neighbor in neighbors {
                    if neighbor == to { return true; }
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        false
    }

    /// Total number of road nodes.
    pub fn node_count(&self) -> usize { self.adjacency.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_query() {
        let mut net = RoadNetwork::new();
        let a = TileCoord::new(0, 0);
        let b = TileCoord::new(1, 0);
        net.add_segment(a, b);
        assert!(net.has_road(a));
        assert!(net.has_road(b));
        assert_eq!(net.node_count(), 2);
    }

    #[test]
    fn connectivity() {
        let mut net = RoadNetwork::new();
        let a = TileCoord::new(0, 0);
        let b = TileCoord::new(1, 0);
        let c = TileCoord::new(2, 0);
        let d = TileCoord::new(5, 5);

        net.add_segment(a, b);
        net.add_segment(b, c);
        assert!(net.is_connected(a, c));
        assert!(!net.is_connected(a, d));
    }

    #[test]
    fn remove_segment() {
        let mut net = RoadNetwork::new();
        let a = TileCoord::new(0, 0);
        let b = TileCoord::new(1, 0);
        let c = TileCoord::new(2, 0);
        net.add_segment(a, b);
        net.add_segment(b, c);
        net.remove_segment(a, b);
        assert!(!net.is_connected(a, c));
        assert!(net.is_connected(b, c));
    }
}
