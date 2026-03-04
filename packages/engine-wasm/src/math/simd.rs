//! SIMD-accelerated kernels with scalar fallback.
//!
//! Feature-gated: `simd` feature enables SIMD paths, otherwise scalar fallback.
//! All operations work on u16 arrays (heatmap values) and use integer arithmetic
//! only -- no floating point.

/// Trait for batch operations on u16 arrays (heatmap values).
///
/// Implementors are zero-size structs; `&self` is carried only so that
/// `get_kernel()` can return `&'static dyn ISIMDKernel`.
pub trait ISIMDKernel {
    /// Human-readable name of this kernel (e.g. "scalar", "wasm-simd").
    fn name(&self) -> &'static str;

    /// Batch add a constant to each element, clamped to `u16::MAX`.
    fn batch_add_u16(&self, data: &mut [u16], value: u16);

    /// Batch multiply each element by a fixed-point ratio (0-65535 maps to 0.0-1.0).
    ///
    /// Formula per element: `(v as u32 * ratio as u32) >> 16`.
    fn batch_scale_u16(&self, data: &mut [u16], ratio: u16);

    /// Batch diffusion step: for each cell, average with 4 neighbours (NESW).
    ///
    /// Border cells use their own value for missing neighbours.
    /// Reads from `src`, writes to `dst`. Both slices must be `width * height` long.
    fn diffuse_u16(&self, src: &[u16], dst: &mut [u16], width: usize, height: usize);

    /// Batch accumulate: `dst[i] = dst[i].saturating_add(src[i])`.
    ///
    /// Processes `min(dst.len(), src.len())` elements.
    fn accumulate_u16(&self, dst: &mut [u16], src: &[u16]);

    /// Find the maximum value in the array. Returns 0 for empty arrays.
    fn max_u16(&self, data: &[u16]) -> u16;

    /// Count elements whose value is strictly above `threshold`.
    fn count_above_u16(&self, data: &[u16], threshold: u16) -> usize;
}

// ---------------------------------------------------------------------------
// Scalar fallback (always available, no SIMD required)
// ---------------------------------------------------------------------------

/// Scalar (non-SIMD) kernel. Always available on every target.
pub struct ScalarKernel;

impl ISIMDKernel for ScalarKernel {
    fn name(&self) -> &'static str {
        "scalar"
    }

    fn batch_add_u16(&self, data: &mut [u16], value: u16) {
        for v in data.iter_mut() {
            *v = v.saturating_add(value);
        }
    }

    fn batch_scale_u16(&self, data: &mut [u16], ratio: u16) {
        for v in data.iter_mut() {
            *v = ((*v as u32 * ratio as u32) >> 16) as u16;
        }
    }

    fn diffuse_u16(&self, src: &[u16], dst: &mut [u16], width: usize, height: usize) {
        debug_assert_eq!(src.len(), width * height);
        debug_assert_eq!(dst.len(), width * height);

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let center = src[idx] as u32;
                let north = if y > 0 { src[(y - 1) * width + x] as u32 } else { center };
                let south = if y + 1 < height { src[(y + 1) * width + x] as u32 } else { center };
                let east = if x + 1 < width { src[y * width + x + 1] as u32 } else { center };
                let west = if x > 0 { src[y * width + x - 1] as u32 } else { center };
                dst[idx] = ((center + north + south + east + west) / 5) as u16;
            }
        }
    }

    fn accumulate_u16(&self, dst: &mut [u16], src: &[u16]) {
        let len = dst.len().min(src.len());
        for i in 0..len {
            dst[i] = dst[i].saturating_add(src[i]);
        }
    }

    fn max_u16(&self, data: &[u16]) -> u16 {
        data.iter().copied().max().unwrap_or(0)
    }

    fn count_above_u16(&self, data: &[u16], threshold: u16) -> usize {
        data.iter().filter(|&&v| v > threshold).count()
    }
}

// ---------------------------------------------------------------------------
// Kernel selection
// ---------------------------------------------------------------------------

/// Get the active kernel.
///
/// Returns the scalar kernel today. When the `simd` feature is enabled on a
/// supported target this will return a SIMD-accelerated implementation instead.
pub fn get_kernel() -> &'static dyn ISIMDKernel {
    &ScalarKernel
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- batch_add_u16 ------------------------------------------------------

    #[test]
    fn batch_add_u16_adds_correctly() {
        let kernel = ScalarKernel;
        let mut data = vec![10, 20, 30, 40, 50];
        kernel.batch_add_u16(&mut data, 5);
        assert_eq!(data, vec![15, 25, 35, 45, 55]);
    }

    #[test]
    fn batch_add_u16_saturates_at_max() {
        let kernel = ScalarKernel;
        let mut data = vec![u16::MAX - 5, u16::MAX, 100];
        kernel.batch_add_u16(&mut data, 10);
        assert_eq!(data[0], u16::MAX);
        assert_eq!(data[1], u16::MAX);
        assert_eq!(data[2], 110);
    }

    #[test]
    fn batch_add_u16_zero_is_noop() {
        let kernel = ScalarKernel;
        let mut data = vec![1, 2, 3];
        kernel.batch_add_u16(&mut data, 0);
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn batch_add_u16_empty_slice() {
        let kernel = ScalarKernel;
        let mut data: Vec<u16> = vec![];
        kernel.batch_add_u16(&mut data, 100);
        assert!(data.is_empty());
    }

    // -- batch_scale_u16 ----------------------------------------------------

    #[test]
    fn batch_scale_u16_halves_values() {
        let kernel = ScalarKernel;
        // ratio = 32768 means 32768/65536 = 0.5
        let mut data = vec![100, 200, 1000, 65534];
        kernel.batch_scale_u16(&mut data, 32768);
        assert_eq!(data[0], 50);
        assert_eq!(data[1], 100);
        assert_eq!(data[2], 500);
        assert_eq!(data[3], 32767);
    }

    #[test]
    fn batch_scale_u16_zeroes_with_zero_ratio() {
        let kernel = ScalarKernel;
        let mut data = vec![100, 200, u16::MAX];
        kernel.batch_scale_u16(&mut data, 0);
        assert_eq!(data, vec![0, 0, 0]);
    }

    #[test]
    fn batch_scale_u16_near_one_preserves() {
        let kernel = ScalarKernel;
        // ratio = 65535 is as close to 1.0 as we can get
        let mut data = vec![100, 200, 1000];
        kernel.batch_scale_u16(&mut data, 65535);
        // 100 * 65535 / 65536 = 99 (due to integer truncation)
        assert_eq!(data[0], 99);
        assert_eq!(data[1], 199);
        assert_eq!(data[2], 999);
    }

    #[test]
    fn batch_scale_u16_quarter() {
        let kernel = ScalarKernel;
        // ratio = 16384 means 16384/65536 = 0.25
        let mut data = vec![400, 1000];
        kernel.batch_scale_u16(&mut data, 16384);
        assert_eq!(data[0], 100);
        assert_eq!(data[1], 250);
    }

    // -- diffuse_u16 --------------------------------------------------------

    #[test]
    fn diffuse_u16_averages_on_3x3_grid() {
        let kernel = ScalarKernel;
        // 3x3 grid with centre = 100, all others = 0
        let src = vec![
            0, 0, 0,
            0, 100, 0,
            0, 0, 0,
        ];
        let mut dst = vec![0u16; 9];
        kernel.diffuse_u16(&src, &mut dst, 3, 3);

        // Centre (1,1): (100 + 0 + 0 + 0 + 0) / 5 = 20
        assert_eq!(dst[4], 20);

        // North of centre (1,0): centre_val=0, north=0(self), south=100, east=0, west=0
        // (0 + 0 + 100 + 0 + 0) / 5 = 20
        assert_eq!(dst[1], 20);

        // Corner (0,0): centre=0, north=0(self), south=0, east=0, west=0(self)
        // (0+0+0+0+0)/5 = 0
        assert_eq!(dst[0], 0);
    }

    #[test]
    fn diffuse_u16_handles_1x1_grid() {
        let kernel = ScalarKernel;
        let src = vec![500u16];
        let mut dst = vec![0u16; 1];
        kernel.diffuse_u16(&src, &mut dst, 1, 1);
        // 1x1: all neighbours are self -> (500*5)/5 = 500
        assert_eq!(dst[0], 500);
    }

    #[test]
    fn diffuse_u16_uniform_grid_unchanged() {
        let kernel = ScalarKernel;
        let val = 42u16;
        let src = vec![val; 16]; // 4x4
        let mut dst = vec![0u16; 16];
        kernel.diffuse_u16(&src, &mut dst, 4, 4);
        // Uniform grid: average of identical values is the same value
        for &v in &dst {
            assert_eq!(v, val);
        }
    }

    #[test]
    fn diffuse_u16_single_row() {
        let kernel = ScalarKernel;
        // 5x1 grid
        let src = vec![0, 0, 100, 0, 0];
        let mut dst = vec![0u16; 5];
        kernel.diffuse_u16(&src, &mut dst, 5, 1);
        // For cell (2,0): north=self=100, south=self=100, east=0, west=0, center=100
        // (100 + 100 + 100 + 0 + 0) / 5 = 60
        assert_eq!(dst[2], 60);
    }

    // -- accumulate_u16 -----------------------------------------------------

    #[test]
    fn accumulate_u16_adds_elementwise() {
        let kernel = ScalarKernel;
        let mut dst = vec![10, 20, 30];
        let src = vec![5, 10, 15];
        kernel.accumulate_u16(&mut dst, &src);
        assert_eq!(dst, vec![15, 30, 45]);
    }

    #[test]
    fn accumulate_u16_saturates() {
        let kernel = ScalarKernel;
        let mut dst = vec![u16::MAX - 10, 100];
        let src = vec![20, 50];
        kernel.accumulate_u16(&mut dst, &src);
        assert_eq!(dst[0], u16::MAX);
        assert_eq!(dst[1], 150);
    }

    #[test]
    fn accumulate_u16_mismatched_lengths() {
        let kernel = ScalarKernel;
        let mut dst = vec![10, 20, 30];
        let src = vec![5, 10];
        kernel.accumulate_u16(&mut dst, &src);
        // Only first 2 elements are affected
        assert_eq!(dst, vec![15, 30, 30]);
    }

    // -- max_u16 ------------------------------------------------------------

    #[test]
    fn max_u16_finds_max() {
        let kernel = ScalarKernel;
        let data = vec![10, 50, 30, 100, 20];
        assert_eq!(kernel.max_u16(&data), 100);
    }

    #[test]
    fn max_u16_empty_returns_zero() {
        let kernel = ScalarKernel;
        let data: Vec<u16> = vec![];
        assert_eq!(kernel.max_u16(&data), 0);
    }

    #[test]
    fn max_u16_single_element() {
        let kernel = ScalarKernel;
        let data = vec![42];
        assert_eq!(kernel.max_u16(&data), 42);
    }

    #[test]
    fn max_u16_all_same() {
        let kernel = ScalarKernel;
        let data = vec![7; 100];
        assert_eq!(kernel.max_u16(&data), 7);
    }

    // -- count_above_u16 ----------------------------------------------------

    #[test]
    fn count_above_u16_correct() {
        let kernel = ScalarKernel;
        let data = vec![10, 20, 30, 40, 50];
        assert_eq!(kernel.count_above_u16(&data, 25), 3); // 30, 40, 50
    }

    #[test]
    fn count_above_u16_none_above() {
        let kernel = ScalarKernel;
        let data = vec![1, 2, 3];
        assert_eq!(kernel.count_above_u16(&data, 100), 0);
    }

    #[test]
    fn count_above_u16_all_above() {
        let kernel = ScalarKernel;
        let data = vec![10, 20, 30];
        assert_eq!(kernel.count_above_u16(&data, 0), 3);
    }

    #[test]
    fn count_above_u16_threshold_equals_value_not_counted() {
        let kernel = ScalarKernel;
        // "strictly above" -- equal values are NOT counted
        let data = vec![10, 20, 30];
        assert_eq!(kernel.count_above_u16(&data, 20), 1); // only 30
    }

    #[test]
    fn count_above_u16_empty() {
        let kernel = ScalarKernel;
        let data: Vec<u16> = vec![];
        assert_eq!(kernel.count_above_u16(&data, 0), 0);
    }

    // -- get_kernel ---------------------------------------------------------

    #[test]
    fn get_kernel_returns_scalar_kernel() {
        let kernel = get_kernel();
        assert_eq!(kernel.name(), "scalar");
    }

    // -- SIMD vs scalar equivalence -----------------------------------------

    #[test]
    fn scalar_and_get_kernel_produce_identical_batch_add() {
        let scalar = ScalarKernel;
        let dyn_kernel = get_kernel();

        let mut data_scalar = vec![100, 200, 300, u16::MAX - 5, 0];
        let mut data_dyn = data_scalar.clone();

        scalar.batch_add_u16(&mut data_scalar, 50);
        dyn_kernel.batch_add_u16(&mut data_dyn, 50);

        assert_eq!(data_scalar, data_dyn);
    }

    #[test]
    fn scalar_and_get_kernel_produce_identical_batch_scale() {
        let scalar = ScalarKernel;
        let dyn_kernel = get_kernel();

        let mut data_scalar = vec![100, 500, 1000, u16::MAX, 0];
        let mut data_dyn = data_scalar.clone();

        scalar.batch_scale_u16(&mut data_scalar, 32768);
        dyn_kernel.batch_scale_u16(&mut data_dyn, 32768);

        assert_eq!(data_scalar, data_dyn);
    }

    #[test]
    fn scalar_and_get_kernel_produce_identical_diffuse() {
        let scalar = ScalarKernel;
        let dyn_kernel = get_kernel();

        let src = vec![
            10, 20, 30,
            40, 50, 60,
            70, 80, 90,
        ];
        let mut dst_scalar = vec![0u16; 9];
        let mut dst_dyn = vec![0u16; 9];

        scalar.diffuse_u16(&src, &mut dst_scalar, 3, 3);
        dyn_kernel.diffuse_u16(&src, &mut dst_dyn, 3, 3);

        assert_eq!(dst_scalar, dst_dyn);
    }

    #[test]
    fn scalar_and_get_kernel_produce_identical_accumulate() {
        let scalar = ScalarKernel;
        let dyn_kernel = get_kernel();

        let mut dst_scalar = vec![100, 200, u16::MAX - 10];
        let mut dst_dyn = dst_scalar.clone();
        let src = vec![50, 100, 20];

        scalar.accumulate_u16(&mut dst_scalar, &src);
        dyn_kernel.accumulate_u16(&mut dst_dyn, &src);

        assert_eq!(dst_scalar, dst_dyn);
    }

    #[test]
    fn scalar_and_get_kernel_produce_identical_max() {
        let scalar = ScalarKernel;
        let dyn_kernel = get_kernel();

        let data = vec![5, 99, 42, 1000, 3];
        assert_eq!(scalar.max_u16(&data), dyn_kernel.max_u16(&data));
    }

    #[test]
    fn scalar_and_get_kernel_produce_identical_count_above() {
        let scalar = ScalarKernel;
        let dyn_kernel = get_kernel();

        let data = vec![10, 20, 30, 40, 50, 60];
        assert_eq!(
            scalar.count_above_u16(&data, 35),
            dyn_kernel.count_above_u16(&data, 35)
        );
    }
}
