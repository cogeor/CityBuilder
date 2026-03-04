//! Allocation tracking for debug/bench builds.
//! Feature-gated: only active when `alloc_tracking` feature is enabled.
//! In release builds, all functions are no-ops with zero cost.

use std::sync::atomic::{AtomicU64, Ordering};

/// Global allocation counters.
static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static FREE_COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);
static BYTES_FREED: AtomicU64 = AtomicU64::new(0);
static PEAK_BYTES: AtomicU64 = AtomicU64::new(0);

/// Memory budget constants.
pub const MAX_HEAP_BYTES: u64 = 64 * 1024 * 1024; // 64 MB budget
pub const MAX_WASM_PAGES: u32 = 1024; // 64 MB in WASM pages (64KB each)

/// Allocation statistics snapshot.
#[derive(Debug, Clone, Default)]
pub struct AllocStats {
    pub alloc_count: u64,
    pub free_count: u64,
    pub bytes_allocated: u64,
    pub bytes_freed: u64,
    pub peak_bytes: u64,
    pub current_bytes: u64,
}

/// Record an allocation event.
pub fn record_alloc(bytes: usize) {
    ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    let prev = BYTES_ALLOCATED.fetch_add(bytes as u64, Ordering::Relaxed);
    let current = prev + bytes as u64 - BYTES_FREED.load(Ordering::Relaxed);
    // Update peak
    loop {
        let peak = PEAK_BYTES.load(Ordering::Relaxed);
        if current <= peak {
            break;
        }
        match PEAK_BYTES.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

/// Record a free event.
pub fn record_free(bytes: usize) {
    FREE_COUNT.fetch_add(1, Ordering::Relaxed);
    BYTES_FREED.fetch_add(bytes as u64, Ordering::Relaxed);
}

/// Get current allocation statistics.
pub fn get_stats() -> AllocStats {
    let alloc_count = ALLOC_COUNT.load(Ordering::Relaxed);
    let free_count = FREE_COUNT.load(Ordering::Relaxed);
    let bytes_allocated = BYTES_ALLOCATED.load(Ordering::Relaxed);
    let bytes_freed = BYTES_FREED.load(Ordering::Relaxed);
    let peak_bytes = PEAK_BYTES.load(Ordering::Relaxed);
    AllocStats {
        alloc_count,
        free_count,
        bytes_allocated,
        bytes_freed,
        peak_bytes,
        current_bytes: bytes_allocated.saturating_sub(bytes_freed),
    }
}

/// Reset all counters.
pub fn reset_stats() {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    FREE_COUNT.store(0, Ordering::Relaxed);
    BYTES_ALLOCATED.store(0, Ordering::Relaxed);
    BYTES_FREED.store(0, Ordering::Relaxed);
    PEAK_BYTES.store(0, Ordering::Relaxed);
}

/// Check if current allocation is within budget.
pub fn is_within_budget() -> bool {
    let stats = get_stats();
    stats.current_bytes <= MAX_HEAP_BYTES
}

/// Trait for allocation tracking behavior.
pub trait IAllocTracker {
    fn on_alloc(&self, bytes: usize);
    fn on_free(&self, bytes: usize);
    fn stats(&self) -> AllocStats;
    fn reset(&self);
}

/// Default tracker using global atomics.
pub struct GlobalAllocTracker;

impl IAllocTracker for GlobalAllocTracker {
    fn on_alloc(&self, bytes: usize) {
        record_alloc(bytes);
    }
    fn on_free(&self, bytes: usize) {
        record_free(bytes);
    }
    fn stats(&self) -> AllocStats {
        get_stats()
    }
    fn reset(&self) {
        reset_stats();
    }
}

/// No-op tracker for release builds.
pub struct NoOpAllocTracker;

impl IAllocTracker for NoOpAllocTracker {
    fn on_alloc(&self, _bytes: usize) {}
    fn on_free(&self, _bytes: usize) {}
    fn stats(&self) -> AllocStats {
        AllocStats::default()
    }
    fn reset(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_alloc_increments_count_and_bytes() {
        reset_stats();
        record_alloc(1024);
        let stats = get_stats();
        assert_eq!(stats.alloc_count, 1);
        assert_eq!(stats.bytes_allocated, 1024);
    }

    #[test]
    fn record_free_increments_count_and_bytes() {
        reset_stats();
        record_alloc(2048);
        record_free(512);
        let stats = get_stats();
        assert_eq!(stats.free_count, 1);
        assert_eq!(stats.bytes_freed, 512);
    }

    #[test]
    fn get_stats_returns_correct_current_bytes() {
        reset_stats();
        record_alloc(4096);
        record_free(1024);
        let stats = get_stats();
        assert_eq!(stats.current_bytes, 3072);
    }

    #[test]
    fn peak_bytes_tracks_maximum() {
        reset_stats();
        record_alloc(8000);
        record_free(4000);
        record_alloc(2000);
        let stats = get_stats();
        // Peak should be 8000 (the first allocation before any frees).
        assert_eq!(stats.peak_bytes, 8000);
        // Current should be 8000 - 4000 + 2000 = 6000.
        assert_eq!(stats.current_bytes, 6000);
    }

    #[test]
    fn reset_stats_clears_everything() {
        reset_stats();
        record_alloc(1000);
        record_free(500);
        reset_stats();
        let stats = get_stats();
        assert_eq!(stats.alloc_count, 0);
        assert_eq!(stats.free_count, 0);
        assert_eq!(stats.bytes_allocated, 0);
        assert_eq!(stats.bytes_freed, 0);
        assert_eq!(stats.peak_bytes, 0);
        assert_eq!(stats.current_bytes, 0);
    }

    #[test]
    fn is_within_budget_returns_true_under_limit() {
        reset_stats();
        record_alloc(1024);
        assert!(is_within_budget());
    }

    #[test]
    fn global_alloc_tracker_implements_trait() {
        reset_stats();
        let tracker = GlobalAllocTracker;
        tracker.on_alloc(2048);
        let stats = tracker.stats();
        assert_eq!(stats.alloc_count, 1);
        assert_eq!(stats.bytes_allocated, 2048);
        tracker.on_free(1024);
        let stats = tracker.stats();
        assert_eq!(stats.free_count, 1);
        assert_eq!(stats.current_bytes, 1024);
        tracker.reset();
        let stats = tracker.stats();
        assert_eq!(stats.current_bytes, 0);
    }

    #[test]
    fn noop_alloc_tracker_returns_defaults() {
        let tracker = NoOpAllocTracker;
        tracker.on_alloc(9999);
        tracker.on_free(5000);
        let stats = tracker.stats();
        assert_eq!(stats.alloc_count, 0);
        assert_eq!(stats.free_count, 0);
        assert_eq!(stats.bytes_allocated, 0);
        assert_eq!(stats.bytes_freed, 0);
        assert_eq!(stats.peak_bytes, 0);
        assert_eq!(stats.current_bytes, 0);
    }

    #[test]
    fn alloc_stats_default_is_zeroed() {
        let stats = AllocStats::default();
        assert_eq!(stats.alloc_count, 0);
        assert_eq!(stats.free_count, 0);
        assert_eq!(stats.bytes_allocated, 0);
        assert_eq!(stats.bytes_freed, 0);
        assert_eq!(stats.peak_bytes, 0);
        assert_eq!(stats.current_bytes, 0);
    }

    #[test]
    fn constants_are_correct() {
        assert_eq!(MAX_HEAP_BYTES, 64 * 1024 * 1024);
        assert_eq!(MAX_WASM_PAGES, 1024);
        // Verify consistency: MAX_WASM_PAGES * 64KB = MAX_HEAP_BYTES
        assert_eq!(MAX_WASM_PAGES as u64 * 64 * 1024, MAX_HEAP_BYTES);
    }
}
