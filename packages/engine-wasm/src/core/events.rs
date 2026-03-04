//! Typed event bus for simulation events.
//!
//! Systems publish events during tick execution. Subscribers (UI, sound, etc.)
//! consume them via drain(). Events are not persisted — they are ephemeral
//! per-tick notifications.

use crate::core_types::*;
use serde::{Deserialize, Serialize};

/// Simulation event types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimEvent {
    BuildingCompleted { handle: EntityHandle, archetype: ArchetypeId },
    BuildingDemolished { handle: EntityHandle },
    PopulationChanged { old: u32, new: u32 },
    HousingShortage { deficit: u32 },
    MigrationWave { incoming: i32 },
    UnemploymentHigh { rate_pct: u8 },
    LaborShortage { deficit: u32 },
    PowerShortage { deficit_kw: u32 },
    WaterShortage { deficit: u32 },
    UtilityRestored { utility_type: UtilityType },
    BudgetDeficit { amount_cents: MoneyCents },
    BudgetSurplus { amount_cents: MoneyCents },
    DebtWarning { treasury_cents: MoneyCents },
    FireStarted { location: TileCoord, entity: EntityHandle },
    FireExtinguished { location: TileCoord },
    FireSpread { from: TileCoord, to: TileCoord },
    CrimeWave { district: u16, severity: u8 },
    TrafficJam { location: TileCoord, density: u16 },
    MilestoneReached { milestone_id: u16 },
    CommandRejected { reason: String },
}

/// Utility type for shortage/restoration events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum UtilityType {
    Power = 0,
    Water = 1,
}

/// Timestamped event wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampedEvent {
    pub tick: Tick,
    pub event: SimEvent,
}

/// Event bus: accumulate events during a tick, drain at the end.
#[derive(Debug, Default)]
pub struct EventBus {
    pending: Vec<TimestampedEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus { pending: Vec::new() }
    }

    /// Publish an event at the current tick.
    pub fn publish(&mut self, tick: Tick, event: SimEvent) {
        self.pending.push(TimestampedEvent { tick, event });
    }

    /// Drain all pending events. Returns them and clears the internal buffer.
    pub fn drain(&mut self) -> Vec<TimestampedEvent> {
        std::mem::take(&mut self.pending)
    }

    /// Number of pending events.
    #[inline]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether the bus has no pending events.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Peek at pending events without draining.
    pub fn peek(&self) -> &[TimestampedEvent] {
        &self.pending
    }

    /// Clear all pending events without returning them.
    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> SimEvent {
        SimEvent::PopulationChanged { old: 100, new: 150 }
    }

    #[test]
    fn publish_and_drain() {
        let mut bus = EventBus::new();
        bus.publish(1, sample_event());
        let events = bus.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, sample_event());
    }

    #[test]
    fn drain_clears_buffer() {
        let mut bus = EventBus::new();
        bus.publish(1, sample_event());
        let _ = bus.drain();
        assert!(bus.is_empty());
        assert_eq!(bus.len(), 0);
    }

    #[test]
    fn multiple_events_accumulate() {
        let mut bus = EventBus::new();
        bus.publish(1, SimEvent::PopulationChanged { old: 0, new: 10 });
        bus.publish(1, SimEvent::HousingShortage { deficit: 5 });
        bus.publish(2, SimEvent::PowerShortage { deficit_kw: 100 });
        assert_eq!(bus.len(), 3);
        let events = bus.drain();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn peek_does_not_consume() {
        let mut bus = EventBus::new();
        bus.publish(1, sample_event());
        let peeked = bus.peek();
        assert_eq!(peeked.len(), 1);
        // Events are still there after peek
        assert_eq!(bus.len(), 1);
        assert!(!bus.is_empty());
    }

    #[test]
    fn empty_after_drain() {
        let mut bus = EventBus::new();
        bus.publish(1, sample_event());
        bus.publish(2, SimEvent::MilestoneReached { milestone_id: 1 });
        let _ = bus.drain();
        assert!(bus.is_empty());
        let events = bus.drain();
        assert!(events.is_empty());
    }

    #[test]
    fn event_equality() {
        let a = SimEvent::BuildingCompleted {
            handle: EntityHandle::new(0, 1),
            archetype: 42,
        };
        let b = SimEvent::BuildingCompleted {
            handle: EntityHandle::new(0, 1),
            archetype: 42,
        };
        let c = SimEvent::BuildingCompleted {
            handle: EntityHandle::new(1, 1),
            archetype: 42,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn timestamped_event_tick_is_correct() {
        let mut bus = EventBus::new();
        bus.publish(42, sample_event());
        bus.publish(99, SimEvent::WaterShortage { deficit: 10 });
        let events = bus.drain();
        assert_eq!(events[0].tick, 42);
        assert_eq!(events[1].tick, 99);
    }
}
