//! Historical statistics recorder -- persistent city metrics.
//!
//! Captures monthly snapshots of key city metrics (population, budget balance,
//! unemployment, etc.) in per-metric ring buffers. Supports query by tick range,
//! compact binary serialization for save files, and configurable buffer depth
//! (default 120 months = 10 game-years).

/// Metric identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MetricId {
    Population = 0,
    BudgetBalance = 1,
    Unemployment = 2,
    AvgHappiness = 3,
    AvgCommuteTime = 4,
    CrimeRate = 5,
    PollutionIndex = 6,
    TrafficCongestion = 7,
}

const METRIC_COUNT: usize = 8;

impl MetricId {
    /// Convert a u8 tag back into a MetricId.
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(MetricId::Population),
            1 => Some(MetricId::BudgetBalance),
            2 => Some(MetricId::Unemployment),
            3 => Some(MetricId::AvgHappiness),
            4 => Some(MetricId::AvgCommuteTime),
            5 => Some(MetricId::CrimeRate),
            6 => Some(MetricId::PollutionIndex),
            7 => Some(MetricId::TrafficCongestion),
            _ => None,
        }
    }
}

/// A single recorded data point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataPoint {
    pub tick: u64,
    pub value: i32,
}

/// Trait for metric snapshot + query API.
pub trait IStatsRecorder {
    /// Record a metric value at the given tick.
    fn snapshot(&mut self, metric: MetricId, tick: u64, value: i32);

    /// Query data points for a metric within a tick range (inclusive).
    fn query(&self, metric: MetricId, start_tick: u64, end_tick: u64) -> Vec<DataPoint>;

    /// Return the most recently recorded data point for a metric.
    fn latest(&self, metric: MetricId) -> Option<DataPoint>;

    /// Clear all recorded data.
    fn clear(&mut self);
}

/// Ring buffer implementation of IStatsRecorder.
///
/// Each metric gets its own fixed-capacity ring buffer. When the buffer is full
/// the oldest entry is overwritten.
pub struct RingStatsRecorder {
    buffers: [Vec<DataPoint>; METRIC_COUNT],
    capacity: usize,
    write_indices: [usize; METRIC_COUNT],
    counts: [usize; METRIC_COUNT],
}

impl RingStatsRecorder {
    /// Create a new recorder with the given per-metric capacity.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be > 0");
        let buffers = std::array::from_fn(|_| {
            let mut v = Vec::with_capacity(capacity);
            v.resize(capacity, DataPoint { tick: 0, value: 0 });
            v
        });
        RingStatsRecorder {
            buffers,
            capacity,
            write_indices: [0; METRIC_COUNT],
            counts: [0; METRIC_COUNT],
        }
    }

    /// Create a recorder with the default capacity of 120 (10 game-years of
    /// monthly snapshots).
    pub fn with_default_capacity() -> Self {
        Self::new(120)
    }

    /// Current capacity per metric.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of recorded entries for a given metric.
    #[inline]
    pub fn count(&self, metric: MetricId) -> usize {
        self.counts[metric as usize]
    }
}

impl IStatsRecorder for RingStatsRecorder {
    fn snapshot(&mut self, metric: MetricId, tick: u64, value: i32) {
        let idx = metric as usize;
        let wi = self.write_indices[idx];
        self.buffers[idx][wi] = DataPoint { tick, value };
        self.write_indices[idx] = (wi + 1) % self.capacity;
        if self.counts[idx] < self.capacity {
            self.counts[idx] += 1;
        }
    }

    fn query(&self, metric: MetricId, start_tick: u64, end_tick: u64) -> Vec<DataPoint> {
        let idx = metric as usize;
        let count = self.counts[idx];
        if count == 0 {
            return Vec::new();
        }

        // Iterate stored entries in chronological order.
        let start_pos = if count < self.capacity {
            0
        } else {
            self.write_indices[idx] // oldest entry when buffer is full
        };

        let mut result = Vec::new();
        for i in 0..count {
            let pos = (start_pos + i) % self.capacity;
            let dp = self.buffers[idx][pos];
            if dp.tick >= start_tick && dp.tick <= end_tick {
                result.push(dp);
            }
        }
        result
    }

    fn latest(&self, metric: MetricId) -> Option<DataPoint> {
        let idx = metric as usize;
        if self.counts[idx] == 0 {
            return None;
        }
        // The most recent write is at (write_index - 1).
        let wi = self.write_indices[idx];
        let pos = if wi == 0 { self.capacity - 1 } else { wi - 1 };
        Some(self.buffers[idx][pos])
    }

    fn clear(&mut self) {
        for idx in 0..METRIC_COUNT {
            self.write_indices[idx] = 0;
            self.counts[idx] = 0;
        }
    }
}

// ─── Serialization ──────────────────────────────────────────────────────────
//
// Binary format per entry: metric_id (u8) + tick (u32) + value (i32) = 9 bytes.
// Header: capacity (u32, 4 bytes).
// Tick is stored as u32 (low 32 bits) to keep the format compact; games are
// unlikely to exceed ~4 billion ticks.

/// Serialize a RingStatsRecorder into a compact binary blob.
pub fn serialize_stats(recorder: &RingStatsRecorder) -> Vec<u8> {
    // Count total entries across all metrics.
    let total: usize = recorder.counts.iter().sum();
    // 4 bytes header + 9 bytes per entry
    let mut buf = Vec::with_capacity(4 + total * 9);

    // Header: capacity as u32 LE
    buf.extend_from_slice(&(recorder.capacity as u32).to_le_bytes());

    for metric_idx in 0..METRIC_COUNT {
        let count = recorder.counts[metric_idx];
        if count == 0 {
            continue;
        }
        let start_pos = if count < recorder.capacity {
            0
        } else {
            recorder.write_indices[metric_idx]
        };
        for i in 0..count {
            let pos = (start_pos + i) % recorder.capacity;
            let dp = recorder.buffers[metric_idx][pos];
            buf.push(metric_idx as u8);
            buf.extend_from_slice(&(dp.tick as u32).to_le_bytes());
            buf.extend_from_slice(&dp.value.to_le_bytes());
        }
    }
    buf
}

/// Deserialize a binary blob back into a RingStatsRecorder.
/// Returns `None` if the data is invalid or too short.
pub fn deserialize_stats(data: &[u8]) -> Option<RingStatsRecorder> {
    if data.len() < 4 {
        return None;
    }

    let capacity = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
    if capacity == 0 {
        return None;
    }

    let mut recorder = RingStatsRecorder::new(capacity);
    let entry_data = &data[4..];

    if entry_data.len() % 9 != 0 {
        return None;
    }

    let entry_count = entry_data.len() / 9;
    for i in 0..entry_count {
        let offset = i * 9;
        let metric_u8 = entry_data[offset];
        let _metric = MetricId::from_u8(metric_u8)?;
        let tick = u32::from_le_bytes(
            entry_data[offset + 1..offset + 5].try_into().ok()?,
        ) as u64;
        let value = i32::from_le_bytes(
            entry_data[offset + 5..offset + 9].try_into().ok()?,
        );
        // Re-insert via snapshot to rebuild ring buffer state.
        recorder.snapshot(_metric, tick, value);
    }

    Some(recorder)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_stores_data_point() {
        let mut rec = RingStatsRecorder::new(10);
        rec.snapshot(MetricId::Population, 100, 5000);
        assert_eq!(rec.count(MetricId::Population), 1);
        let pts = rec.query(MetricId::Population, 0, 200);
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].tick, 100);
        assert_eq!(pts[0].value, 5000);
    }

    #[test]
    fn query_returns_correct_range() {
        let mut rec = RingStatsRecorder::new(10);
        rec.snapshot(MetricId::BudgetBalance, 10, 100);
        rec.snapshot(MetricId::BudgetBalance, 20, 200);
        rec.snapshot(MetricId::BudgetBalance, 30, 300);
        rec.snapshot(MetricId::BudgetBalance, 40, 400);

        let pts = rec.query(MetricId::BudgetBalance, 15, 35);
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].tick, 20);
        assert_eq!(pts[1].tick, 30);
    }

    #[test]
    fn ring_buffer_wraps_correctly() {
        let mut rec = RingStatsRecorder::new(3);
        rec.snapshot(MetricId::CrimeRate, 1, 10);
        rec.snapshot(MetricId::CrimeRate, 2, 20);
        rec.snapshot(MetricId::CrimeRate, 3, 30);
        // Buffer is now full; next write overwrites oldest.
        rec.snapshot(MetricId::CrimeRate, 4, 40);
        assert_eq!(rec.count(MetricId::CrimeRate), 3);

        let pts = rec.query(MetricId::CrimeRate, 0, 100);
        assert_eq!(pts.len(), 3);
        // tick=1 should be gone
        assert_eq!(pts[0].tick, 2);
        assert_eq!(pts[1].tick, 3);
        assert_eq!(pts[2].tick, 4);
    }

    #[test]
    fn latest_returns_most_recent() {
        let mut rec = RingStatsRecorder::new(10);
        rec.snapshot(MetricId::AvgHappiness, 5, 70);
        rec.snapshot(MetricId::AvgHappiness, 10, 80);
        rec.snapshot(MetricId::AvgHappiness, 15, 90);

        let latest = rec.latest(MetricId::AvgHappiness).unwrap();
        assert_eq!(latest.tick, 15);
        assert_eq!(latest.value, 90);
    }

    #[test]
    fn query_empty_metric_returns_empty() {
        let rec = RingStatsRecorder::new(10);
        let pts = rec.query(MetricId::Unemployment, 0, 1000);
        assert!(pts.is_empty());
    }

    #[test]
    fn latest_on_empty_returns_none() {
        let rec = RingStatsRecorder::new(10);
        assert!(rec.latest(MetricId::Population).is_none());
    }

    #[test]
    fn clear_removes_all_data() {
        let mut rec = RingStatsRecorder::new(10);
        rec.snapshot(MetricId::Population, 1, 100);
        rec.snapshot(MetricId::CrimeRate, 2, 50);
        rec.clear();
        assert_eq!(rec.count(MetricId::Population), 0);
        assert_eq!(rec.count(MetricId::CrimeRate), 0);
        assert!(rec.latest(MetricId::Population).is_none());
    }

    #[test]
    fn capacity_limits_work() {
        let mut rec = RingStatsRecorder::new(5);
        for i in 0..20 {
            rec.snapshot(MetricId::PollutionIndex, i as u64, i);
        }
        assert_eq!(rec.count(MetricId::PollutionIndex), 5);

        let pts = rec.query(MetricId::PollutionIndex, 0, 100);
        assert_eq!(pts.len(), 5);
        // Only the last 5 entries should remain (15..19).
        assert_eq!(pts[0].value, 15);
        assert_eq!(pts[4].value, 19);
    }

    #[test]
    fn multiple_metrics_independent() {
        let mut rec = RingStatsRecorder::new(10);
        rec.snapshot(MetricId::Population, 1, 1000);
        rec.snapshot(MetricId::CrimeRate, 1, 50);
        rec.snapshot(MetricId::TrafficCongestion, 1, 80);

        assert_eq!(rec.count(MetricId::Population), 1);
        assert_eq!(rec.count(MetricId::CrimeRate), 1);
        assert_eq!(rec.count(MetricId::TrafficCongestion), 1);
        assert_eq!(rec.count(MetricId::BudgetBalance), 0);

        let pop = rec.query(MetricId::Population, 0, 10);
        assert_eq!(pop[0].value, 1000);
        let crime = rec.query(MetricId::CrimeRate, 0, 10);
        assert_eq!(crime[0].value, 50);
    }

    #[test]
    fn serialize_deserialize_round_trip() {
        let mut rec = RingStatsRecorder::new(10);
        rec.snapshot(MetricId::Population, 100, 5000);
        rec.snapshot(MetricId::BudgetBalance, 100, -2000);
        rec.snapshot(MetricId::CrimeRate, 200, 42);
        rec.snapshot(MetricId::Population, 200, 5500);

        let bytes = serialize_stats(&rec);
        let restored = deserialize_stats(&bytes).expect("deserialization should succeed");

        assert_eq!(restored.capacity(), rec.capacity());

        // Verify all data points survived the round trip.
        let pop = restored.query(MetricId::Population, 0, 1000);
        assert_eq!(pop.len(), 2);
        assert_eq!(pop[0], DataPoint { tick: 100, value: 5000 });
        assert_eq!(pop[1], DataPoint { tick: 200, value: 5500 });

        let budget = restored.query(MetricId::BudgetBalance, 0, 1000);
        assert_eq!(budget.len(), 1);
        assert_eq!(budget[0].value, -2000);

        let crime = restored.query(MetricId::CrimeRate, 0, 1000);
        assert_eq!(crime.len(), 1);
        assert_eq!(crime[0].value, 42);
    }

    #[test]
    fn deserialize_invalid_data_returns_none() {
        // Too short
        assert!(deserialize_stats(&[]).is_none());
        assert!(deserialize_stats(&[1, 2]).is_none());

        // Zero capacity
        assert!(deserialize_stats(&[0, 0, 0, 0]).is_none());

        // Non-multiple-of-9 payload
        assert!(deserialize_stats(&[10, 0, 0, 0, 1, 2, 3]).is_none());

        // Invalid metric id
        let mut bad = Vec::new();
        bad.extend_from_slice(&10u32.to_le_bytes()); // capacity = 10
        bad.push(99); // invalid metric id
        bad.extend_from_slice(&1u32.to_le_bytes());
        bad.extend_from_slice(&1i32.to_le_bytes());
        assert!(deserialize_stats(&bad).is_none());
    }

    #[test]
    fn default_capacity_is_120() {
        let rec = RingStatsRecorder::with_default_capacity();
        assert_eq!(rec.capacity(), 120);
    }

    #[test]
    fn latest_after_wrap() {
        let mut rec = RingStatsRecorder::new(3);
        rec.snapshot(MetricId::AvgCommuteTime, 1, 10);
        rec.snapshot(MetricId::AvgCommuteTime, 2, 20);
        rec.snapshot(MetricId::AvgCommuteTime, 3, 30);
        rec.snapshot(MetricId::AvgCommuteTime, 4, 40);
        // write_index is now 1 (wrapped), latest should be at index 0 = tick 4
        let latest = rec.latest(MetricId::AvgCommuteTime).unwrap();
        assert_eq!(latest.tick, 4);
        assert_eq!(latest.value, 40);
    }
}
