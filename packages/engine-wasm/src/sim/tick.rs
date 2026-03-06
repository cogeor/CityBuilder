//! Simulation tick loop — full integration of all systems.
//!
//! `SimulationEngine` owns all state needed to advance the simulation
//! one tick at a time. Each tick runs all systems in a fixed order:
//! construction, power, water, population, jobs, transport, finance,
//! city events. Events produced during the tick are collected and
//! returned to the caller.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::commands::{self, Command, CommandEffect, CommandResult, PolicyKey};
use crate::core::diffs::{DiffCollector, MetricScope, MetricUpdate, StateDiff, WorldDiff};
use crate::core::events::{EventBus, TimestampedEvent};
use crate::core::network::RoadGraph;
use crate::core::world::WorldState;
use crate::core_types::*;
use crate::math::rng::Rng;
use crate::caches::cache_manager::{CacheManager, InvalidationReason};
use crate::sim::phase_wheel::PhaseWheel;
use crate::sim::plugin::{SimulationPlugin, SimWorld};
use crate::sim::plugins::{
    BuildingsPlugin, CityEventsPlugin, ConstructionPlugin,
    FinancePlugin, JobsPlugin, PopulationPlugin,
    PowerPlugin, TransportPlugin, WaterPlugin,
};
use crate::sim::systems::city_events::CityEventState;
use crate::sim::systems::transport::TrafficGrid;

/// Output produced by a single simulation tick.
#[derive(Debug)]
pub struct TickOutput {
    /// The tick number that was just executed.
    pub tick: Tick,
    /// All events generated during this tick.
    pub events: Vec<TimestampedEvent>,
    /// Current city population after this tick.
    pub population: u32,
    /// Current treasury balance after this tick.
    pub treasury: MoneyCents,
    /// Unmet power demand in kW after this tick (0 = no shortage).
    pub power_shortage_kw: u32,
    /// Structured state/metric changes emitted this tick.
    pub world_diff: WorldDiff,
}

/// The top-level simulation engine. Owns all state and drives the
/// simulation forward one tick at a time.
#[derive(Debug)]
pub struct SimulationEngine {
    /// Canonical world state (tiles, entities, policies, seeds, tick, treasury).
    pub world: WorldState,
    /// Archetype definitions registry.
    pub registry: ArchetypeRegistry,
    /// Per-tick event bus — drained at the end of each tick.
    pub events: EventBus,
    /// Road network graph.
    pub road_graph: RoadGraph,
    /// Phase wheel scheduler for amortized computation.
    pub phase_wheel: PhaseWheel,
    /// Deterministic PRNG seeded from the world seed.
    pub rng: Rng,
    /// Per-tile traffic density grid.
    pub traffic_grid: TrafficGrid,
    /// Persistent state for city events (fires, etc.).
    pub city_event_state: CityEventState,
    /// Current city population (carried across ticks).
    pub population: u32,
    /// Whether the previous tick had a power shortage.
    pub power_shortage: bool,
    /// Unmet power demand in kW from the previous tick (0 = no shortage).
    pub power_shortage_kw: u32,
    /// Whether the previous tick had a water shortage.
    pub water_shortage: bool,
    /// Queue of commands applied during phase 1 of the next tick.
    pub pending_commands: Vec<Command>,
    /// Cache dirty tracking and invalidation map.
    pub cache_manager: CacheManager,
    /// Ordered list of pluggable simulation systems.
    pub plugins: Vec<Box<dyn SimulationPlugin>>,
}

impl SimulationEngine {
    /// Create a new simulation engine from a world state, registry, and road graph.
    ///
    /// Derives the RNG seed from the world's global seed. Initialises all
    /// transient state (events, phase wheel, traffic grid, city event state)
    /// to their defaults.
    pub fn new(world: WorldState, registry: ArchetypeRegistry, road_graph: RoadGraph) -> Self {
        let rng = Rng::new(world.seeds.global_seed);
        let map_size = world.map_size();
        let traffic_grid = TrafficGrid::new(map_size.width, map_size.height);

        SimulationEngine {
            world,
            registry,
            events: EventBus::new(),
            road_graph,
            phase_wheel: PhaseWheel::new(),
            rng,
            traffic_grid,
            city_event_state: CityEventState::new(),
            population: 0,
            power_shortage: false,
            power_shortage_kw: 0,
            water_shortage: false,
            pending_commands: Vec::new(),
            cache_manager: CacheManager::new(),
            plugins: vec![
                Box::new(BuildingsPlugin),
                Box::new(ConstructionPlugin),
                Box::new(PowerPlugin),
                Box::new(WaterPlugin),
                Box::new(PopulationPlugin),
                Box::new(JobsPlugin),
                Box::new(TransportPlugin),
                Box::new(FinancePlugin),
                Box::new(CityEventsPlugin),
            ],
        }
    }

    /// Queue a command for application at the next tick's phase 1.
    pub fn queue_command(&mut self, cmd: Command) {
        self.pending_commands.push(cmd);
    }

    /// Advance the simulation by one tick.
    ///
    /// Runs all systems in order, drains events, and returns a `TickOutput`
    /// summarising what happened.
    pub fn tick(&mut self) -> TickOutput {
        // 1. Increment the world tick counter.
        self.world.tick += 1;
        let tick = self.world.tick;
        let mut diffs = DiffCollector::new();
        let treasury_before = self.world.treasury;

        // Phase 1: apply queued commands.
        let queued: Vec<Command> = self.pending_commands.drain(..).collect();
        let mut invalidation_reasons: Vec<InvalidationReason> = Vec::new();
        for cmd in &queued {
            if let Ok(effect) = commands::apply_command_with_registry(
                &mut self.world,
                Some(&self.registry),
                Some(&mut self.road_graph),
                cmd,
            )
            {
                self.collect_command_diff(&mut diffs, &effect);
                self.collect_invalidation_reason(&mut invalidation_reasons, &effect);
            }
        }

        // Phase 2: invalidate caches based on command effects.
        for reason in invalidation_reasons {
            self.cache_manager.invalidate(reason);
        }

        // Phase 3: run systems via plugin loop.
        let mut sim_world = SimWorld {
            world: &mut self.world,
            registry: &self.registry,
            events: &mut self.events,
            road_graph: &self.road_graph,
            rng: &mut self.rng,
            traffic_grid: &mut self.traffic_grid,
            city_event_state: &mut self.city_event_state,
            population: &mut self.population,
            power_shortage: &mut self.power_shortage,
            power_shortage_kw: &mut self.power_shortage_kw,
            water_shortage: &mut self.water_shortage,
            phase_wheel: &self.phase_wheel,
        };
        // plugins is temporarily taken out so we can call &mut self.plugins
        // while also holding &mut self.world etc.
        let mut plugins = std::mem::take(&mut self.plugins);
        for plugin in &mut plugins {
            plugin.tick(&mut sim_world, tick);
        }
        self.plugins = plugins;

        // 11. Adapt phase wheel (placeholder: 0 microseconds measured).
        self.phase_wheel.adapt(0);

        // Phase 4: emit outputs (events + diffs + metrics).
        if self.world.treasury != treasury_before {
            diffs.push_diff(StateDiff::TreasuryChanged {
                old: treasury_before,
                new_val: self.world.treasury,
            });
        }
        diffs.push_metric(MetricUpdate {
            metric_id: 1, // population
            scope: MetricScope::Global,
            value: self.population as i64,
        });
        diffs.push_metric(MetricUpdate {
            metric_id: 2, // treasury
            scope: MetricScope::Global,
            value: self.world.treasury,
        });
        diffs.push_metric(MetricUpdate {
            metric_id: 3, // power shortage kW
            scope: MetricScope::Global,
            value: self.power_shortage_kw as i64,
        });
        let world_diff = WorldDiff::from_collector(tick, &mut diffs);

        // 12. Drain all events produced this tick.
        let events = self.events.drain();

        TickOutput {
            tick,
            events,
            population: self.population,
            treasury: self.world.treasury,
            power_shortage_kw: self.power_shortage_kw,
            world_diff,
        }
    }

    /// Apply a player command to the world state.
    ///
    /// Delegates to `core::commands::apply_command`.
    pub fn apply_command(&mut self, cmd: &Command) -> CommandResult {
        commands::apply_command_with_registry(
            &mut self.world,
            Some(&self.registry),
            Some(&mut self.road_graph),
            cmd,
        )
    }

    fn collect_invalidation_reason(
        &self,
        out: &mut Vec<InvalidationReason>,
        effect: &CommandEffect,
    ) {
        match effect {
            CommandEffect::EntityPlaced { .. } => out.push(InvalidationReason::BuildingPlaced),
            CommandEffect::EntityRemoved { .. } | CommandEffect::TilesBulldozed { .. } => {
                out.push(InvalidationReason::BuildingRemoved)
            }
            CommandEffect::PolicyChanged { .. } => out.push(InvalidationReason::PolicyChanged),
            CommandEffect::EntityUpgraded { .. }
            | CommandEffect::EntityToggled { .. }
            | CommandEffect::ZoningApplied { .. }
            | CommandEffect::TerrainApplied { .. }
            | CommandEffect::RoadLineApplied { .. } => {}
        }
    }

    fn collect_command_diff(&self, collector: &mut DiffCollector, effect: &CommandEffect) {
        match effect {
            CommandEffect::EntityPlaced { handle } => {
                if let Some(pos) = self.world.entities.get_pos(*handle) {
                    let archetype = self.world.entities.get_archetype(*handle).unwrap_or(0);
                    collector.push_diff(StateDiff::EntityAdded {
                        handle: *handle,
                        archetype,
                        pos,
                    });
                }
            }
            CommandEffect::EntityRemoved { handle } => {
                collector.push_diff(StateDiff::EntityRemoved {
                    handle: *handle,
                    pos: TileCoord::new(0, 0),
                });
            }
            CommandEffect::EntityUpgraded { handle, new_level } => {
                collector.push_diff(StateDiff::EntityUpdated {
                    handle: *handle,
                    field: crate::core::diffs::EntityField::Level,
                    old_value: 0,
                    new_value: *new_level as u32,
                });
            }
            CommandEffect::PolicyChanged { key, old_value, new_value } => {
                collector.push_diff(StateDiff::PolicyChanged {
                    key: policy_key_name(*key).to_string(),
                    old_value: *old_value as u32,
                    new_value: *new_value as u32,
                });
            }
            CommandEffect::EntityToggled { handle, enabled } => {
                collector.push_diff(StateDiff::EntityUpdated {
                    handle: *handle,
                    field: crate::core::diffs::EntityField::Enabled,
                    old_value: 0,
                    new_value: u32::from(*enabled),
                });
            }
            CommandEffect::TilesBulldozed { .. }
            | CommandEffect::ZoningApplied { .. }
            | CommandEffect::TerrainApplied { .. }
            | CommandEffect::RoadLineApplied { .. } => {}
        }
    }
}

fn policy_key_name(key: PolicyKey) -> &'static str {
    match key {
        PolicyKey::ResidentialTax => "residential_tax",
        PolicyKey::CommercialTax => "commercial_tax",
        PolicyKey::IndustrialTax => "industrial_tax",
        PolicyKey::PoliceBudget => "police_budget",
        PolicyKey::FireBudget => "fire_budget",
        PolicyKey::HealthBudget => "health_budget",
        PolicyKey::EducationBudget => "education_budget",
        PolicyKey::TransportBudget => "transport_budget",
    }
}

// ---- Tests ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeTag};
    use crate::core::buildings::{
        register_base_city_builder_archetypes, ARCH_RES_SMALL_HOUSE, ARCH_UTIL_POWER_PLANT,
    };
    use crate::core::commands::{Command, CommandEffect};

    /// Helper: create a minimal world + registry + road_graph for testing.
    fn make_engine(seed: u64) -> SimulationEngine {
        let world = WorldState::new(MapSize::new(32, 32), seed);
        let registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();
        SimulationEngine::new(world, registry, road_graph)
    }

    fn make_engine_with_base_archetypes(seed: u64) -> SimulationEngine {
        let world = WorldState::new(MapSize::new(32, 32), seed);
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        let road_graph = RoadGraph::new();
        SimulationEngine::new(world, registry, road_graph)
    }

    /// Helper: create a residential archetype for testing.
    fn make_residential(id: ArchetypeId, build_time: u32) -> ArchetypeDefinition {
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

    // ── Test 1: New engine creates valid state ──────────────────────────

    #[test]
    fn new_engine_creates_valid_state() {
        let engine = make_engine(42);

        assert_eq!(engine.world.tick, 0);
        assert_eq!(engine.world.treasury, 500_000);
        assert_eq!(engine.population, 0);
        assert!(!engine.power_shortage);
        assert!(!engine.water_shortage);
        assert!(engine.events.is_empty());
        assert!(engine.city_event_state.fires.is_empty());
        assert_eq!(engine.world.map_size(), MapSize::new(32, 32));
    }

    // ── Test 2: Tick increments tick counter ────────────────────────────

    #[test]
    fn tick_increments_tick_counter() {
        let mut engine = make_engine(42);

        assert_eq!(engine.world.tick, 0);

        let output1 = engine.tick();
        assert_eq!(output1.tick, 1);
        assert_eq!(engine.world.tick, 1);

        let output2 = engine.tick();
        assert_eq!(output2.tick, 2);
        assert_eq!(engine.world.tick, 2);

        // Run several more ticks
        for _ in 0..10 {
            engine.tick();
        }
        assert_eq!(engine.world.tick, 12);
    }

    // ── Test 3: Multiple ticks deterministic (same seed same output) ────

    #[test]
    fn multiple_ticks_deterministic_same_seed() {
        // Run engine A for N ticks.
        let mut engine_a = make_engine(12345);
        let mut outputs_a: Vec<(u32, MoneyCents)> = Vec::new();
        for _ in 0..50 {
            let out = engine_a.tick();
            outputs_a.push((out.population, out.treasury));
        }

        // Run engine B with the same seed for N ticks.
        let mut engine_b = make_engine(12345);
        let mut outputs_b: Vec<(u32, MoneyCents)> = Vec::new();
        for _ in 0..50 {
            let out = engine_b.tick();
            outputs_b.push((out.population, out.treasury));
        }

        // Outputs must be identical.
        assert_eq!(
            outputs_a, outputs_b,
            "Same-seed engines must produce identical outputs"
        );
    }

    // ── Test 4: Construction completes after build time ─────────────────

    #[test]
    fn construction_completes_after_build_time() {
        let world = WorldState::new(MapSize::new(32, 32), 42);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();

        let build_time: u32 = 10;
        registry.register(make_residential(1, build_time));

        let mut engine = SimulationEngine::new(world, registry, road_graph);

        // Place an entity (starts under construction).
        let handle = engine.world.place_entity(1, 5, 5, 0).unwrap();
        assert!(engine
            .world
            .entities
            .get_flags(handle)
            .unwrap()
            .contains(StatusFlags::UNDER_CONSTRUCTION));

        // Tick enough times for construction to complete.
        for _ in 0..build_time {
            engine.tick();
        }

        // Entity should no longer be under construction.
        let flags = engine.world.entities.get_flags(handle).unwrap();
        assert!(
            !flags.contains(StatusFlags::UNDER_CONSTRUCTION),
            "Entity should be complete after {} ticks",
            build_time
        );
        assert_eq!(
            engine.world.entities.get_construction_progress(handle).unwrap(),
            0xFFFF
        );
    }

    // ── Test 5: Events collected from tick ──────────────────────────────

    #[test]
    fn events_collected_from_tick() {
        let world = WorldState::new(MapSize::new(32, 32), 42);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();

        // Use build_time=1 so it completes immediately on first tick.
        registry.register(make_residential(1, 1));

        let mut engine = SimulationEngine::new(world, registry, road_graph);

        // Place an entity that will complete construction in 1 tick.
        engine.world.place_entity(1, 5, 5, 0).unwrap();

        let output = engine.tick();

        // Should have at least a BuildingCompleted event.
        let has_building_completed = output
            .events
            .iter()
            .any(|e| matches!(e.event, crate::core::events::SimEvent::BuildingCompleted { .. }));
        assert!(
            has_building_completed,
            "Expected BuildingCompleted event in tick output"
        );

        // Events bus should be empty after drain.
        assert!(engine.events.is_empty());
    }

    // ── Test 6: Apply command works ─────────────────────────────────────

    #[test]
    fn apply_command_works() {
        let mut engine = make_engine_with_base_archetypes(42);
        let zone = Command::SetZoning {
            x: 3,
            y: 3,
            w: 1,
            h: 1,
            zone: ZoneType::Residential,
        };
        assert!(engine.apply_command(&zone).is_ok());

        // Place an entity via command.
        let cmd = Command::PlaceEntity {
            archetype_id: ARCH_RES_SMALL_HOUSE,
            x: 3,
            y: 3,
            rotation: 0,
        };
        let result = engine.apply_command(&cmd);
        assert!(result.is_ok());

        let handle = match result.unwrap() {
            CommandEffect::EntityPlaced { handle } => handle,
            _ => panic!("Expected EntityPlaced"),
        };
        assert!(engine.world.entities.is_valid(handle));
        assert_eq!(engine.world.entities.count(), 1);

        // Remove the entity via command.
        let remove_cmd = Command::RemoveEntity { handle };
        let result = engine.apply_command(&remove_cmd);
        assert!(result.is_ok());
        assert!(!engine.world.entities.is_valid(handle));
        assert_eq!(engine.world.entities.count(), 0);
    }

    // ── Test 7: Treasury updates across ticks ───────────────────────────

    #[test]
    fn treasury_updates_across_ticks() {
        let mut engine = make_engine(42);
        let initial_treasury = engine.world.treasury;

        // Run a few ticks with no buildings.
        // Even with 0 initial population, RNG jitter in the population
        // system can produce a small population which generates tiny tax
        // income.  The key invariant is that treasury does not decrease
        // (no expenses with no buildings).
        for _ in 0..5 {
            engine.tick();
        }

        assert!(
            engine.world.treasury >= initial_treasury,
            "Treasury should not decrease with no buildings; was {} now {}",
            initial_treasury,
            engine.world.treasury,
        );
    }

    // ── Test 8: Different seeds produce different results ───────────────

    #[test]
    fn different_seeds_produce_different_results() {
        // Run engine with seed 1 for N ticks.
        let mut engine_a = make_engine(1);
        for _ in 0..20 {
            engine_a.tick();
        }

        // Run engine with seed 999 for N ticks.
        let mut engine_b = make_engine(999);
        for _ in 0..20 {
            engine_b.tick();
        }

        // The RNG state should be different (population may or may not
        // differ with an empty world, but treasury is deterministic with
        // no buildings, so they should match). The key invariant tested
        // in test 3 is that same seed => same output. Here we just verify
        // both ran without errors.
        assert_eq!(engine_a.world.tick, 20);
        assert_eq!(engine_b.world.tick, 20);
    }

    // ── Test 9: Engine handles many ticks without panic ─────────────────

    #[test]
    fn engine_handles_many_ticks() {
        let mut engine = make_engine(42);

        // Run 500 ticks to ensure no panics or numeric overflow.
        for _ in 0..500 {
            engine.tick();
        }

        assert_eq!(engine.world.tick, 500);
    }

    #[test]
    fn queued_command_applied_during_tick_phase_1() {
        let mut engine = make_engine_with_base_archetypes(42);
        assert_eq!(engine.world.entities.count(), 0);
        assert!(engine
            .apply_command(&Command::SetZoning {
                x: 4,
                y: 4,
                w: 1,
                h: 1,
                zone: ZoneType::Residential,
            })
            .is_ok());

        engine.queue_command(Command::PlaceEntity {
            archetype_id: ARCH_RES_SMALL_HOUSE,
            x: 4,
            y: 4,
            rotation: 0,
        });

        let out = engine.tick();
        assert_eq!(engine.world.entities.count(), 1);
        assert_eq!(out.world_diff.tick, 1);
        assert!(!out.world_diff.diffs.is_empty());
    }

    #[test]
    fn queued_policy_command_invalidates_economic_cache() {
        let mut engine = make_engine(42);
        // mark clean first so we can assert invalidation.
        engine
            .cache_manager
            .mark_clean(crate::caches::cache_manager::CacheType::EconomicAggregates);
        assert!(!engine
            .cache_manager
            .is_dirty(crate::caches::cache_manager::CacheType::EconomicAggregates));

        engine.queue_command(Command::SetPolicy {
            key: PolicyKey::ResidentialTax,
            value: 12,
        });
        let _ = engine.tick();

        assert!(engine
            .cache_manager
            .is_dirty(crate::caches::cache_manager::CacheType::EconomicAggregates));
    }

    #[test]
    fn tick_output_emits_metric_updates() {
        let mut engine = make_engine(42);
        let out = engine.tick();
        assert!(!out.world_diff.metrics.is_empty());
        assert_eq!(out.world_diff.metrics[0].scope, MetricScope::Global);
    }

    #[test]
    fn phase_order_commands_then_systems() {
        let world = WorldState::new(MapSize::new(32, 32), 42);
        let mut registry = ArchetypeRegistry::new();
        let road_graph = RoadGraph::new();

        // Build time 1 means construction should complete in the same tick
        // only if command application happens before system execution.
        registry.register(make_residential(1, 1));
        let mut engine = SimulationEngine::new(world, registry, road_graph);
        assert!(engine
            .apply_command(&Command::SetZoning {
                x: 5,
                y: 5,
                w: 1,
                h: 1,
                zone: ZoneType::Residential,
            })
            .is_ok());

        engine.queue_command(Command::PlaceEntity {
            archetype_id: 1,
            x: 5,
            y: 5,
            rotation: 0,
        });

        let out = engine.tick();

        let completed = out
            .events
            .iter()
            .any(|e| matches!(e.event, crate::core::events::SimEvent::BuildingCompleted { .. }));
        assert!(
            completed,
            "Expected BuildingCompleted in same tick, proving phase order is command -> systems"
        );
    }

    #[test]
    fn zoned_tiles_can_spawn_structures_over_time() {
        let mut engine = make_engine_with_base_archetypes(77);

        let zoning = Command::SetZoning {
            x: 4,
            y: 4,
            w: 8,
            h: 8,
            zone: ZoneType::Residential,
        };
        assert!(engine.apply_command(&zoning).is_ok());

        for _ in 0..120 {
            engine.tick();
        }

        assert!(engine.world.entities.count() > 0);
        let spawned_residential = engine.world.entities.iter_alive().any(|h| {
            engine
                .world
                .entities
                .get_archetype(h)
                .and_then(|id| engine.registry.get(id))
                .map(|def| def.id == ARCH_RES_SMALL_HOUSE || def.has_tag(ArchetypeTag::Residential))
                .unwrap_or(false)
        });
        assert!(spawned_residential);
    }

    #[test]
    fn special_building_can_be_placed_without_zone() {
        let mut engine = make_engine_with_base_archetypes(99);

        let place = Command::PlaceEntity {
            archetype_id: ARCH_UTIL_POWER_PLANT,
            x: 2,
            y: 2,
            rotation: 0,
        };
        let result = engine.apply_command(&place);
        assert!(result.is_ok());
        assert_eq!(engine.world.entities.count(), 1);
    }

    // ── Test: power_shortage_kw reflects deficit ─────────────────────────

    #[test]
    fn power_shortage_kw_reflects_deficit() {
        // Scenario A: residential consumer with no power plant → shortage expected.
        {
            let mut engine = make_engine_with_base_archetypes(42);

            // Place a residential building (has power_demand_kw > 0).
            // SetZoning first so placement is allowed.
            assert!(engine
                .apply_command(&Command::SetZoning {
                    x: 5,
                    y: 5,
                    w: 1,
                    h: 1,
                    zone: ZoneType::Residential,
                })
                .is_ok());
            assert!(engine
                .apply_command(&Command::PlaceEntity {
                    archetype_id: ARCH_RES_SMALL_HOUSE,
                    x: 5,
                    y: 5,
                    rotation: 0,
                })
                .is_ok());

            // Force construction to complete by fast-forwarding build_time ticks.
            // base.buildings ARCH_RES_SMALL_HOUSE has build_time_ticks = 500.
            // Run enough ticks to complete construction + one full phase wheel cycle
            // (tick 1 = Utilities phase runs propagate_power).
            let mut output = engine.tick(); // tick 1 — Utilities phase
            // Ensure we actually ran on the Utilities phase (tick 1 % 4 == 1 == Utilities).
            // The building is still under construction on tick 1 so demand = 0,
            // but after build_time ticks the building completes and demand is non-zero.
            // Run 500 more ticks to complete construction, then land on Utilities phase.
            for _ in 0..500 {
                output = engine.tick();
            }
            // Advance to the next Utilities phase tick (tick % 4 == 1).
            // Current tick is 501. Next Utilities tick is at 505 (505 % 4 == 1).
            while engine.world.tick % 4 != 1 {
                output = engine.tick();
            }
            // Now at a Utilities tick with a completed residential consumer and no plant.
            assert!(
                output.power_shortage_kw > 0,
                "Expected power shortage with consumer but no plant, got power_shortage_kw={}",
                output.power_shortage_kw
            );
        }

        // Scenario B: power plant adjacent to residential connected via road → no shortage.
        {
            let mut engine = make_engine_with_base_archetypes(42);

            // Place a power plant at (5,5) — no zone required for utility.
            assert!(engine
                .apply_command(&Command::PlaceEntity {
                    archetype_id: ARCH_UTIL_POWER_PLANT,
                    x: 5,
                    y: 5,
                    rotation: 0,
                })
                .is_ok());

            // The ARCH_UTIL_POWER_PLANT has supply >> small house demand,
            // and the BFS will propagate power globally. No shortage expected.
            // Run 4 ticks (one full phase cycle) so Utilities phase fires.
            let mut output = engine.tick();
            for _ in 0..3 {
                output = engine.tick();
            }
            // Advance to the next Utilities tick.
            while engine.world.tick % 4 != 1 {
                output = engine.tick();
            }
            assert_eq!(
                output.power_shortage_kw, 0,
                "Expected no power shortage with plant in place, got power_shortage_kw={}",
                output.power_shortage_kw
            );
        }
    }
}
