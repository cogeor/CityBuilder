use townbuilder_engine::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use townbuilder_engine::core::commands::{Command, PolicyKey};
use townbuilder_engine::core::network::RoadGraph;
use townbuilder_engine::core::world::WorldState;
use townbuilder_engine::core_types::MapSize;
use townbuilder_engine::sim::tick::SimulationEngine;

fn make_residential(id: u16, build_time: u32) -> ArchetypeDefinition {
    ArchetypeDefinition {
        id,
        name: format!("House {}", id),
        tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
        footprint_w: 1,
        footprint_h: 1,
        coverage_ratio_pct: 50,
        floors: 2,
        usable_ratio_pct: 80,
        base_cost_cents: 100_000,
        base_upkeep_cents_per_tick: 10,
        power_demand_kw: 5,
        power_supply_kw: 0,
        water_demand: 2,
        water_supply: 0,
        service_radius: 0,
        desirability_radius: 2,
        desirability_magnitude: 5,
        pollution: 0,
        noise: 1,
        build_time_ticks: build_time,
        max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 0,
        living_space_per_person_m2: 40,
    }
}

#[test]
fn queued_commands_apply_before_systems_and_emit_outputs() {
    let world = WorldState::new(MapSize::new(32, 32), 42);
    let mut registry = ArchetypeRegistry::new();
    registry.register(make_residential(1, 1));
    let mut engine = SimulationEngine::new(world, registry, RoadGraph::new());

    engine.queue_command(Command::PlaceEntity {
        archetype_id: 1,
        x: 4,
        y: 4,
        rotation: 0,
    });
    engine.queue_command(Command::SetPolicy {
        key: PolicyKey::ResidentialTax,
        value: 11,
    });

    let out = engine.tick();
    assert_eq!(out.tick, 1);
    assert!(!out.world_diff.metrics.is_empty());
    assert!(!out.world_diff.diffs.is_empty());
}
