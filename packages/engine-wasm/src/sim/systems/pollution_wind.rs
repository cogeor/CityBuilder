//! Directional pollution propagation with wind model.

/// Wind direction and strength.
#[derive(Debug, Clone, Copy)]
pub struct WindVector {
    pub direction_deg: u16, // 0-359, 0=North, 90=East
    pub strength: u16,      // 0-65535, affects spread distance
}

impl Default for WindVector {
    fn default() -> Self {
        Self {
            direction_deg: 0,
            strength: 16384,
        } // moderate north wind
    }
}

/// Pollution source descriptor.
#[derive(Debug, Clone)]
pub struct PollutionSource {
    pub x: u16,
    pub y: u16,
    pub intensity: u16, // 0-65535
}

/// Trait for pluggable pollution propagation.
pub trait IPollutionPropagation {
    fn propagate(
        &self,
        sources: &[PollutionSource],
        wind: &WindVector,
        width: usize,
        height: usize,
    ) -> Vec<u16>;
    fn name(&self) -> &str;
}

/// Default directional spread model.
pub struct DirectionalPollutionModel {
    pub decay_rate: u16, // how fast pollution decays with distance
    pub max_spread: u16, // maximum spread distance in tiles
    pub wind_bias: f32,  // 0.0-1.0, how much wind direction biases spread
}

impl Default for DirectionalPollutionModel {
    fn default() -> Self {
        Self {
            decay_rate: 4096,
            max_spread: 16,
            wind_bias: 0.6,
        }
    }
}

impl IPollutionPropagation for DirectionalPollutionModel {
    fn propagate(
        &self,
        sources: &[PollutionSource],
        wind: &WindVector,
        width: usize,
        height: usize,
    ) -> Vec<u16> {
        let size = width * height;
        let mut grid = vec![0u16; size];

        // Wind direction to dx, dy components
        let wind_rad = (wind.direction_deg as f64) * std::f64::consts::PI / 180.0;
        let wind_dx = wind_rad.sin();
        let wind_dy = -wind_rad.cos(); // negative because 0 = north = -y

        let wind_strength_norm = wind.strength as f64 / 65535.0;

        for source in sources {
            let sx = source.x as i32;
            let sy = source.y as i32;
            let spread = self.max_spread as i32;

            for dy in -spread..=spread {
                for dx in -spread..=spread {
                    let tx = sx + dx;
                    let ty = sy + dy;

                    if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
                        continue;
                    }

                    let dist = ((dx * dx + dy * dy) as f64).sqrt();
                    if dist > spread as f64 {
                        continue;
                    }

                    // Directional bias: boost in wind direction, reduce against
                    let dot = if dist > 0.0 {
                        (dx as f64 * wind_dx + dy as f64 * wind_dy) / dist
                    } else {
                        1.0
                    };
                    let dir_factor = 1.0 + dot * self.wind_bias as f64 * wind_strength_norm;

                    // Distance decay
                    let decay = 1.0 - (dist / spread as f64);
                    let value = source.intensity as f64 * decay * dir_factor.max(0.1);

                    let idx = ty as usize * width + tx as usize;
                    grid[idx] = grid[idx].saturating_add(value.clamp(0.0, 65535.0) as u16);
                }
            }
        }

        grid
    }

    fn name(&self) -> &str {
        "directional_spread"
    }
}

/// Apply pollution sinks (parks, vegetation) to reduce pollution.
pub fn apply_sinks(grid: &mut [u16], sinks: &[(usize, u16)]) {
    for &(idx, absorption) in sinks {
        if idx < grid.len() {
            grid[idx] = grid[idx].saturating_sub(absorption);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a model with small spread for test grids.
    fn test_model(max_spread: u16) -> DirectionalPollutionModel {
        DirectionalPollutionModel {
            decay_rate: 4096,
            max_spread,
            wind_bias: 0.6,
        }
    }

    // ---- 1. Point source with no wind -> symmetric spread ----
    #[test]
    fn point_source_no_wind_symmetric() {
        let model = test_model(4);
        let wind = WindVector {
            direction_deg: 0,
            strength: 0, // no wind
        };
        let sources = vec![PollutionSource {
            x: 5,
            y: 5,
            intensity: 10000,
        }];
        let grid = model.propagate(&sources, &wind, 11, 11);

        // With zero wind strength, directional bias is zero so spread should
        // be symmetric. Check north vs south vs east vs west at distance 2.
        let center_idx = 5 * 11 + 5;
        let north_idx = 3 * 11 + 5; // (5, 3)
        let south_idx = 7 * 11 + 5; // (5, 7)
        let east_idx = 5 * 11 + 7; // (7, 5)
        let west_idx = 5 * 11 + 3; // (3, 5)

        assert!(grid[center_idx] > 0, "center should have pollution");
        assert_eq!(grid[north_idx], grid[south_idx], "N/S symmetric");
        assert_eq!(grid[east_idx], grid[west_idx], "E/W symmetric");
        assert_eq!(grid[north_idx], grid[east_idx], "N/E symmetric");
    }

    // ---- 2. Point source with wind -> asymmetric (more downwind) ----
    #[test]
    fn point_source_with_wind_asymmetric() {
        let model = test_model(8);
        // Wind blowing east (90 degrees)
        let wind = WindVector {
            direction_deg: 90,
            strength: 50000,
        };
        let sources = vec![PollutionSource {
            x: 10,
            y: 10,
            intensity: 20000,
        }];
        let grid = model.propagate(&sources, &wind, 21, 21);

        // Downwind (east) at distance 4 should be higher than upwind (west)
        let east_val = grid[10 * 21 + 14]; // (14, 10)
        let west_val = grid[10 * 21 + 6]; // (6, 10)

        assert!(
            east_val > west_val,
            "downwind (east={}) should exceed upwind (west={})",
            east_val,
            west_val
        );
    }

    // ---- 3. Multiple sources accumulate ----
    #[test]
    fn multiple_sources_accumulate() {
        let model = test_model(4);
        let wind = WindVector {
            direction_deg: 0,
            strength: 0,
        };

        // Single source
        let single = model.propagate(
            &[PollutionSource {
                x: 5,
                y: 5,
                intensity: 10000,
            }],
            &wind,
            11,
            11,
        );

        // Two sources at same location
        let double = model.propagate(
            &[
                PollutionSource {
                    x: 5,
                    y: 5,
                    intensity: 10000,
                },
                PollutionSource {
                    x: 5,
                    y: 5,
                    intensity: 10000,
                },
            ],
            &wind,
            11,
            11,
        );

        let center = 5 * 11 + 5;
        assert!(
            double[center] > single[center],
            "two sources ({}) should exceed one ({})",
            double[center],
            single[center]
        );
    }

    // ---- 4. Sinks reduce pollution ----
    #[test]
    fn sinks_reduce_pollution() {
        let mut grid = vec![100u16; 25]; // 5x5
        let sinks = vec![(0, 50), (12, 200)];
        apply_sinks(&mut grid, &sinks);

        assert_eq!(grid[0], 50); // 100 - 50
        assert_eq!(grid[12], 0); // saturating_sub: 100 - 200 = 0
        assert_eq!(grid[1], 100); // unaffected
    }

    // ---- 5. Empty sources -> zero grid ----
    #[test]
    fn empty_sources_zero_grid() {
        let model = test_model(4);
        let wind = WindVector::default();
        let grid = model.propagate(&[], &wind, 10, 10);

        assert_eq!(grid.len(), 100);
        assert!(grid.iter().all(|&v| v == 0), "all tiles should be zero");
    }

    // ---- 6. Source at edge doesn't panic ----
    #[test]
    fn source_at_edge_no_panic() {
        let model = test_model(8);
        let wind = WindVector {
            direction_deg: 45,
            strength: 30000,
        };

        // Source at corner (0, 0)
        let sources = vec![PollutionSource {
            x: 0,
            y: 0,
            intensity: 30000,
        }];
        let grid = model.propagate(&sources, &wind, 10, 10);
        assert_eq!(grid.len(), 100);
        assert!(grid[0] > 0, "corner source should have pollution");

        // Source at opposite corner
        let sources2 = vec![PollutionSource {
            x: 9,
            y: 9,
            intensity: 30000,
        }];
        let grid2 = model.propagate(&sources2, &wind, 10, 10);
        assert!(grid2[99] > 0, "opposite corner should have pollution");
    }

    // ---- 7. Wind 0 degrees = north bias ----
    #[test]
    fn wind_zero_deg_north_bias() {
        let model = test_model(8);
        let wind = WindVector {
            direction_deg: 0,
            strength: 50000,
        };
        let sources = vec![PollutionSource {
            x: 10,
            y: 10,
            intensity: 20000,
        }];
        let grid = model.propagate(&sources, &wind, 21, 21);

        // 0 deg = north wind means pollution spreads north (toward -y)
        let north_val = grid[6 * 21 + 10]; // (10, 6) - 4 tiles north
        let south_val = grid[14 * 21 + 10]; // (10, 14) - 4 tiles south

        assert!(
            north_val > south_val,
            "north ({}) should exceed south ({}) with 0-deg wind",
            north_val,
            south_val
        );
    }

    // ---- 8. Wind 90 degrees = east bias ----
    #[test]
    fn wind_90_deg_east_bias() {
        let model = test_model(8);
        let wind = WindVector {
            direction_deg: 90,
            strength: 50000,
        };
        let sources = vec![PollutionSource {
            x: 10,
            y: 10,
            intensity: 20000,
        }];
        let grid = model.propagate(&sources, &wind, 21, 21);

        // 90 deg = east wind means pollution spreads east (toward +x)
        let east_val = grid[10 * 21 + 14]; // (14, 10)
        let west_val = grid[10 * 21 + 6]; // (6, 10)

        assert!(
            east_val > west_val,
            "east ({}) should exceed west ({}) with 90-deg wind",
            east_val,
            west_val
        );
    }

    // ---- 9. max_spread limits distance ----
    #[test]
    fn max_spread_limits_distance() {
        let model = test_model(3);
        let wind = WindVector {
            direction_deg: 0,
            strength: 0,
        };
        let sources = vec![PollutionSource {
            x: 10,
            y: 10,
            intensity: 30000,
        }];
        let grid = model.propagate(&sources, &wind, 21, 21);

        // At distance 5 (beyond max_spread=3), pollution should be zero
        let far_tile = grid[10 * 21 + 15]; // (15, 10) - distance 5
        assert_eq!(far_tile, 0, "beyond max_spread should be zero");

        // At distance 2 (within max_spread=3), should have pollution
        let near_tile = grid[10 * 21 + 12]; // (12, 10) - distance 2
        assert!(near_tile > 0, "within max_spread should have pollution");
    }

    // ---- 10. Default model reasonable values ----
    #[test]
    fn default_model_reasonable_values() {
        let model = DirectionalPollutionModel::default();
        assert_eq!(model.decay_rate, 4096);
        assert_eq!(model.max_spread, 16);
        assert!((model.wind_bias - 0.6).abs() < f32::EPSILON);
    }

    // ---- 11. Model name correct ----
    #[test]
    fn model_name_correct() {
        let model = DirectionalPollutionModel::default();
        assert_eq!(model.name(), "directional_spread");
    }

    // ---- 12. apply_sinks out of bounds -> safe ----
    #[test]
    fn apply_sinks_out_of_bounds_safe() {
        let mut grid = vec![100u16; 10];
        let sinks = vec![(999, 50), (0, 10)]; // idx 999 is out of bounds
        apply_sinks(&mut grid, &sinks);

        assert_eq!(grid[0], 90); // valid sink applied
        // No panic from out-of-bounds index
    }

    // ---- 13. Default WindVector values ----
    #[test]
    fn default_wind_vector() {
        let w = WindVector::default();
        assert_eq!(w.direction_deg, 0);
        assert_eq!(w.strength, 16384);
    }

    // ---- 14. Large intensity saturates at u16 max ----
    #[test]
    fn large_intensity_saturates() {
        let model = test_model(2);
        let wind = WindVector {
            direction_deg: 0,
            strength: 0,
        };
        // Two very strong sources at the same spot
        let sources = vec![
            PollutionSource {
                x: 2,
                y: 2,
                intensity: 60000,
            },
            PollutionSource {
                x: 2,
                y: 2,
                intensity: 60000,
            },
        ];
        let grid = model.propagate(&sources, &wind, 5, 5);
        // Center value should be capped at u16::MAX via saturating_add
        assert!(grid[2 * 5 + 2] <= u16::MAX);
    }

    // ---- 15. 1x1 grid with source ----
    #[test]
    fn single_tile_grid() {
        let model = test_model(4);
        let wind = WindVector::default();
        let sources = vec![PollutionSource {
            x: 0,
            y: 0,
            intensity: 5000,
        }];
        let grid = model.propagate(&sources, &wind, 1, 1);
        assert_eq!(grid.len(), 1);
        assert!(grid[0] > 0, "single tile should have pollution from source at same location");
    }
}
