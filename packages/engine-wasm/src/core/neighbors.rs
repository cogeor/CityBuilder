//! Compile-time neighbor offset arrays for 4- and 8-connectivity tile walks.

/// 4-connected neighbor offsets (N, S, E, W).
pub const NEIGHBORS_4: [(i16, i16); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];

/// 8-connected neighbor offsets (N, NE, E, SE, S, SW, W, NW).
pub const NEIGHBORS_8: [(i16, i16); 8] = [
    (0, 1), (1, 1), (1, 0), (1, -1),
    (0, -1), (-1, -1), (-1, 0), (-1, 1),
];
