//! Deterministic PRNG for simulation.
//!
//! Uses PCG-XSH-RR (32-bit output, 64-bit state).
//! All randomness in the simulation flows through this module.
//! No platform RNG allowed.

/// PCG-XSH-RR 32-bit PRNG with 64-bit state.
/// Deterministic and portable across all platforms.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    /// PCG multiplier constant.
    const MULTIPLIER: u64 = 6364136223846793005;

    /// Create a new RNG with the given seed.
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng { state: 0, inc: 1 };
        // PCG seeding protocol
        rng.next_u32(); // advance state
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32(); // mix
        rng
    }

    /// Create a new RNG with seed and stream (sequence).
    pub fn with_stream(seed: u64, stream: u64) -> Self {
        let mut rng = Rng {
            state: 0,
            inc: (stream << 1) | 1, // must be odd
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// Generate next u32 value.
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        // Advance state
        self.state = old_state
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(self.inc);
        // PCG-XSH-RR output function
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Generate a u32 in [0, bound) using rejection sampling.
    pub fn next_bounded(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        let threshold = bound.wrapping_neg() % bound; // (2^32 - bound) % bound
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % bound;
            }
        }
    }

    /// Generate a bool with the given probability (0-65535 as Q0.16).
    /// probability = 0 -> always false, probability = 65535 -> almost always true.
    pub fn chance(&mut self, probability_q16: u16) -> bool {
        (self.next_u32() >> 16) as u16 <= probability_q16
    }

    /// Generate a value in [min, max] inclusive.
    pub fn range_inclusive(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u32 + 1;
        min + self.next_bounded(range) as i32
    }

    /// Fork: derive a child RNG from a string key (deterministic).
    /// Uses FNV-1a hash of the key to create a new seed.
    pub fn fork(&self, key: &str) -> Rng {
        let seed = fnv1a_hash(self.state, key);
        Rng::new(seed)
    }
}

/// Derive a per-system seed from root seed + system name.
/// Uses FNV-1a for simplicity and determinism.
pub fn derive_system_seed(root_seed: u64, system_name: &str) -> u64 {
    fnv1a_hash(root_seed, system_name)
}

/// FNV-1a hash (64-bit) for deterministic seed derivation.
fn fnv1a_hash(base: u64, key: &str) -> u64 {
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;

    let mut hash = FNV_OFFSET ^ base;
    for byte in key.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut rng1 = Rng::new(42);
        let mut rng2 = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn different_seeds_different_sequences() {
        let mut rng1 = Rng::new(42);
        let mut rng2 = Rng::new(99);
        let mut all_same = true;
        for _ in 0..100 {
            if rng1.next_u32() != rng2.next_u32() {
                all_same = false;
                break;
            }
        }
        assert!(!all_same, "Different seeds should produce different sequences");
    }

    #[test]
    fn replay_determinism() {
        let mut rng1 = Rng::new(12345);
        let seq1: Vec<u32> = (0..1000).map(|_| rng1.next_u32()).collect();

        let mut rng2 = Rng::new(12345);
        let seq2: Vec<u32> = (0..1000).map(|_| rng2.next_u32()).collect();

        assert_eq!(seq1, seq2, "Replayed RNG must produce identical sequences");
    }

    #[test]
    fn fork_produces_deterministic_child() {
        let rng = Rng::new(42);
        let mut child1 = rng.fork("population");
        let mut child2 = rng.fork("population");

        for _ in 0..100 {
            assert_eq!(child1.next_u32(), child2.next_u32());
        }
    }

    #[test]
    fn derive_system_seed_deterministic() {
        let seed1 = derive_system_seed(42, "economy");
        let seed2 = derive_system_seed(42, "economy");
        assert_eq!(seed1, seed2);
    }

    #[test]
    fn different_system_names_different_seeds() {
        let seed1 = derive_system_seed(42, "economy");
        let seed2 = derive_system_seed(42, "population");
        assert_ne!(seed1, seed2);
    }

    #[test]
    fn next_bounded_stays_in_range() {
        let mut rng = Rng::new(42);
        for bound in [1, 2, 3, 5, 10, 100, 1000, u32::MAX] {
            for _ in 0..1000 {
                let val = rng.next_bounded(bound);
                assert!(val < bound, "next_bounded({}) returned {}", bound, val);
            }
        }
    }

    #[test]
    fn next_bounded_zero_returns_zero() {
        let mut rng = Rng::new(42);
        assert_eq!(rng.next_bounded(0), 0);
    }

    #[test]
    fn range_inclusive_stays_in_range() {
        let mut rng = Rng::new(42);
        for _ in 0..10000 {
            let val = rng.range_inclusive(-10, 10);
            assert!(
                val >= -10 && val <= 10,
                "range_inclusive(-10, 10) returned {}",
                val
            );
        }
    }

    #[test]
    fn range_inclusive_min_equals_max() {
        let mut rng = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(rng.range_inclusive(5, 5), 5);
        }
    }

    #[test]
    fn chance_zero_never_true() {
        let mut rng = Rng::new(42);
        for _ in 0..1000 {
            assert!(!rng.chance(0), "chance(0) should never return true");
        }
    }

    #[test]
    fn chance_max_almost_always_true() {
        let mut rng = Rng::new(42);
        let mut true_count = 0;
        for _ in 0..1000 {
            if rng.chance(65535) {
                true_count += 1;
            }
        }
        assert!(
            true_count > 990,
            "chance(65535) should be almost always true, got {}/1000",
            true_count
        );
    }

    #[test]
    fn with_stream_deterministic() {
        let mut rng1 = Rng::with_stream(42, 7);
        let mut rng2 = Rng::with_stream(42, 7);
        for _ in 0..100 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn different_streams_different_sequences() {
        let mut rng1 = Rng::with_stream(42, 1);
        let mut rng2 = Rng::with_stream(42, 2);
        let mut all_same = true;
        for _ in 0..100 {
            if rng1.next_u32() != rng2.next_u32() {
                all_same = false;
                break;
            }
        }
        assert!(
            !all_same,
            "Different streams should produce different sequences"
        );
    }

    #[test]
    fn fork_different_keys_different_children() {
        let rng = Rng::new(42);
        let mut child1 = rng.fork("economy");
        let mut child2 = rng.fork("population");
        let mut all_same = true;
        for _ in 0..100 {
            if child1.next_u32() != child2.next_u32() {
                all_same = false;
                break;
            }
        }
        assert!(
            !all_same,
            "Different fork keys should produce different children"
        );
    }
}
