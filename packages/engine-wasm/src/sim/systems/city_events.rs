//! City events system: fires, crime waves, and disasters.
//!
//! Each tick, evaluates random chances for fires to start on active entities,
//! manages fire spread and extinguishment, and triggers periodic crime waves.
//! Fire probability is influenced by the city's fire budget; crime wave
//! probability is influenced by the police budget.

use crate::core::entity::EntityStore;
use crate::core::events::{EventBus, SimEvent};
use crate::core::world::CityPolicies;
use crate::core_types::*;
use crate::math::rng::Rng;

// ---- ActiveFire ----

/// An active fire burning on an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFire {
    /// The entity that is on fire.
    pub entity: EntityHandle,
    /// The tile location of the fire.
    pub location: TileCoord,
    /// Number of ticks remaining before the fire burns out.
    pub remaining_ticks: u32,
}

// ---- CityEventState ----

/// Persistent state for the city events system, tracked across ticks.
#[derive(Debug)]
pub struct CityEventState {
    /// Currently active fires.
    pub fires: Vec<ActiveFire>,
}

impl CityEventState {
    /// Create a new empty city event state.
    pub fn new() -> Self {
        CityEventState { fires: Vec::new() }
    }

    /// Add a new active fire.
    pub fn add_fire(&mut self, fire: ActiveFire) {
        self.fires.push(fire);
    }

    /// Decrement remaining_ticks on all active fires.
    pub fn tick_fires(&mut self) {
        for fire in &mut self.fires {
            fire.remaining_ticks = fire.remaining_ticks.saturating_sub(1);
        }
    }

    /// Remove fires whose remaining_ticks have reached zero. Returns removed fires.
    pub fn remove_expired(&mut self) -> Vec<ActiveFire> {
        let mut expired = Vec::new();
        self.fires.retain(|fire| {
            if fire.remaining_ticks == 0 {
                expired.push(fire.clone());
                false
            } else {
                true
            }
        });
        expired
    }
}

// ---- Constants ----

/// Default fire duration in ticks.
const FIRE_DURATION_TICKS: u32 = 200;

/// Base fire probability in Q0.16 (1 out of 65535).
const BASE_FIRE_PROB_Q16: u16 = 1;

/// Fire spread probability: 1/1000 chance per fire per tick.
/// In Q0.16: 65535 / 1000 ~ 65.
const FIRE_SPREAD_PROB_Q16: u16 = 65;

/// Maximum manhattan distance for fire spread.
const FIRE_SPREAD_MAX_DISTANCE: u32 = 2;

/// Crime wave check interval in ticks.
const CRIME_WAVE_INTERVAL: u64 = 10_000;

/// Base crime wave probability in Q0.16 when police budget is 0.
const BASE_CRIME_PROB_Q16: u16 = 6553; // ~10% chance

// ---- tick_city_events ----

/// Run city events for one tick.
///
/// Checks for new fires on active entities, spreads existing fires,
/// extinguishes fires whose duration has elapsed, and periodically
/// triggers crime waves.
pub fn tick_city_events(
    state: &mut CityEventState,
    entities: &mut EntityStore,
    events: &mut EventBus,
    rng: &mut Rng,
    tick: Tick,
    policies: &CityPolicies,
) {
    // --- Fire ignition: check each active entity for random fire start ---
    let fire_prob = compute_fire_probability(policies);

    // Collect candidate entities first to avoid borrow conflict.
    let candidates: Vec<(EntityHandle, TileCoord)> = entities
        .iter_alive()
        .filter_map(|handle| {
            let flags = entities.get_flags(handle)?;
            let enabled = entities.get_enabled(handle)?;
            if !enabled {
                return None;
            }
            // Skip entities already on fire.
            if flags.contains(StatusFlags::ON_FIRE) {
                return None;
            }
            let pos = entities.get_pos(handle)?;
            Some((handle, pos))
        })
        .collect();

    for (handle, pos) in &candidates {
        if rng.chance(fire_prob) {
            // Start a fire on this entity.
            let current_flags = entities.get_flags(*handle).unwrap_or(StatusFlags::NONE);
            entities.set_flags(*handle, current_flags.insert(StatusFlags::ON_FIRE));
            state.add_fire(ActiveFire {
                entity: *handle,
                location: *pos,
                remaining_ticks: FIRE_DURATION_TICKS,
            });
            events.publish(
                tick,
                SimEvent::FireStarted {
                    location: *pos,
                    entity: *handle,
                },
            );
        }
    }

    // --- Fire spread ---
    let mut new_fires: Vec<(EntityHandle, TileCoord)> = Vec::new();

    for fire in &state.fires {
        if rng.chance(FIRE_SPREAD_PROB_Q16) {
            // Find an adjacent entity within manhattan distance 2 that is not on fire.
            for (handle, pos) in &candidates {
                if pos.manhattan_distance(&fire.location) <= FIRE_SPREAD_MAX_DISTANCE
                    && !entities
                        .get_flags(*handle)
                        .unwrap_or(StatusFlags::NONE)
                        .contains(StatusFlags::ON_FIRE)
                {
                    new_fires.push((*handle, *pos));
                    events.publish(
                        tick,
                        SimEvent::FireSpread {
                            from: fire.location,
                            to: *pos,
                        },
                    );
                    break; // Only spread to one entity per fire per tick.
                }
            }
        }
    }

    for (handle, pos) in new_fires {
        let current_flags = entities.get_flags(handle).unwrap_or(StatusFlags::NONE);
        entities.set_flags(handle, current_flags.insert(StatusFlags::ON_FIRE));
        state.add_fire(ActiveFire {
            entity: handle,
            location: pos,
            remaining_ticks: FIRE_DURATION_TICKS,
        });
    }

    // --- Tick down fire durations ---
    state.tick_fires();

    // --- Extinguish expired fires ---
    let expired = state.remove_expired();
    for fire in &expired {
        if entities.is_valid(fire.entity) {
            let current_flags = entities.get_flags(fire.entity).unwrap_or(StatusFlags::NONE);
            // Clear ON_FIRE, set DAMAGED.
            let new_flags = current_flags
                .remove(StatusFlags::ON_FIRE)
                .insert(StatusFlags::DAMAGED);
            entities.set_flags(fire.entity, new_flags);
        }
        events.publish(
            tick,
            SimEvent::FireExtinguished {
                location: fire.location,
            },
        );
    }

    // --- Crime waves: check every CRIME_WAVE_INTERVAL ticks ---
    if tick > 0 && tick % CRIME_WAVE_INTERVAL == 0 {
        let crime_prob = compute_crime_probability(policies);
        if rng.chance(crime_prob) {
            let district = rng.next_bounded(256) as u16;
            let severity = rng.range_inclusive(1, 5) as u8;
            events.publish(
                tick,
                SimEvent::CrimeWave {
                    district,
                    severity,
                },
            );
        }
    }
}

/// Compute effective fire probability based on fire budget.
///
/// Higher fire_budget_pct reduces the probability.
/// At fire_budget_pct=0, probability = BASE_FIRE_PROB_Q16.
/// At fire_budget_pct=200 (max), probability = 0.
fn compute_fire_probability(policies: &CityPolicies) -> u16 {
    let budget = policies.fire_budget_pct as u32;
    // Scale down: effective_prob = base * (200 - budget) / 200
    let effective = (BASE_FIRE_PROB_Q16 as u32)
        .saturating_mul(200u32.saturating_sub(budget))
        / 200;
    effective as u16
}

/// Compute effective crime wave probability based on police budget.
///
/// Higher police_budget_pct reduces the probability.
/// At police_budget_pct=0, probability = BASE_CRIME_PROB_Q16.
/// At police_budget_pct=200 (max), probability = 0.
fn compute_crime_probability(policies: &CityPolicies) -> u16 {
    let budget = policies.police_budget_pct as u32;
    let effective = (BASE_CRIME_PROB_Q16 as u32)
        .saturating_mul(200u32.saturating_sub(budget))
        / 200;
    effective as u16
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create test fixtures.
    fn make_test_fixtures() -> (CityEventState, EntityStore, EventBus, Rng, CityPolicies) {
        let state = CityEventState::new();
        let entities = EntityStore::new(64);
        let events = EventBus::new();
        let rng = Rng::new(42);
        let policies = CityPolicies::default();
        (state, entities, events, rng, policies)
    }

    /// Helper: allocate an entity and clear UNDER_CONSTRUCTION so it's active.
    fn spawn_active_entity(entities: &mut EntityStore, x: i16, y: i16) -> EntityHandle {
        let h = entities.alloc(1, x, y, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        h
    }

    // ---- Test 1: New state is empty ----

    #[test]
    fn new_state_is_empty() {
        let state = CityEventState::new();
        assert!(state.fires.is_empty());
    }

    // ---- Test 2: Fire starts on random chance ----

    #[test]
    fn fire_starts_on_random_chance() {
        let mut state = CityEventState::new();
        let mut entities = EntityStore::new(256);
        let mut events = EventBus::new();

        // Spawn many entities to increase chance of fire.
        let mut handles = Vec::new();
        for i in 0..100 {
            handles.push(spawn_active_entity(&mut entities, i, 0));
        }

        // Use a high fire probability by setting fire_budget_pct to 0.
        let mut low_budget = CityPolicies::default();
        low_budget.fire_budget_pct = 0;

        // Run many ticks with a fresh RNG until a fire starts.
        let mut rng = Rng::new(12345);
        let mut fire_started = false;
        for tick in 1..=500 {
            tick_city_events(&mut state, &mut entities, &mut events, &mut rng, tick, &low_budget);
            if !state.fires.is_empty() {
                fire_started = true;
                break;
            }
        }
        assert!(fire_started, "Expected a fire to start within 500 ticks");
    }

    // ---- Test 3: Fire decrements remaining ticks ----

    #[test]
    fn fire_decrements_remaining_ticks() {
        let mut state = CityEventState::new();
        state.add_fire(ActiveFire {
            entity: EntityHandle::new(0, 0),
            location: TileCoord::new(5, 5),
            remaining_ticks: 10,
        });

        state.tick_fires();
        assert_eq!(state.fires[0].remaining_ticks, 9);

        state.tick_fires();
        assert_eq!(state.fires[0].remaining_ticks, 8);
    }

    // ---- Test 4: Fire extinguishes when remaining=0 ----

    #[test]
    fn fire_extinguishes_when_remaining_zero() {
        let mut state = CityEventState::new();
        state.add_fire(ActiveFire {
            entity: EntityHandle::new(0, 0),
            location: TileCoord::new(5, 5),
            remaining_ticks: 1,
        });

        state.tick_fires(); // remaining goes to 0
        let expired = state.remove_expired();
        assert_eq!(expired.len(), 1);
        assert!(state.fires.is_empty());
    }

    // ---- Test 5: ON_FIRE flag set correctly ----

    #[test]
    fn on_fire_flag_set_when_fire_starts() {
        let (mut state, mut entities, mut events, _, _) = make_test_fixtures();
        let h = spawn_active_entity(&mut entities, 5, 5);
        let pos = entities.get_pos(h).unwrap();

        // Manually start a fire via tick_city_events machinery:
        let current = entities.get_flags(h).unwrap();
        entities.set_flags(h, current.insert(StatusFlags::ON_FIRE));
        state.add_fire(ActiveFire {
            entity: h,
            location: pos,
            remaining_ticks: FIRE_DURATION_TICKS,
        });
        events.publish(
            1,
            SimEvent::FireStarted {
                location: pos,
                entity: h,
            },
        );

        let flags = entities.get_flags(h).unwrap();
        assert!(flags.contains(StatusFlags::ON_FIRE));
    }

    // ---- Test 6: ON_FIRE flag cleared and DAMAGED set after extinguish ----

    #[test]
    fn on_fire_cleared_damaged_set_after_extinguish() {
        let (mut state, mut entities, mut events, mut rng, policies) = make_test_fixtures();
        let h = spawn_active_entity(&mut entities, 5, 5);

        // Set the entity on fire manually.
        entities.set_flags(h, StatusFlags::ON_FIRE);
        state.add_fire(ActiveFire {
            entity: h,
            location: TileCoord::new(5, 5),
            remaining_ticks: 1,
        });

        // Tick once: fire will expire (remaining goes from 1 to 0).
        tick_city_events(&mut state, &mut entities, &mut events, &mut rng, 1, &policies);

        let flags = entities.get_flags(h).unwrap();
        assert!(
            !flags.contains(StatusFlags::ON_FIRE),
            "ON_FIRE should be cleared"
        );
        assert!(
            flags.contains(StatusFlags::DAMAGED),
            "DAMAGED should be set"
        );
    }

    // ---- Test 7: FireStarted event emitted ----

    #[test]
    fn fire_started_event_emitted() {
        let (mut state, mut entities, mut events, _, _) = make_test_fixtures();
        let h = spawn_active_entity(&mut entities, 3, 4);
        let pos = TileCoord::new(3, 4);

        // Manually simulate fire start.
        entities.set_flags(h, StatusFlags::ON_FIRE);
        state.add_fire(ActiveFire {
            entity: h,
            location: pos,
            remaining_ticks: FIRE_DURATION_TICKS,
        });
        events.publish(1, SimEvent::FireStarted { location: pos, entity: h });

        let drained = events.drain();
        assert!(drained.iter().any(|e| matches!(
            &e.event,
            SimEvent::FireStarted { location, entity }
            if *location == pos && *entity == h
        )));
    }

    // ---- Test 8: FireExtinguished event emitted ----

    #[test]
    fn fire_extinguished_event_emitted() {
        let (mut state, mut entities, mut events, mut rng, policies) = make_test_fixtures();
        let h = spawn_active_entity(&mut entities, 7, 7);

        entities.set_flags(h, StatusFlags::ON_FIRE);
        state.add_fire(ActiveFire {
            entity: h,
            location: TileCoord::new(7, 7),
            remaining_ticks: 1,
        });

        tick_city_events(&mut state, &mut entities, &mut events, &mut rng, 1, &policies);

        let drained = events.drain();
        assert!(drained.iter().any(|e| matches!(
            &e.event,
            SimEvent::FireExtinguished { location }
            if *location == TileCoord::new(7, 7)
        )));
    }

    // ---- Test 9: Fire spread mechanics ----

    #[test]
    fn fire_spread_mechanics() {
        let (mut state, mut entities, mut events, _, policies) = make_test_fixtures();

        // Place two entities close together.
        let h1 = spawn_active_entity(&mut entities, 5, 5);
        let h2 = spawn_active_entity(&mut entities, 6, 5);

        // Start fire on h1.
        entities.set_flags(h1, StatusFlags::ON_FIRE);
        state.add_fire(ActiveFire {
            entity: h1,
            location: TileCoord::new(5, 5),
            remaining_ticks: 100,
        });

        // Run many ticks with deterministic RNG to try to get fire to spread.
        let mut rng = Rng::new(77777);
        let mut spread = false;
        for tick in 1..=2000 {
            tick_city_events(&mut state, &mut entities, &mut events, &mut rng, tick, &policies);
            let flags2 = entities.get_flags(h2).unwrap_or(StatusFlags::NONE);
            if flags2.contains(StatusFlags::ON_FIRE) {
                spread = true;
                break;
            }
        }
        assert!(spread, "Fire should eventually spread to adjacent entity");
    }

    // ---- Test 10: Crime wave trigger ----

    #[test]
    fn crime_wave_trigger() {
        let (mut state, mut entities, mut events, _, _) = make_test_fixtures();

        // Set police budget to 0 for highest crime probability.
        let mut policies = CityPolicies::default();
        policies.police_budget_pct = 0;

        // Run at crime wave interval ticks many times to trigger a crime wave.
        let mut rng = Rng::new(42);
        let mut crime_triggered = false;
        for i in 1..=100 {
            let tick = i * CRIME_WAVE_INTERVAL;
            tick_city_events(&mut state, &mut entities, &mut events, &mut rng, tick, &policies);
            let drained = events.drain();
            if drained.iter().any(|e| matches!(&e.event, SimEvent::CrimeWave { .. })) {
                crime_triggered = true;
                break;
            }
        }
        assert!(
            crime_triggered,
            "Expected crime wave to trigger within 100 intervals"
        );
    }

    // ---- Test 11: Empty world no panic ----

    #[test]
    fn empty_world_no_panic() {
        let (mut state, mut entities, mut events, mut rng, policies) = make_test_fixtures();

        // Run several ticks on an empty world; should not panic.
        for tick in 0..100 {
            tick_city_events(&mut state, &mut entities, &mut events, &mut rng, tick, &policies);
        }
        assert!(state.fires.is_empty());
    }

    // ---- Test 12: High fire budget reduces fire chance ----

    #[test]
    fn high_fire_budget_reduces_fire_chance() {
        // With fire_budget_pct = 200, fire probability should be 0.
        let mut policies = CityPolicies::default();
        policies.fire_budget_pct = 200;
        let prob = compute_fire_probability(&policies);
        assert_eq!(prob, 0, "Fire probability should be 0 at max budget");

        // With fire_budget_pct = 0, fire probability should be at base level.
        policies.fire_budget_pct = 0;
        let prob_zero = compute_fire_probability(&policies);
        assert_eq!(
            prob_zero, BASE_FIRE_PROB_Q16,
            "Fire probability should be at base level with 0 budget"
        );

        // With fire_budget_pct = 100 (default), should be half of base.
        policies.fire_budget_pct = 100;
        let prob_default = compute_fire_probability(&policies);
        assert!(
            prob_default < prob_zero,
            "Default budget should produce lower fire probability than 0 budget"
        );
    }

    // ---- Test 13: add_fire increases fire count ----

    #[test]
    fn add_fire_increases_count() {
        let mut state = CityEventState::new();
        assert_eq!(state.fires.len(), 0);

        state.add_fire(ActiveFire {
            entity: EntityHandle::new(0, 0),
            location: TileCoord::new(1, 1),
            remaining_ticks: 50,
        });
        assert_eq!(state.fires.len(), 1);

        state.add_fire(ActiveFire {
            entity: EntityHandle::new(1, 0),
            location: TileCoord::new(2, 2),
            remaining_ticks: 100,
        });
        assert_eq!(state.fires.len(), 2);
    }

    // ---- Test 14: remove_expired only removes zero-tick fires ----

    #[test]
    fn remove_expired_only_removes_zero() {
        let mut state = CityEventState::new();
        state.add_fire(ActiveFire {
            entity: EntityHandle::new(0, 0),
            location: TileCoord::new(1, 1),
            remaining_ticks: 0,
        });
        state.add_fire(ActiveFire {
            entity: EntityHandle::new(1, 0),
            location: TileCoord::new(2, 2),
            remaining_ticks: 5,
        });
        state.add_fire(ActiveFire {
            entity: EntityHandle::new(2, 0),
            location: TileCoord::new(3, 3),
            remaining_ticks: 0,
        });

        let expired = state.remove_expired();
        assert_eq!(expired.len(), 2);
        assert_eq!(state.fires.len(), 1);
        assert_eq!(state.fires[0].remaining_ticks, 5);
    }

    // ---- Test 15: compute_crime_probability ----

    #[test]
    fn compute_crime_probability_scales_with_budget() {
        let mut policies = CityPolicies::default();

        // Max police budget: no crime.
        policies.police_budget_pct = 200;
        assert_eq!(compute_crime_probability(&policies), 0);

        // Zero police budget: full crime chance.
        policies.police_budget_pct = 0;
        assert_eq!(compute_crime_probability(&policies), BASE_CRIME_PROB_Q16);

        // Default (100): half chance.
        policies.police_budget_pct = 100;
        let half = compute_crime_probability(&policies);
        assert!(half > 0 && half < BASE_CRIME_PROB_Q16);
    }

    // ---- Test 16: Disabled entities do not catch fire ----

    #[test]
    fn disabled_entities_do_not_catch_fire() {
        let (mut state, mut entities, mut events, _, _) = make_test_fixtures();

        // Create a disabled entity.
        let h = entities.alloc(1, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        entities.set_enabled(h, false);

        // Use zero fire budget for maximum fire chance.
        let mut policies = CityPolicies::default();
        policies.fire_budget_pct = 0;

        let mut rng = Rng::new(42);
        for tick in 1..=500 {
            tick_city_events(&mut state, &mut entities, &mut events, &mut rng, tick, &policies);
        }

        let flags = entities.get_flags(h).unwrap();
        assert!(
            !flags.contains(StatusFlags::ON_FIRE),
            "Disabled entities should not catch fire"
        );
    }
}
