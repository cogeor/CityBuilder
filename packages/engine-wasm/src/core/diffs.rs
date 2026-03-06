//! State diffs and output model for simulation tick results.
//!
//! Each simulation tick produces a set of `StateDiff` entries describing what
//! changed, plus `MetricUpdate` entries for aggregate values. A `DiffCollector`
//! accumulates these during tick execution; at tick end, they are bundled into
//! a `WorldDiff` for the UI/network layer to consume.

use crate::core_types::*;
use serde::{Deserialize, Serialize};

// ─── EntityField ───────────────────────────────────────────────────────────

/// Identifies which field of an entity was updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityField {
    Level,
    Flags,
    ConstructionProgress,
    Enabled,
}

// ─── TileField ─────────────────────────────────────────────────────────────

/// Identifies which field of a tile was changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileField {
    Terrain(TerrainType),
    Zone(ZoneType),
    Elevation(u8),
}

// ─── StateDiff ─────────────────────────────────────────────────────────────

/// Describes a single state change within one tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateDiff {
    EntityAdded {
        handle: EntityHandle,
        archetype: ArchetypeId,
        pos: TileCoord,
    },
    EntityRemoved {
        handle: EntityHandle,
        pos: TileCoord,
    },
    EntityUpdated {
        handle: EntityHandle,
        field: EntityField,
        old_value: u32,
        new_value: u32,
    },
    TileChanged {
        pos: TileCoord,
        field: TileField,
    },
    PolicyChanged {
        key: String,
        old_value: u32,
        new_value: u32,
    },
    TreasuryChanged {
        old: MoneyCents,
        new_val: MoneyCents,
    },
}

// ─── MetricScope ───────────────────────────────────────────────────────────

/// Scope to which a metric update applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricScope {
    Global,
    District(u16),
    Tile(TileCoord),
    Entity(EntityHandle),
}

// ─── MetricUpdate ──────────────────────────────────────────────────────────

/// A single metric value update produced during a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricUpdate {
    pub metric_id: u16,
    pub scope: MetricScope,
    pub value: i64,
}

// ─── DiffCollector ─────────────────────────────────────────────────────────

/// Accumulates state diffs and metric updates during a simulation tick.
///
/// Systems push diffs and metrics as they execute; at tick end the collector
/// is drained to build a `WorldDiff`.
#[derive(Debug, Clone, Default)]
pub struct DiffCollector {
    diffs: Vec<StateDiff>,
    metrics: Vec<MetricUpdate>,
}

impl DiffCollector {
    /// Create a new, empty collector.
    #[inline]
    pub fn new() -> Self {
        Self {
            diffs: Vec::new(),
            metrics: Vec::new(),
        }
    }

    /// Record a state diff.
    #[inline]
    pub fn push_diff(&mut self, diff: StateDiff) {
        self.diffs.push(diff);
    }

    /// Record a metric update.
    #[inline]
    pub fn push_metric(&mut self, metric: MetricUpdate) {
        self.metrics.push(metric);
    }

    /// Drain all accumulated diffs, leaving the internal vec empty.
    #[inline]
    pub fn drain_diffs(&mut self) -> Vec<StateDiff> {
        std::mem::take(&mut self.diffs)
    }

    /// Drain all accumulated metrics, leaving the internal vec empty.
    #[inline]
    pub fn drain_metrics(&mut self) -> Vec<MetricUpdate> {
        std::mem::take(&mut self.metrics)
    }

    /// Total number of accumulated diffs and metrics.
    #[inline]
    pub fn len(&self) -> usize {
        self.diffs.len() + self.metrics.len()
    }

    /// Returns `true` if no diffs or metrics have been recorded.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.diffs.is_empty() && self.metrics.is_empty()
    }
}

// ─── WorldDiff ─────────────────────────────────────────────────────────────

/// Complete summary of all changes produced by a single simulation tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDiff {
    pub tick: Tick,
    pub diffs: Vec<StateDiff>,
    pub metrics: Vec<MetricUpdate>,
}

impl WorldDiff {
    /// Build a `WorldDiff` by draining a collector at the given tick.
    pub fn from_collector(tick: Tick, collector: &mut DiffCollector) -> Self {
        Self {
            tick,
            diffs: collector.drain_diffs(),
            metrics: collector.drain_metrics(),
        }
    }

    /// Construct an empty `WorldDiff` for the given tick with no diffs or metrics.
    ///
    /// Used when a tick is skipped (e.g. when the simulation is paused) and a
    /// valid `WorldDiff` value is still required by the caller.
    pub fn empty(tick: Tick) -> Self {
        Self {
            tick,
            diffs: Vec::new(),
            metrics: Vec::new(),
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_handle() -> EntityHandle {
        EntityHandle::new(1, 1)
    }

    fn sample_pos() -> TileCoord {
        TileCoord::new(5, 10)
    }

    #[test]
    fn collector_starts_empty() {
        let c = DiffCollector::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn push_diff_and_drain() {
        let mut c = DiffCollector::new();
        c.push_diff(StateDiff::EntityAdded {
            handle: sample_handle(),
            archetype: 1,
            pos: sample_pos(),
        });
        assert_eq!(c.len(), 1);
        let drained = c.drain_diffs();
        assert_eq!(drained.len(), 1);
        assert!(c.diffs.is_empty());
    }

    #[test]
    fn push_metric_and_drain() {
        let mut c = DiffCollector::new();
        c.push_metric(MetricUpdate {
            metric_id: 42,
            scope: MetricScope::Global,
            value: 1000,
        });
        assert_eq!(c.len(), 1);
        let drained = c.drain_metrics();
        assert_eq!(drained.len(), 1);
        assert!(c.metrics.is_empty());
    }

    #[test]
    fn multiple_diffs_accumulate() {
        let mut c = DiffCollector::new();
        c.push_diff(StateDiff::EntityAdded {
            handle: sample_handle(),
            archetype: 1,
            pos: sample_pos(),
        });
        c.push_diff(StateDiff::EntityRemoved {
            handle: sample_handle(),
            pos: sample_pos(),
        });
        c.push_diff(StateDiff::TreasuryChanged {
            old: 500,
            new_val: 300,
        });
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn drain_clears_collector() {
        let mut c = DiffCollector::new();
        c.push_diff(StateDiff::TreasuryChanged {
            old: 100,
            new_val: 200,
        });
        c.push_metric(MetricUpdate {
            metric_id: 1,
            scope: MetricScope::Global,
            value: 50,
        });
        assert!(!c.is_empty());
        let _ = c.drain_diffs();
        let _ = c.drain_metrics();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn state_diff_variants_constructable() {
        let _added = StateDiff::EntityAdded {
            handle: sample_handle(),
            archetype: 5,
            pos: sample_pos(),
        };
        let _removed = StateDiff::EntityRemoved {
            handle: sample_handle(),
            pos: sample_pos(),
        };
        let _updated = StateDiff::EntityUpdated {
            handle: sample_handle(),
            field: EntityField::Level,
            old_value: 1,
            new_value: 2,
        };
        let _tile = StateDiff::TileChanged {
            pos: sample_pos(),
            field: TileField::Zone(ZoneType::Residential),
        };
        let _policy = StateDiff::PolicyChanged {
            key: "tax_rate".to_string(),
            old_value: 10,
            new_value: 12,
        };
        let _treasury = StateDiff::TreasuryChanged {
            old: 1000,
            new_val: 900,
        };
    }

    #[test]
    fn metric_scope_variants() {
        let _global = MetricScope::Global;
        let _district = MetricScope::District(3);
        let _tile = MetricScope::Tile(sample_pos());
        let _entity = MetricScope::Entity(sample_handle());

        // Verify equality works across variants
        assert_eq!(MetricScope::Global, MetricScope::Global);
        assert_ne!(MetricScope::Global, MetricScope::District(0));
        assert_eq!(MetricScope::District(3), MetricScope::District(3));
        assert_ne!(MetricScope::District(1), MetricScope::District(2));
    }

    #[test]
    fn world_diff_from_collector() {
        let mut c = DiffCollector::new();
        c.push_diff(StateDiff::TreasuryChanged {
            old: 1000,
            new_val: 800,
        });
        c.push_metric(MetricUpdate {
            metric_id: 10,
            scope: MetricScope::Global,
            value: -200,
        });

        let wd = WorldDiff::from_collector(42, &mut c);
        assert_eq!(wd.tick, 42);
        assert_eq!(wd.diffs.len(), 1);
        assert_eq!(wd.metrics.len(), 1);
        assert!(c.is_empty());
    }

    #[test]
    fn entity_field_variants() {
        let fields = [
            EntityField::Level,
            EntityField::Flags,
            EntityField::ConstructionProgress,
            EntityField::Enabled,
        ];
        // All variants are distinct
        for i in 0..fields.len() {
            for j in (i + 1)..fields.len() {
                assert_ne!(fields[i], fields[j]);
            }
        }
    }

    #[test]
    fn tile_field_variants() {
        let terrain = TileField::Terrain(TerrainType::Grass);
        let zone = TileField::Zone(ZoneType::Residential);
        let elevation = TileField::Elevation(10);

        assert_ne!(terrain, zone);
        assert_ne!(zone, elevation);
        assert_eq!(
            TileField::Terrain(TerrainType::Water),
            TileField::Terrain(TerrainType::Water)
        );
    }

    #[test]
    fn is_empty_len_correct() {
        let mut c = DiffCollector::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);

        c.push_diff(StateDiff::EntityRemoved {
            handle: sample_handle(),
            pos: sample_pos(),
        });
        assert!(!c.is_empty());
        assert_eq!(c.len(), 1);

        c.push_metric(MetricUpdate {
            metric_id: 99,
            scope: MetricScope::District(7),
            value: 42,
        });
        assert_eq!(c.len(), 2);

        // Draining only diffs keeps metrics
        let _ = c.drain_diffs();
        assert!(!c.is_empty());
        assert_eq!(c.len(), 1);
    }
}
