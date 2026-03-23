use city_core::{ArchetypeId, EntityHandle, Tick, TileCoord};
use crate::types::MoneyCents;
use serde::{Deserialize, Serialize};

/// Utility type for shortage/restoration events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum UtilityType {
    Power      = 0,
    Water      = 1,
    HealthCare = 2,
}

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
    UtilityShortage { kind: String, deficit: u32 },
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

    pub fn publish(&mut self, tick: Tick, event: SimEvent) {
        self.pending.push(TimestampedEvent { tick, event });
    }

    pub fn drain(&mut self) -> Vec<TimestampedEvent> {
        std::mem::take(&mut self.pending)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn peek(&self) -> &[TimestampedEvent] {
        &self.pending
    }

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
        bus.publish(2, SimEvent::UtilityShortage { kind: "electricity".into(), deficit: 100 });
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
        bus.publish(99, SimEvent::UtilityShortage { kind: "water".into(), deficit: 10 });
        let events = bus.drain();
        assert_eq!(events[0].tick, 42);
        assert_eq!(events[1].tick, 99);
    }

    #[test]
    fn clear_removes_all() {
        let mut bus = EventBus::new();
        bus.publish(1, sample_event());
        bus.clear();
        assert!(bus.is_empty());
    }
}
