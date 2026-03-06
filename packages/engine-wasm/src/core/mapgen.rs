//! Procedural map generation -- noise-based terrain.
//!
//! Provides an `IMapGenerator` trait for pluggable terrain generation algorithms
//! and a `DefaultMapGenerator` implementation using simplex-like hash noise.
//! All generation is seeded and deterministic: same seed + params = identical map.

use crate::core::tilemap::TileMap;
use crate::core_types::TerrainType;

// ---- Map Generation Parameters ------------------------------------------------

/// Map generation parameters controlling terrain characteristics.
#[derive(Debug, Clone)]
pub struct MapGenParams {
    /// Fraction of the map that should be water (0.0-1.0).
    pub water_ratio: f32,
    /// How hilly/mountainous the terrain is (0.0-1.0).
    pub hilliness: f32,
    /// Number of rivers to generate.
    pub river_count: u8,
    /// Number of smoothing passes for coastlines.
    pub coast_smoothing: u8,
}

impl Default for MapGenParams {
    fn default() -> Self {
        MapGenParams {
            water_ratio: 0.3,
            hilliness: 0.5,
            river_count: 2,
            coast_smoothing: 3,
        }
    }
}

// ---- Terrain Types -----------------------------------------------------------

/// Terrain types for generated tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GenTerrain {
    DeepWater = 0,
    ShallowWater = 1,
    Sand = 2,
    Grass = 3,
    Forest = 4,
    Hill = 5,
    Mountain = 6,
    River = 7,
}

impl From<GenTerrain> for TerrainType {
    fn from(g: GenTerrain) -> Self {
        match g {
            GenTerrain::DeepWater    => TerrainType::Water,
            GenTerrain::ShallowWater => TerrainType::Water,
            GenTerrain::River        => TerrainType::Water,
            GenTerrain::Sand         => TerrainType::Sand,
            GenTerrain::Grass        => TerrainType::Grass,
            GenTerrain::Forest       => TerrainType::Forest,
            GenTerrain::Hill         => TerrainType::Rock,
            GenTerrain::Mountain     => TerrainType::Rock,
        }
    }
}

// ---- Generated Tile ----------------------------------------------------------

/// Per-tile output from the map generator.
#[derive(Debug, Clone)]
pub struct GeneratedTile {
    /// Terrain classification.
    pub terrain: GenTerrain,
    /// Elevation value (0-65535).
    pub elevation: u16,
    /// Moisture value (0-255).
    pub moisture: u8,
}

// ---- Generated Map -----------------------------------------------------------

/// Complete map generation result.
#[derive(Debug, Clone)]
pub struct GeneratedMap {
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<GeneratedTile>,
}

impl GeneratedMap {
    /// Get a tile reference at (x, y). Returns None if out of bounds.
    #[inline]
    pub fn get(&self, x: u16, y: u16) -> Option<&GeneratedTile> {
        if x < self.width && y < self.height {
            Some(&self.tiles[(y as usize) * (self.width as usize) + (x as usize)])
        } else {
            None
        }
    }
}

// ---- Trait -------------------------------------------------------------------

/// Trait for pluggable terrain generation algorithms.
pub trait IMapGenerator {
    /// Generate a map with the given seed, dimensions, and parameters.
    fn generate(&self, seed: u64, width: u16, height: u16, params: &MapGenParams) -> GeneratedMap;
}

// ---- Seeded Hash Noise -------------------------------------------------------

/// Simple hash function for seeded pseudo-random values.
/// Combines seed with coordinates to produce a deterministic u64.
#[inline]
fn hash(seed: u64, x: i32, y: i32) -> u64 {
    // Based on a simple integer hash (splitmix-like mixing).
    let mut h = seed
        .wrapping_add((x as u64).wrapping_mul(0x9E3779B97F4A7C15))
        .wrapping_add((y as u64).wrapping_mul(0x6C62272E07BB0142));
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D049BB133111EB);
    h ^ (h >> 31)
}

/// Convert a hash value to a float in the range [0.0, 1.0).
#[inline]
fn hash_to_unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Smooth interpolation (cubic Hermite / smoothstep).
#[inline]
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 2D value noise sampled at floating-point coordinates.
/// Returns a value in the range -1.0 to 1.0.
fn noise2d(seed: u64, x: f32, y: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;

    let sx = smoothstep(fx);
    let sy = smoothstep(fy);

    let v00 = hash_to_unit(hash(seed, ix, iy));
    let v10 = hash_to_unit(hash(seed, ix + 1, iy));
    let v01 = hash_to_unit(hash(seed, ix, iy + 1));
    let v11 = hash_to_unit(hash(seed, ix + 1, iy + 1));

    let top = v00 + sx * (v10 - v00);
    let bot = v01 + sx * (v11 - v01);
    let val = top + sy * (bot - top);

    // Map from [0,1] to [-1,1]
    val * 2.0 - 1.0
}

/// Multi-octave fractal noise for richer detail.
/// Returns a value approximately in -1.0 to 1.0 (may slightly exceed due to
/// octave summation, so we clamp).
fn octave_noise(seed: u64, x: f32, y: f32, octaves: u8) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut max_amplitude = 0.0f32;

    for i in 0..octaves {
        // Offset the seed per octave to avoid correlation.
        let oct_seed = seed.wrapping_add(i as u64 * 31337);
        value += noise2d(oct_seed, x * frequency, y * frequency) * amplitude;
        max_amplitude += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    if max_amplitude > 0.0 {
        value /= max_amplitude;
    }

    value.clamp(-1.0, 1.0)
}

// ---- Elevation Generation ----------------------------------------------------

/// Generate an elevation grid using octave noise.
/// Returns a Vec of u16 values in the range 0-65535.
fn generate_elevation(seed: u64, w: u16, h: u16, hilliness: f32) -> Vec<u16> {
    let count = w as usize * h as usize;
    let mut elevation = Vec::with_capacity(count);

    // Scale determines how "zoomed in" the noise is.
    // Higher hilliness = higher octave count and more variation.
    let octaves = 3 + (hilliness * 4.0) as u8; // 3-7 octaves
    let scale = 0.02 + hilliness * 0.03; // frequency scaling

    for y in 0..h {
        for x in 0..w {
            let nx = x as f32 * scale;
            let ny = y as f32 * scale;
            let n = octave_noise(seed, nx, ny, octaves);
            // Map from [-1,1] to [0, 65535]
            let normalized = (n + 1.0) * 0.5; // [0, 1]
            let e = (normalized * 65535.0).clamp(0.0, 65535.0) as u16;
            elevation.push(e);
        }
    }

    elevation
}

// ---- Moisture Generation -----------------------------------------------------

/// Generate a moisture grid using noise with a different seed offset.
fn generate_moisture(seed: u64, w: u16, h: u16) -> Vec<u8> {
    let moisture_seed = seed.wrapping_add(0xDEAD_BEEF_CAFE_1234);
    let count = w as usize * h as usize;
    let mut moisture = Vec::with_capacity(count);

    for y in 0..h {
        for x in 0..w {
            let nx = x as f32 * 0.03;
            let ny = y as f32 * 0.03;
            let n = octave_noise(moisture_seed, nx, ny, 4);
            let normalized = (n + 1.0) * 0.5; // [0, 1]
            let m = (normalized * 255.0).clamp(0.0, 255.0) as u8;
            moisture.push(m);
        }
    }

    moisture
}

// ---- Terrain Assignment ------------------------------------------------------

/// Determine the water elevation threshold such that approximately `water_ratio`
/// fraction of tiles are below the threshold.
fn compute_water_threshold(elevation: &[u16], water_ratio: f32) -> u16 {
    if water_ratio <= 0.0 {
        return 0;
    }
    if water_ratio >= 1.0 {
        return u16::MAX;
    }

    // Sort a copy of elevations to find the percentile.
    let mut sorted: Vec<u16> = elevation.to_vec();
    sorted.sort_unstable();

    let index = ((sorted.len() as f32) * water_ratio).min(sorted.len() as f32 - 1.0) as usize;
    sorted[index]
}

/// Assign a terrain type based on elevation, moisture, and water threshold.
fn assign_terrain(elevation: u16, moisture: u8, water_threshold: u16) -> GenTerrain {
    if elevation <= water_threshold {
        // Below water threshold: split into deep and shallow.
        if water_threshold > 0 {
            let depth_ratio = elevation as f32 / water_threshold as f32;
            if depth_ratio < 0.5 {
                GenTerrain::DeepWater
            } else {
                GenTerrain::ShallowWater
            }
        } else {
            GenTerrain::DeepWater
        }
    } else {
        // Land: assign based on elevation and moisture.
        let land_range = 65535u32.saturating_sub(water_threshold as u32);
        if land_range == 0 {
            return GenTerrain::Grass;
        }
        let land_ratio = (elevation as u32 - water_threshold as u32) as f32 / land_range as f32;

        if land_ratio < 0.05 {
            GenTerrain::Sand
        } else if land_ratio > 0.85 {
            GenTerrain::Mountain
        } else if land_ratio > 0.65 {
            GenTerrain::Hill
        } else if moisture > 150 {
            GenTerrain::Forest
        } else {
            GenTerrain::Grass
        }
    }
}

// ---- River Generation --------------------------------------------------------

/// Trace a river downhill from a starting position.
/// Returns a list of (x, y) tile coordinates forming the river path.
fn trace_river(elevation: &[u16], w: u16, h: u16, start_x: u16, start_y: u16) -> Vec<(u16, u16)> {
    let mut path = Vec::new();
    let mut cx = start_x;
    let mut cy = start_y;

    // Limit iterations to prevent infinite loops on flat terrain.
    let max_steps = (w as usize + h as usize) * 2;

    for _ in 0..max_steps {
        path.push((cx, cy));

        // Find the neighbor with the lowest elevation.
        let current_elev = elevation[cy as usize * w as usize + cx as usize];
        let mut best_x = cx;
        let mut best_y = cy;
        let mut best_elev = current_elev;

        let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dx, dy) in neighbors {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                let ne = elevation[ny as usize * w as usize + nx as usize];
                if ne < best_elev {
                    best_elev = ne;
                    best_x = nx as u16;
                    best_y = ny as u16;
                }
            }
        }

        // If no downhill neighbor, stop (we reached a local minimum).
        if best_x == cx && best_y == cy {
            break;
        }

        cx = best_x;
        cy = best_y;

        // Stop if we reached the edge of the map.
        if cx == 0 || cy == 0 || cx == w - 1 || cy == h - 1 {
            path.push((cx, cy));
            break;
        }
    }

    path
}

/// Find high-elevation starting points for rivers using a seeded selection.
fn find_river_sources(
    seed: u64,
    elevation: &[u16],
    w: u16,
    h: u16,
    water_threshold: u16,
    count: u8,
) -> Vec<(u16, u16)> {
    // Collect candidate positions: tiles above 75th percentile of land elevation.
    let mut land_elevations: Vec<u16> = elevation
        .iter()
        .copied()
        .filter(|&e| e > water_threshold)
        .collect();

    if land_elevations.is_empty() || count == 0 {
        return Vec::new();
    }

    land_elevations.sort_unstable();
    let high_threshold = land_elevations[land_elevations.len() * 3 / 4];

    let candidates: Vec<(u16, u16)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| elevation[y as usize * w as usize + x as usize] >= high_threshold)
        .collect();

    if candidates.is_empty() {
        return Vec::new();
    }

    // Use the seed to deterministically pick sources from candidates.
    let mut sources = Vec::new();
    for i in 0..count {
        let h_val = hash(seed.wrapping_add(0xBEEF), i as i32, count as i32);
        let idx = (h_val as usize) % candidates.len();
        sources.push(candidates[idx]);
    }

    sources
}

// ---- Coastline Smoothing -----------------------------------------------------

/// Apply cellular automata smoothing to coastlines.
/// A tile becomes water if the majority of its 3x3 neighborhood is water,
/// and vice versa.
fn smooth_coastline(
    terrain: &mut [GenTerrain],
    w: u16,
    h: u16,
    passes: u8,
) {
    let count = w as usize * h as usize;

    for _ in 0..passes {
        let snapshot: Vec<GenTerrain> = terrain[..count].to_vec();

        for y in 0..h {
            for x in 0..w {
                let idx = y as usize * w as usize + x as usize;
                let mut water_count = 0u8;
                let mut total = 0u8;

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                            let ni = ny as usize * w as usize + nx as usize;
                            total += 1;
                            match snapshot[ni] {
                                GenTerrain::DeepWater | GenTerrain::ShallowWater => {
                                    water_count += 1;
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // If majority is water and current tile is land (non-river), make it shallow water.
                // If majority is land and current tile is water, make it sand.
                let is_water = matches!(
                    snapshot[idx],
                    GenTerrain::DeepWater | GenTerrain::ShallowWater
                );
                let is_river = snapshot[idx] == GenTerrain::River;

                if !is_river {
                    if !is_water && water_count > total / 2 + 1 {
                        terrain[idx] = GenTerrain::ShallowWater;
                    } else if is_water && water_count < total / 2 {
                        terrain[idx] = GenTerrain::Sand;
                    }
                }
            }
        }
    }
}

// ---- Default Map Generator ---------------------------------------------------

/// Default noise-based terrain generator.
pub struct DefaultMapGenerator;

impl IMapGenerator for DefaultMapGenerator {
    fn generate(&self, seed: u64, width: u16, height: u16, params: &MapGenParams) -> GeneratedMap {
        let count = width as usize * height as usize;

        if count == 0 {
            return GeneratedMap {
                width,
                height,
                tiles: Vec::new(),
            };
        }

        // Step 1: Generate elevation and moisture grids.
        let elevation = generate_elevation(seed, width, height, params.hilliness);
        let moisture = generate_moisture(seed, width, height);

        // Step 2: Compute water threshold from desired water ratio.
        let water_threshold = compute_water_threshold(&elevation, params.water_ratio);

        // Step 3: Assign initial terrain from elevation + moisture.
        let mut terrain: Vec<GenTerrain> = (0..count)
            .map(|i| assign_terrain(elevation[i], moisture[i], water_threshold))
            .collect();

        // Step 4: Generate rivers.
        if params.river_count > 0 {
            let sources = find_river_sources(
                seed,
                &elevation,
                width,
                height,
                water_threshold,
                params.river_count,
            );
            for (sx, sy) in sources {
                let river_path = trace_river(&elevation, width, height, sx, sy);
                for (rx, ry) in river_path {
                    let idx = ry as usize * width as usize + rx as usize;
                    // Only overwrite land tiles with river (don't overwrite water).
                    if !matches!(terrain[idx], GenTerrain::DeepWater | GenTerrain::ShallowWater) {
                        terrain[idx] = GenTerrain::River;
                    }
                }
            }
        }

        // Step 5: Smooth coastlines.
        if params.coast_smoothing > 0 {
            smooth_coastline(&mut terrain, width, height, params.coast_smoothing);
        }

        // Step 6: Assemble tiles.
        let tiles: Vec<GeneratedTile> = (0..count)
            .map(|i| GeneratedTile {
                terrain: terrain[i],
                elevation: elevation[i],
                moisture: moisture[i],
            })
            .collect();

        GeneratedMap {
            width,
            height,
            tiles,
        }
    }
}

// ---- TileMap Conversion ------------------------------------------------------

/// Build a [`TileMap`] from the output of a map generator.
///
/// Each tile's `terrain` field is populated via the existing
/// `From<GenTerrain> for TerrainType` conversion. All other `TileValue`
/// fields (kind, zone, flags) are left at their defaults.
pub fn tile_map_from_generated(map: &GeneratedMap) -> TileMap {
    let mut tilemap = TileMap::new(map.width as u32, map.height as u32);
    for (i, gen_tile) in map.tiles.iter().enumerate() {
        let terrain = TerrainType::from(gen_tile.terrain);
        // tile_index is row-major: i == y * width + x
        let x = (i % map.width as usize) as u32;
        let y = (i / map.width as usize) as u32;
        tilemap.set_terrain(x, y, terrain);
    }
    tilemap
}

// ---- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_default(seed: u64, w: u16, h: u16) -> GeneratedMap {
        let gen = DefaultMapGenerator;
        gen.generate(seed, w, h, &MapGenParams::default())
    }

    // 1. Default params are reasonable.
    #[test]
    fn default_params_reasonable() {
        let p = MapGenParams::default();
        assert!((p.water_ratio - 0.3).abs() < f32::EPSILON);
        assert!((p.hilliness - 0.5).abs() < f32::EPSILON);
        assert_eq!(p.river_count, 2);
        assert_eq!(p.coast_smoothing, 3);
    }

    // 2. Deterministic: same seed -> same map.
    #[test]
    fn deterministic_same_seed() {
        let map1 = gen_default(42, 32, 32);
        let map2 = gen_default(42, 32, 32);
        assert_eq!(map1.tiles.len(), map2.tiles.len());
        for (a, b) in map1.tiles.iter().zip(map2.tiles.iter()) {
            assert_eq!(a.terrain, b.terrain);
            assert_eq!(a.elevation, b.elevation);
            assert_eq!(a.moisture, b.moisture);
        }
    }

    // 3. Different seeds -> different maps.
    #[test]
    fn different_seeds_different_maps() {
        let map1 = gen_default(1, 32, 32);
        let map2 = gen_default(9999, 32, 32);
        // At least some tiles should differ.
        let diffs = map1
            .tiles
            .iter()
            .zip(map2.tiles.iter())
            .filter(|(a, b)| a.elevation != b.elevation)
            .count();
        assert!(diffs > 0, "Maps with different seeds should differ");
    }

    // 4. Map has correct dimensions.
    #[test]
    fn correct_dimensions() {
        let map = gen_default(1, 64, 48);
        assert_eq!(map.width, 64);
        assert_eq!(map.height, 48);
        assert_eq!(map.tiles.len(), 64 * 48);
    }

    // 5. All tiles have valid terrain.
    #[test]
    fn all_tiles_valid_terrain() {
        let map = gen_default(100, 32, 32);
        for tile in &map.tiles {
            // Ensure terrain is one of the defined variants.
            let t = tile.terrain as u8;
            assert!(t <= 7, "Terrain value {} out of range", t);
        }
    }

    // 6. Water ratio approximately matches parameter.
    #[test]
    fn water_ratio_approximately_matches() {
        let params = MapGenParams {
            water_ratio: 0.3,
            hilliness: 0.5,
            river_count: 0, // exclude rivers for cleaner measurement
            coast_smoothing: 0,
        };
        let gen = DefaultMapGenerator;
        let map = gen.generate(42, 64, 64, &params);
        let water_count = map
            .tiles
            .iter()
            .filter(|t| matches!(t.terrain, GenTerrain::DeepWater | GenTerrain::ShallowWater))
            .count();
        let actual_ratio = water_count as f32 / map.tiles.len() as f32;
        // Allow +/- 0.15 tolerance due to threshold quantization.
        assert!(
            (actual_ratio - 0.3).abs() < 0.15,
            "Water ratio {:.3} too far from target 0.3",
            actual_ratio
        );
    }

    // 7. Elevation range is valid (0-65535).
    #[test]
    fn elevation_range_valid() {
        let map = gen_default(7, 64, 64);
        for tile in &map.tiles {
            // u16 is inherently 0-65535, but verify generation doesn't panic.
            let _ = tile.elevation;
        }
        // Check that there is variation.
        let min_e = map.tiles.iter().map(|t| t.elevation).min().unwrap();
        let max_e = map.tiles.iter().map(|t| t.elevation).max().unwrap();
        assert!(max_e > min_e, "Elevation should have variation");
    }

    // 8. Mountain terrain at high elevation.
    #[test]
    fn mountain_at_high_elevation() {
        // Use high hilliness to get mountains.
        let params = MapGenParams {
            water_ratio: 0.1,
            hilliness: 0.9,
            river_count: 0,
            coast_smoothing: 0,
        };
        let gen = DefaultMapGenerator;
        let map = gen.generate(42, 64, 64, &params);
        let mountains: Vec<&GeneratedTile> = map
            .tiles
            .iter()
            .filter(|t| t.terrain == GenTerrain::Mountain)
            .collect();
        // With high hilliness, there should be some mountains.
        // Verify that mountain tiles have relatively high elevation.
        if !mountains.is_empty() {
            let avg_mountain_elev: f64 =
                mountains.iter().map(|t| t.elevation as f64).sum::<f64>() / mountains.len() as f64;
            let avg_all_elev: f64 =
                map.tiles.iter().map(|t| t.elevation as f64).sum::<f64>() / map.tiles.len() as f64;
            assert!(
                avg_mountain_elev > avg_all_elev,
                "Mountain tiles should have above-average elevation"
            );
        }
    }

    // 9. Water terrain at low elevation.
    #[test]
    fn water_at_low_elevation() {
        let params = MapGenParams {
            water_ratio: 0.3,
            hilliness: 0.5,
            river_count: 0,
            coast_smoothing: 0,
        };
        let gen = DefaultMapGenerator;
        let map = gen.generate(99, 64, 64, &params);
        let water_tiles: Vec<&GeneratedTile> = map
            .tiles
            .iter()
            .filter(|t| {
                matches!(
                    t.terrain,
                    GenTerrain::DeepWater | GenTerrain::ShallowWater
                )
            })
            .collect();
        if !water_tiles.is_empty() {
            let avg_water_elev: f64 =
                water_tiles.iter().map(|t| t.elevation as f64).sum::<f64>()
                    / water_tiles.len() as f64;
            let avg_all_elev: f64 =
                map.tiles.iter().map(|t| t.elevation as f64).sum::<f64>() / map.tiles.len() as f64;
            assert!(
                avg_water_elev < avg_all_elev,
                "Water tiles should have below-average elevation"
            );
        }
    }

    // 10. River tiles exist when river_count > 0.
    #[test]
    fn rivers_exist_when_requested() {
        let params = MapGenParams {
            water_ratio: 0.2,
            hilliness: 0.6,
            river_count: 3,
            coast_smoothing: 0,
        };
        let gen = DefaultMapGenerator;
        let map = gen.generate(42, 64, 64, &params);
        let river_count = map
            .tiles
            .iter()
            .filter(|t| t.terrain == GenTerrain::River)
            .count();
        assert!(river_count > 0, "Should have river tiles when river_count > 0");
    }

    // 11. Empty map (0 rivers, 0 water) still generates valid terrain.
    #[test]
    fn zero_water_zero_rivers_valid() {
        let params = MapGenParams {
            water_ratio: 0.0,
            hilliness: 0.5,
            river_count: 0,
            coast_smoothing: 0,
        };
        let gen = DefaultMapGenerator;
        let map = gen.generate(42, 32, 32, &params);
        assert_eq!(map.tiles.len(), 32 * 32);
        // With water_ratio=0, there should be no water tiles.
        let water_count = map
            .tiles
            .iter()
            .filter(|t| matches!(t.terrain, GenTerrain::DeepWater | GenTerrain::ShallowWater))
            .count();
        assert_eq!(water_count, 0, "No water when water_ratio=0");
    }

    // 12. Coastline smoothing produces valid terrain.
    #[test]
    fn coastline_smoothing_valid() {
        let params = MapGenParams {
            water_ratio: 0.3,
            hilliness: 0.5,
            river_count: 0,
            coast_smoothing: 5,
        };
        let gen = DefaultMapGenerator;
        let map = gen.generate(42, 32, 32, &params);
        for tile in &map.tiles {
            let t = tile.terrain as u8;
            assert!(t <= 7, "Smoothed terrain value {} out of range", t);
        }
    }

    // 13. Moisture values in valid range.
    #[test]
    fn moisture_values_valid() {
        let map = gen_default(42, 32, 32);
        // u8 is inherently 0-255, verify variation exists.
        let min_m = map.tiles.iter().map(|t| t.moisture).min().unwrap();
        let max_m = map.tiles.iter().map(|t| t.moisture).max().unwrap();
        assert!(max_m > min_m, "Moisture should have variation");
    }

    // 14. Small map (8x8) generates successfully.
    #[test]
    fn small_map_8x8() {
        let map = gen_default(1, 8, 8);
        assert_eq!(map.width, 8);
        assert_eq!(map.height, 8);
        assert_eq!(map.tiles.len(), 64);
    }

    // 15. Large map (128x128) generates successfully.
    #[test]
    fn large_map_128x128() {
        let map = gen_default(1, 128, 128);
        assert_eq!(map.width, 128);
        assert_eq!(map.height, 128);
        assert_eq!(map.tiles.len(), 128 * 128);
    }

    // 16. noise2d returns values in [-1, 1].
    #[test]
    fn noise2d_range() {
        for i in 0..1000 {
            let x = (i as f32) * 0.1;
            let y = (i as f32) * 0.07;
            let v = noise2d(42, x, y);
            assert!(
                v >= -1.0 && v <= 1.0,
                "noise2d({}, {}) = {} out of range",
                x,
                y,
                v
            );
        }
    }

    // 17. octave_noise returns values in [-1, 1].
    #[test]
    fn octave_noise_range() {
        for i in 0..500 {
            let x = (i as f32) * 0.15;
            let y = (i as f32) * 0.12;
            let v = octave_noise(42, x, y, 5);
            assert!(
                v >= -1.0 && v <= 1.0,
                "octave_noise({}, {}) = {} out of range",
                x,
                y,
                v
            );
        }
    }

    // 18. GeneratedMap::get returns correct tile.
    #[test]
    fn generated_map_get() {
        let map = gen_default(42, 16, 16);
        let tile = map.get(5, 3).unwrap();
        let idx = 3 * 16 + 5;
        assert_eq!(tile.elevation, map.tiles[idx].elevation);
        assert!(map.get(16, 0).is_none());
        assert!(map.get(0, 16).is_none());
    }

    // 19. Trace river follows downhill path.
    #[test]
    fn trace_river_downhill() {
        // Create a simple elevation gradient: higher on left, lower on right.
        let w: u16 = 8;
        let h: u16 = 8;
        let mut elevation = vec![0u16; 64];
        for y in 0..h {
            for x in 0..w {
                elevation[y as usize * w as usize + x as usize] = (w - x) as u16 * 1000;
            }
        }
        let path = trace_river(&elevation, w, h, 0, 4);
        assert!(!path.is_empty());
        // Path should move to the right (toward lower elevation).
        if path.len() > 1 {
            assert!(path.last().unwrap().0 > path[0].0, "River should flow downhill");
        }
    }

    // 20. Zero-size map edge case.
    #[test]
    fn zero_size_map() {
        let gen = DefaultMapGenerator;
        let map = gen.generate(42, 0, 0, &MapGenParams::default());
        assert_eq!(map.tiles.len(), 0);
    }

    // 21. From<GenTerrain> for TerrainType — all variants.
    #[test]
    fn gen_terrain_to_terrain_type_water_variants() {
        use crate::core_types::TerrainType;
        assert_eq!(TerrainType::from(GenTerrain::DeepWater),    TerrainType::Water);
        assert_eq!(TerrainType::from(GenTerrain::ShallowWater), TerrainType::Water);
        assert_eq!(TerrainType::from(GenTerrain::River),        TerrainType::Water);
    }

    #[test]
    fn gen_terrain_to_terrain_type_land_variants() {
        use crate::core_types::TerrainType;
        assert_eq!(TerrainType::from(GenTerrain::Sand),     TerrainType::Sand);
        assert_eq!(TerrainType::from(GenTerrain::Grass),    TerrainType::Grass);
        assert_eq!(TerrainType::from(GenTerrain::Forest),   TerrainType::Forest);
        assert_eq!(TerrainType::from(GenTerrain::Hill),     TerrainType::Rock);
        assert_eq!(TerrainType::from(GenTerrain::Mountain), TerrainType::Rock);
    }

    // 22. tile_map_from_generated: dimensions match GeneratedMap.
    #[test]
    fn tile_map_from_generated_dimensions() {
        use crate::core::mapgen::tile_map_from_generated;
        let map = gen_default(1, 32, 24);
        let tilemap = tile_map_from_generated(&map);
        assert_eq!(tilemap.width(), 32);
        assert_eq!(tilemap.height(), 24);
        assert_eq!(tilemap.len(), 32 * 24);
    }

    // 23. tile_map_from_generated: water GenTerrain becomes TerrainType::Water.
    #[test]
    fn tile_map_from_generated_water_terrain() {
        use crate::core::mapgen::tile_map_from_generated;
        use crate::core_types::TerrainType;
        // Use a high water_ratio to guarantee water tiles.
        let gen = DefaultMapGenerator;
        let params = MapGenParams {
            water_ratio: 0.8,
            hilliness: 0.5,
            river_count: 0,
            coast_smoothing: 0,
        };
        let map = gen.generate(1, 32, 32, &params);
        let tilemap = tile_map_from_generated(&map);

        // Every GeneratedTile that maps to Water should match.
        for (i, gen_tile) in map.tiles.iter().enumerate() {
            let x = (i % map.width as usize) as u32;
            let y = (i / map.width as usize) as u32;
            let expected = TerrainType::from(gen_tile.terrain);
            let actual = tilemap.get(x, y).unwrap().terrain;
            assert_eq!(actual, expected, "mismatch at ({}, {})", x, y);
        }
    }

    // 24. tile_map_from_generated: non-water tiles preserve correct terrain.
    #[test]
    fn tile_map_from_generated_land_terrain() {
        use crate::core::mapgen::tile_map_from_generated;
        use crate::core_types::TerrainType;
        let params = MapGenParams {
            water_ratio: 0.0,
            hilliness: 0.0,
            river_count: 0,
            coast_smoothing: 0,
        };
        let gen = DefaultMapGenerator;
        let map = gen.generate(7, 16, 16, &params);
        let tilemap = tile_map_from_generated(&map);
        // With water_ratio=0 all tiles should be land; none should be Water.
        for (_, _, tile) in tilemap.iter() {
            assert_ne!(tile.terrain, TerrainType::Water);
        }
    }

    // 25. tile_map_from_generated: kind and zone remain at defaults.
    #[test]
    fn tile_map_from_generated_overlay_defaults() {
        use crate::core::mapgen::tile_map_from_generated;
        use crate::core::tilemap::{TileKind, TileFlags};
        use crate::core_types::ZoneType;
        let map = gen_default(99, 16, 16);
        let tilemap = tile_map_from_generated(&map);
        for (_, _, tile) in tilemap.iter() {
            assert_eq!(tile.kind, TileKind::Empty);
            assert_eq!(tile.zone, ZoneType::None);
            assert!(tile.flags.is_empty());
        }
    }
}
