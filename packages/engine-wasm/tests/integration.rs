//! End-to-end integration tests for the simulation engine.
//!
//! Tests verify the full game loop: place buildings -> run ticks -> verify outcomes.
//! These are integration tests that exercise public APIs across multiple modules.

use townbuilder_engine::core::archetypes::{
    ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag,
};
use townbuilder_engine::core::entity::EntityStore;
use townbuilder_engine::core::events::{EventBus, SimEvent};
use townbuilder_engine::core::world::WorldState;
use townbuilder_engine::core_types::*;
use townbuilder_engine::io::save::{deserialize_world, serialize_world};
use townbuilder_engine::math::rng::Rng;
use townbuilder_engine::sim::systems::construction;

// ---- Helpers ----------------------------------------------------------------

/// Create a residential archetype with the given id and build time.
fn test_residential(id: ArchetypeId, build_time: u32) -> ArchetypeDefinition {
    ArchetypeDefinition {
        id,
        name: format!("Test House {}", id),
        tags: vec![ArchetypeTag::Residential],
        footprint_w: 1,
        footprint_h: 1,
        coverage_ratio_pct: 50,
        floors: 2,
        usable_ratio_pct: 80,
        base_cost_cents: 15000,
        base_upkeep_cents_per_tick: 1,
        power_demand_kw: 5,
        power_supply_kw: 0,
        water_demand: 2,
        water_supply: 0,
        water_coverage_radius: 0,
        is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 2,
        desirability_magnitude: 1,
        pollution: 0,
        noise: 1,
        build_time_ticks: build_time,
        max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 0,
        living_space_per_person_m2: 40,
    }
}

/// Create a utility/power-plant archetype with the given id and build time.
fn test_power_plant(id: ArchetypeId, build_time: u32) -> ArchetypeDefinition {
    ArchetypeDefinition {
        id,
        name: format!("Test Power {}", id),
        tags: vec![ArchetypeTag::Utility],
        footprint_w: 2,
        footprint_h: 2,
        coverage_ratio_pct: 70,
        floors: 1,
        usable_ratio_pct: 90,
        base_cost_cents: 100000,
        base_upkeep_cents_per_tick: 10,
        power_demand_kw: 0,
        power_supply_kw: 500,
        water_demand: 5,
        water_supply: 0,
        water_coverage_radius: 0,
        is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 5,
        desirability_magnitude: -3,
        pollution: 20,
        noise: 15,
        build_time_ticks: build_time,
        max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 25,
        living_space_per_person_m2: 0,
    }
}

// ---- Test 1: Construction completes after build_time ticks ------------------

#[test]
fn construction_completes_and_building_becomes_active() {
    let mut entities = EntityStore::new(64);
    let mut registry = ArchetypeRegistry::new();
    let mut events = EventBus::new();

    let build_time = 10u32;
    registry.register(test_residential(1, build_time));
    let handle = entities.alloc(1, 5, 5, 0).unwrap();

    // Entity should start under construction.
    assert!(entities
        .get_flags(handle)
        .unwrap()
        .contains(StatusFlags::UNDER_CONSTRUCTION));
    assert_eq!(entities.get_construction_progress(handle).unwrap(), 0);

    // Run for exactly build_time ticks.
    for tick in 0..build_time as u64 {
        construction::tick_construction(&mut entities, &registry, &mut events, tick);
    }

    // Construction should be complete.
    assert_eq!(
        entities.get_construction_progress(handle).unwrap(),
        0xFFFF
    );
    assert!(
        !entities
            .get_flags(handle)
            .unwrap()
            .contains(StatusFlags::UNDER_CONSTRUCTION)
    );

    // Should have emitted a BuildingCompleted event.
    let drained = events.drain();
    assert!(drained.iter().any(|e| matches!(
        e.event,
        SimEvent::BuildingCompleted {
            archetype: 1,
            ..
        }
    )));
}

// ---- Test 2: Multiple buildings construct independently ---------------------

#[test]
fn multiple_buildings_construct_independently() {
    let mut entities = EntityStore::new(64);
    let mut registry = ArchetypeRegistry::new();
    let mut events = EventBus::new();

    registry.register(test_residential(1, 10)); // 10 tick build
    registry.register(test_power_plant(2, 20)); // 20 tick build

    let h1 = entities.alloc(1, 0, 0, 0).unwrap();
    let h2 = entities.alloc(2, 5, 5, 0).unwrap();

    // Run 10 ticks: h1 should complete, h2 should not.
    for tick in 0..10u64 {
        construction::tick_construction(&mut entities, &registry, &mut events, tick);
    }

    assert_eq!(
        entities.get_construction_progress(h1).unwrap(),
        0xFFFF,
        "h1 (10-tick build) should be complete after 10 ticks"
    );
    assert!(
        !entities
            .get_flags(h1)
            .unwrap()
            .contains(StatusFlags::UNDER_CONSTRUCTION)
    );

    assert!(
        entities.get_construction_progress(h2).unwrap() < 0xFFFF,
        "h2 (20-tick build) should NOT be complete after 10 ticks"
    );
    assert!(entities
        .get_flags(h2)
        .unwrap()
        .contains(StatusFlags::UNDER_CONSTRUCTION));

    // Run 10 more ticks: h2 should now complete.
    for tick in 10..20u64 {
        construction::tick_construction(&mut entities, &registry, &mut events, tick);
    }

    assert_eq!(
        entities.get_construction_progress(h2).unwrap(),
        0xFFFF,
        "h2 should be complete after 20 ticks total"
    );
    assert!(
        !entities
            .get_flags(h2)
            .unwrap()
            .contains(StatusFlags::UNDER_CONSTRUCTION)
    );
}

// ---- Test 3: Deterministic RNG same seed -> same sequence -------------------

#[test]
fn deterministic_rng_produces_same_sequence() {
    let mut rng1 = Rng::new(12345);
    let mut rng2 = Rng::new(12345);

    for i in 0..100 {
        assert_eq!(
            rng1.next_u32(),
            rng2.next_u32(),
            "RNG diverged at iteration {}",
            i
        );
    }
}

// ---- Test 4: Different seeds -> different sequences -------------------------

#[test]
fn different_seeds_produce_different_sequences() {
    let mut rng1 = Rng::new(12345);
    let mut rng2 = Rng::new(54321);

    let mut same_count = 0;
    for _ in 0..100 {
        if rng1.next_u32() == rng2.next_u32() {
            same_count += 1;
        }
    }
    // Extremely unlikely to have more than a few collisions.
    assert!(
        same_count < 5,
        "Too many collisions ({}) between different seeds",
        same_count
    );
}

// ---- Test 5: Construction ticks_remaining decreases correctly ---------------

#[test]
fn construction_ticks_remaining_decreases() {
    let mut entities = EntityStore::new(64);
    let mut registry = ArchetypeRegistry::new();
    let mut events = EventBus::new();

    let build_time = 10u32;
    registry.register(test_residential(1, build_time));
    let handle = entities.alloc(1, 0, 0, 0).unwrap();

    let initial = construction::ticks_remaining(&entities, &registry, handle).unwrap();
    assert_eq!(initial, build_time, "initial ticks_remaining should equal build_time");

    // After 5 ticks, should have ~5 remaining.
    for tick in 0..5u64 {
        construction::tick_construction(&mut entities, &registry, &mut events, tick);
    }

    let remaining = construction::ticks_remaining(&entities, &registry, handle).unwrap();
    assert_eq!(
        remaining, 5,
        "After 5 of 10 ticks, 5 should remain"
    );

    // After all ticks complete, remaining should be 0.
    for tick in 5..10u64 {
        construction::tick_construction(&mut entities, &registry, &mut events, tick);
    }

    let final_remaining = construction::ticks_remaining(&entities, &registry, handle).unwrap();
    assert_eq!(final_remaining, 0, "After completion, ticks_remaining should be 0");
}

// ---- Test 6: Entity generation prevents use-after-free ----------------------

#[test]
fn entity_store_generation_prevents_use_after_free() {
    let mut entities = EntityStore::new(64);
    let handle = entities.alloc(1, 0, 0, 0).unwrap();

    // Verify handle is valid.
    assert!(entities.is_valid(handle));
    assert_eq!(entities.get_archetype(handle), Some(1));

    // Free the entity.
    entities.free(handle);

    // Old handle should now be invalid.
    assert!(!entities.is_valid(handle));
    assert!(entities.get_archetype(handle).is_none());
    assert!(entities.get_pos(handle).is_none());
    assert!(entities.get_flags(handle).is_none());
    assert!(entities.get_construction_progress(handle).is_none());

    // Allocate into the same slot -- generation should differ.
    let new_handle = entities.alloc(2, 1, 1, 0).unwrap();
    assert_eq!(new_handle.index, handle.index, "Should reuse same slot");
    assert_ne!(
        new_handle.generation, handle.generation,
        "Generation should have incremented"
    );

    // Old handle still invalid, new handle valid.
    assert!(!entities.is_valid(handle));
    assert!(entities.is_valid(new_handle));
    assert_eq!(entities.get_archetype(new_handle), Some(2));
}

// ---- Test 7: Save/load round-trip preserves state ---------------------------

#[test]
fn save_and_load_round_trip_preserves_state() {
    let size = MapSize::new(16, 16);
    let mut world = WorldState::new(size, 42);
    world.city_name = "Integration City".to_string();
    world.tick = 5000;
    world.treasury = 1_000_000;

    // Place some entities.
    let h1 = world.place_entity(10, 3, 4, 1).unwrap();
    let h2 = world.place_entity(20, 7, 8, 2).unwrap();

    // Customize entity properties.
    world
        .entities
        .set_flags(h1, StatusFlags::POWERED | StatusFlags::STAFFED);
    world.entities.set_construction_progress(h1, 0xFFFF);
    world.entities.set_level(h1, 3);

    world.entities.set_flags(h2, StatusFlags::UNDER_CONSTRUCTION);
    world.entities.set_construction_progress(h2, 0x4000);
    world.entities.set_enabled(h2, false);

    // Serialize.
    let bytes = serialize_world(&world);

    // Deserialize.
    let loaded = deserialize_world(&bytes).expect("deserialization should succeed");

    // Verify world-level properties.
    assert_eq!(loaded.map_size().width, size.width);
    assert_eq!(loaded.map_size().height, size.height);
    assert_eq!(loaded.city_name, "Integration City");
    assert_eq!(loaded.tick, 5000);
    assert_eq!(loaded.treasury, 1_000_000);
    assert_eq!(loaded.seeds.global_seed, 42);

    // Verify entity count.
    assert_eq!(loaded.entities.count(), 2);

    // Verify entities by archetype.
    let alive: Vec<EntityHandle> = loaded.entities.iter_alive().collect();
    assert_eq!(alive.len(), 2);

    let e1 = alive
        .iter()
        .find(|h| loaded.entities.get_archetype(**h) == Some(10))
        .expect("Should find entity with archetype 10");
    assert_eq!(loaded.entities.get_pos(*e1), Some(TileCoord::new(3, 4)));
    assert_eq!(loaded.entities.get_level(*e1), Some(3));
    assert_eq!(
        loaded.entities.get_construction_progress(*e1),
        Some(0xFFFF)
    );
    let e1_flags = loaded.entities.get_flags(*e1).unwrap();
    assert!(e1_flags.contains(StatusFlags::POWERED));
    assert!(e1_flags.contains(StatusFlags::STAFFED));

    let e2 = alive
        .iter()
        .find(|h| loaded.entities.get_archetype(**h) == Some(20))
        .expect("Should find entity with archetype 20");
    assert_eq!(loaded.entities.get_pos(*e2), Some(TileCoord::new(7, 8)));
    assert_eq!(
        loaded.entities.get_construction_progress(*e2),
        Some(0x4000)
    );
    assert_eq!(loaded.entities.get_enabled(*e2), Some(false));
}

// ---- Test 8: Full game loop -- place, construct, verify ---------------------

#[test]
fn full_game_loop_place_construct_verify() {
    // This test exercises the full flow: create world, register archetypes,
    // place a building via WorldState, run construction ticks, verify completion.

    let mut world = WorldState::new(MapSize::new(32, 32), 99);
    let mut registry = ArchetypeRegistry::new();
    let mut events = EventBus::new();

    let build_time = 5u32;
    registry.register(test_residential(1, build_time));

    // Place building through WorldState API.
    let handle = world.place_entity(1, 10, 10, 0).unwrap();
    assert_eq!(world.entities.count(), 1);

    // Run construction ticks using world's entity store.
    for tick in 0..build_time as u64 {
        construction::tick_construction(&mut world.entities, &registry, &mut events, tick);
    }

    // Building should be complete.
    assert_eq!(
        world.entities.get_construction_progress(handle).unwrap(),
        0xFFFF
    );
    assert!(
        !world
            .entities
            .get_flags(handle)
            .unwrap()
            .contains(StatusFlags::UNDER_CONSTRUCTION)
    );

    // Event should have been emitted.
    let drained = events.drain();
    let completed_events: Vec<_> = drained
        .iter()
        .filter(|e| matches!(e.event, SimEvent::BuildingCompleted { .. }))
        .collect();
    assert_eq!(completed_events.len(), 1);

    // Remove the building.
    assert!(world.remove_entity(handle));
    assert_eq!(world.entities.count(), 0);
    assert!(!world.entities.is_valid(handle));
}

// ---- Test 9: RNG fork determinism across systems ----------------------------

#[test]
fn rng_fork_produces_deterministic_child_sequences() {
    let rng = Rng::new(42);

    // Fork the same key twice -- should produce identical sequences.
    let mut child_a = rng.fork("construction");
    let mut child_b = rng.fork("construction");

    let seq_a: Vec<u32> = (0..50).map(|_| child_a.next_u32()).collect();
    let seq_b: Vec<u32> = (0..50).map(|_| child_b.next_u32()).collect();
    assert_eq!(seq_a, seq_b, "Same fork key must produce identical sequences");

    // Fork a different key -- should produce a different sequence.
    let mut child_c = rng.fork("population");
    let seq_c: Vec<u32> = (0..50).map(|_| child_c.next_u32()).collect();
    assert_ne!(
        seq_a, seq_c,
        "Different fork keys should produce different sequences"
    );
}

// ---- Test 10: WorldState buildability checks --------------------------------

#[test]
fn world_buildability_and_entity_placement() {
    let mut world = WorldState::new(MapSize::new(16, 16), 1);

    // Default tiles (grass) should be buildable.
    assert!(world.is_buildable(0, 0));
    assert!(world.is_buildable(15, 15));

    // Out of bounds should not be buildable.
    assert!(!world.is_buildable(16, 0));
    assert!(!world.is_buildable(-1, 0));

    // Water tiles should not be buildable.
    world.tiles.set_terrain(5, 5, TerrainType::Water);
    assert!(!world.is_buildable(5, 5));

    // Placing entity on valid tile should succeed.
    let h = world.place_entity(1, 3, 3, 0);
    assert!(h.is_some());

    // Placing entity out of bounds should fail.
    let h_oob = world.place_entity(1, 16, 0, 0);
    assert!(h_oob.is_none());
}
