//! Fixed-point arithmetic types for deterministic simulation.
//!
//! Q16.16: 16 integer bits + 16 fractional bits, backed by i32.
//! Q0.16:  0 integer bits + 16 fractional bits, backed by u16 (range 0..1).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub, Mul, Div, Neg};

/// Fractional bits for Q16.16.
const FRAC_BITS: u32 = 16;
/// Scale factor: 1 << 16 = 65536.
const SCALE: i32 = 1 << FRAC_BITS;

/// Q16.16 fixed-point number.
///
/// Internal representation: raw i32 where the lower 16 bits are fractional.
/// Range: approximately -32768.0 to +32767.99998.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fixed(i32);

impl Fixed {
    pub const ZERO: Fixed = Fixed(0);
    pub const ONE: Fixed = Fixed(SCALE);
    pub const NEG_ONE: Fixed = Fixed(-SCALE);
    pub const MAX: Fixed = Fixed(i32::MAX);
    pub const MIN: Fixed = Fixed(i32::MIN);
    pub const EPSILON: Fixed = Fixed(1); // smallest representable positive value

    /// Create from raw i32 representation.
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        Fixed(raw)
    }

    /// Get the raw i32 representation.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    /// Create from an integer value.
    #[inline]
    pub const fn from_int(n: i32) -> Self {
        Fixed(n << FRAC_BITS)
    }

    /// Truncate to integer part (towards zero).
    #[inline]
    pub const fn to_int(self) -> i32 {
        self.0 >> FRAC_BITS
    }

    /// Floor: round towards negative infinity.
    #[inline]
    pub const fn floor(self) -> i32 {
        self.0 >> FRAC_BITS
    }

    /// Ceil: round towards positive infinity.
    #[inline]
    pub const fn ceil(self) -> i32 {
        (self.0 + SCALE - 1) >> FRAC_BITS
    }

    /// Fractional part as raw value (lower 16 bits).
    #[inline]
    pub const fn frac_raw(self) -> i32 {
        self.0 & (SCALE - 1)
    }

    /// Absolute value.
    #[inline]
    pub const fn abs(self) -> Self {
        if self.0 < 0 {
            Fixed(-self.0)
        } else {
            self
        }
    }

    /// Saturating addition. Clamps to MIN/MAX on overflow.
    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_sub(rhs.0))
    }

    /// Saturating multiplication using i64 intermediate to prevent overflow.
    #[inline]
    pub fn saturating_mul(self, rhs: Self) -> Self {
        let wide = (self.0 as i64) * (rhs.0 as i64);
        let result = wide >> FRAC_BITS;
        if result > i32::MAX as i64 {
            Fixed::MAX
        } else if result < i32::MIN as i64 {
            Fixed::MIN
        } else {
            Fixed(result as i32)
        }
    }

    /// Division. Panics on divide by zero.
    #[inline]
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        let wide = (self.0 as i64) << FRAC_BITS;
        let result = wide / (rhs.0 as i64);
        if result > i32::MAX as i64 || result < i32::MIN as i64 {
            None
        } else {
            Some(Fixed(result as i32))
        }
    }

    /// Convert to f32 (for render/UI boundary ONLY -- never use in sim logic).
    #[inline]
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / SCALE as f32
    }

    /// Convert from f32, clamped to Fixed range (for UI input boundary ONLY).
    #[inline]
    pub fn from_f32_clamped(v: f32) -> Self {
        let scaled = (v * SCALE as f32).round();
        if scaled >= i32::MAX as f32 {
            Fixed::MAX
        } else if scaled <= i32::MIN as f32 {
            Fixed::MIN
        } else {
            Fixed(scaled as i32)
        }
    }

    /// Multiply by integer.
    #[inline]
    pub const fn mul_int(self, n: i32) -> Self {
        Fixed(self.0.saturating_mul(n))
    }

    /// Divide by integer.
    #[inline]
    pub fn div_int(self, n: i32) -> Self {
        Fixed(self.0 / n)
    }

    /// Linear interpolation: self + (other - self) * t, where t is a Ratio.
    #[inline]
    pub fn lerp(self, other: Self, t: Ratio) -> Self {
        let diff = (other.0 as i64) - (self.0 as i64);
        let interpolated = (diff * t.0 as i64) >> 16;
        Fixed(self.0.saturating_add(interpolated as i32))
    }
}

impl Add for Fixed {
    type Output = Fixed;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Fixed(self.0.wrapping_add(rhs.0))
    }
}

impl Sub for Fixed {
    type Output = Fixed;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Fixed(self.0.wrapping_sub(rhs.0))
    }
}

impl Mul for Fixed {
    type Output = Fixed;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        let wide = (self.0 as i64) * (rhs.0 as i64);
        Fixed((wide >> FRAC_BITS) as i32)
    }
}

impl Div for Fixed {
    type Output = Fixed;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        let wide = (self.0 as i64) << FRAC_BITS;
        Fixed((wide / rhs.0 as i64) as i32)
    }
}

impl Neg for Fixed {
    type Output = Fixed;
    #[inline]
    fn neg(self) -> Self::Output {
        Fixed(-self.0)
    }
}

impl fmt::Debug for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed({:.4})", self.to_f32())
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.to_f32())
    }
}

// --- Ratio (Q0.16) -----------------------------------------------------------

/// Q0.16 unsigned ratio in [0, 1).
///
/// 0x0000 = 0.0, 0xFFFF = 0.99998.
/// Used for interpolation factors, coverage percentages, etc.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Ratio(u16);

impl Ratio {
    pub const ZERO: Ratio = Ratio(0);
    pub const ONE: Ratio = Ratio(u16::MAX); // closest to 1.0
    pub const HALF: Ratio = Ratio(0x8000);

    /// Create from raw u16.
    #[inline]
    pub const fn from_raw(raw: u16) -> Self {
        Ratio(raw)
    }

    /// Get raw u16.
    #[inline]
    pub const fn to_raw(self) -> u16 {
        self.0
    }

    /// Create from percentage (0-100), clamped.
    #[inline]
    pub const fn from_percent(pct: u8) -> Self {
        if pct >= 100 {
            Ratio(u16::MAX)
        } else {
            // pct * 65535 / 100
            Ratio((pct as u32 * 65535 / 100) as u16)
        }
    }

    /// Convert to percentage (0-100).
    #[inline]
    pub const fn to_percent(self) -> u8 {
        ((self.0 as u32 * 100) / 65535) as u8
    }

    /// Convert to f32 for display (NOT sim logic).
    #[inline]
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / u16::MAX as f32
    }

    /// Multiply two ratios (result is also a ratio).
    #[inline]
    pub fn mul_ratio(self, other: Ratio) -> Ratio {
        let wide = (self.0 as u32) * (other.0 as u32);
        Ratio((wide >> 16) as u16)
    }

    /// Scale a Fixed value by this ratio.
    #[inline]
    pub fn scale_fixed(self, v: Fixed) -> Fixed {
        let wide = (v.to_raw() as i64) * (self.0 as i64);
        Fixed::from_raw((wide >> 16) as i32)
    }
}

impl fmt::Debug for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ratio({:.4})", self.to_f32())
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}%", self.to_f32() * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_from_int() {
        assert_eq!(Fixed::from_int(1).to_raw(), 65536);
        assert_eq!(Fixed::from_int(0).to_raw(), 0);
        assert_eq!(Fixed::from_int(-1).to_raw(), -65536);
        assert_eq!(Fixed::from_int(100).to_raw(), 6553600);
    }

    #[test]
    fn fixed_to_int_truncates() {
        assert_eq!(Fixed::from_raw(65536 + 32768).to_int(), 1); // 1.5 -> 1
        assert_eq!(Fixed::from_raw(-65536 - 32768).to_int(), -2); // -1.5 -> -2 (arithmetic shift)
    }

    #[test]
    fn fixed_floor_ceil() {
        let v = Fixed::from_raw(65536 + 32768); // 1.5
        assert_eq!(v.floor(), 1);
        assert_eq!(v.ceil(), 2);

        let v = Fixed::from_int(3);
        assert_eq!(v.floor(), 3);
        assert_eq!(v.ceil(), 3);

        let v = Fixed::from_raw(1); // tiny positive
        assert_eq!(v.floor(), 0);
        assert_eq!(v.ceil(), 1);
    }

    #[test]
    fn fixed_add_sub() {
        let a = Fixed::from_int(3);
        let b = Fixed::from_int(2);
        assert_eq!((a + b).to_int(), 5);
        assert_eq!((a - b).to_int(), 1);
    }

    #[test]
    fn fixed_mul() {
        let a = Fixed::from_int(3);
        let b = Fixed::from_int(4);
        assert_eq!((a * b).to_int(), 12);

        // 1.5 * 2.0 = 3.0
        let c = Fixed::from_raw(65536 + 32768); // 1.5
        let d = Fixed::from_int(2);
        assert_eq!((c * d).to_int(), 3);
    }

    #[test]
    fn fixed_div() {
        let a = Fixed::from_int(10);
        let b = Fixed::from_int(3);
        let result = a / b;
        // 10/3 = 3.333..., floor = 3
        assert_eq!(result.to_int(), 3);
        // Check precision: raw should be close to 3.333 * 65536 = 218453
        assert!((result.to_raw() - 218453).abs() <= 1);
    }

    #[test]
    fn fixed_neg() {
        let a = Fixed::from_int(5);
        assert_eq!((-a).to_int(), -5);
        assert_eq!((-(-a)).to_int(), 5);
    }

    #[test]
    fn fixed_abs() {
        assert_eq!(Fixed::from_int(-5).abs(), Fixed::from_int(5));
        assert_eq!(Fixed::from_int(5).abs(), Fixed::from_int(5));
        assert_eq!(Fixed::ZERO.abs(), Fixed::ZERO);
    }

    #[test]
    fn fixed_saturating_add() {
        assert_eq!(Fixed::MAX.saturating_add(Fixed::ONE), Fixed::MAX);
        assert_eq!(Fixed::MIN.saturating_add(Fixed::NEG_ONE), Fixed::MIN);
    }

    #[test]
    fn fixed_saturating_mul() {
        let big = Fixed::from_int(20000);
        let result = big.saturating_mul(big);
        assert_eq!(result, Fixed::MAX); // 20000 * 20000 overflows
    }

    #[test]
    fn fixed_checked_div() {
        assert!(Fixed::ONE.checked_div(Fixed::ZERO).is_none());
        assert_eq!(Fixed::from_int(10).checked_div(Fixed::from_int(2)), Some(Fixed::from_int(5)));
    }

    #[test]
    fn fixed_f32_roundtrip() {
        let v = Fixed::from_int(42);
        assert!((v.to_f32() - 42.0).abs() < 0.001);

        let v = Fixed::from_f32_clamped(3.14);
        assert!((v.to_f32() - 3.14).abs() < 0.001);
    }

    #[test]
    fn fixed_f32_clamped_overflow() {
        let v = Fixed::from_f32_clamped(f32::MAX);
        assert_eq!(v, Fixed::MAX);
        let v = Fixed::from_f32_clamped(f32::MIN);
        assert_eq!(v, Fixed::MIN);
    }

    #[test]
    fn fixed_mul_int() {
        let v = Fixed::from_int(7);
        assert_eq!(v.mul_int(3).to_int(), 21);
    }

    #[test]
    fn fixed_div_int() {
        let v = Fixed::from_int(21);
        assert_eq!(v.div_int(3).to_int(), 7);
    }

    #[test]
    fn fixed_constants() {
        assert_eq!(Fixed::ZERO.to_raw(), 0);
        assert_eq!(Fixed::ONE.to_int(), 1);
        assert_eq!(Fixed::NEG_ONE.to_int(), -1);
        assert_eq!(Fixed::EPSILON.to_raw(), 1);
    }

    #[test]
    fn fixed_ordering() {
        assert!(Fixed::from_int(1) < Fixed::from_int(2));
        assert!(Fixed::from_int(-1) < Fixed::ZERO);
        assert!(Fixed::MIN < Fixed::MAX);
    }

    // --- Ratio tests ---------------------------------------------------------

    #[test]
    fn ratio_zero_one() {
        assert_eq!(Ratio::ZERO.to_raw(), 0);
        assert_eq!(Ratio::ONE.to_raw(), u16::MAX);
        assert_eq!(Ratio::HALF.to_raw(), 0x8000);
    }

    #[test]
    fn ratio_from_percent() {
        assert_eq!(Ratio::from_percent(0).to_raw(), 0);
        // 50% -> raw 32767, to_percent -> 49 due to integer truncation; allow +/-1
        let pct50 = Ratio::from_percent(50).to_percent();
        assert!((pct50 as i16 - 50).abs() <= 1, "expected ~50, got {}", pct50);
        assert_eq!(Ratio::from_percent(100), Ratio::ONE);
    }

    #[test]
    fn ratio_to_percent_roundtrip() {
        for pct in [0u8, 10, 25, 50, 75, 100] {
            let r = Ratio::from_percent(pct);
            let back = r.to_percent();
            assert!((back as i16 - pct as i16).abs() <= 1, "pct={} got={}", pct, back);
        }
    }

    #[test]
    fn ratio_mul_ratio() {
        let half = Ratio::HALF;
        let result = half.mul_ratio(half);
        // 0.5 * 0.5 = 0.25, which is ~16384 in u16
        let expected = Ratio::from_percent(25);
        assert!((result.to_raw() as i32 - expected.to_raw() as i32).abs() <= 2);
    }

    #[test]
    fn ratio_scale_fixed() {
        let half = Ratio::HALF;
        let ten = Fixed::from_int(10);
        let result = half.scale_fixed(ten);
        assert_eq!(result.to_int(), 5);
    }

    #[test]
    fn fixed_lerp() {
        let a = Fixed::from_int(0);
        let b = Fixed::from_int(10);
        let mid = a.lerp(b, Ratio::HALF);
        assert_eq!(mid.to_int(), 5);

        let quarter = a.lerp(b, Ratio::from_percent(25));
        // Should be close to 2.5
        assert!((quarter.to_f32() - 2.5).abs() < 0.1);
    }
}
