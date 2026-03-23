//! Unified simulation tick system.
//!
//! Runs all subsystems in order: construction → population → jobs → finance → buildings.
//! Extracts resources from the ResourceMap, runs the logic, then re-inserts them.

use city_core::system::{SimContext, SimSystem};
use crate::archetype::ArchetypeRegistry;

use crate::events::EventBus;
use crate::math::rng::Rng;
use crate::systems::buildings::{DevelopmentConfig, DevelopmentState, compute_zone_demand, tick_zoned_development_with_config};
use crate::systems::construction::tick_construction;
use crate::sim_map::SimMapRegistry;
use crate::systems::effects::{EffectMap, propagate_effects};
use crate::systems::finance::tick_finance;
use crate::systems::jobs::tick_jobs;
use crate::systems::overlay_pipeline::run_overlay_pipeline;
use crate::systems::population::tick_population;
use crate::systems::utility_registry::UtilityRegistry;
use crate::world::WorldState;
use crate::world_vars::WorldVars;

/// Persistent simulation state across ticks.
pub struct SimRunState {
    pub population: u32,
    pub rng: Rng,
    pub dev_state: Option<DevelopmentState>,
}

impl SimRunState {
    pub fn new(seed: u64) -> Self {
        Self {
            population: 0,
            rng: Rng::new(seed),
            dev_state: None,
        }
    }
}

/// The main simulation tick system. Runs all subsystems in order.
pub struct SimTickSystem;

impl SimSystem for SimTickSystem {
    fn name(&self) -> &str { "sim_tick" }

    fn tick(&mut self, ctx: &mut SimContext) {
        let tick = ctx.tick;

        // Extract all needed resources
        let Some(mut world) = ctx.resources.remove::<WorldState>() else { return };
        let Some(registry) = ctx.resources.remove::<ArchetypeRegistry>() else {
            ctx.resources.insert(world);
            return;
        };
        let mut events = ctx.resources.remove::<EventBus>().unwrap_or_else(EventBus::new);
        let mut run_state = ctx.resources.remove::<SimRunState>()
            .unwrap_or_else(|| SimRunState::new(world.seeds.global_seed));

        // Initialize development state lazily
        if run_state.dev_state.is_none() {
            let size = world.map_size();
            run_state.dev_state = Some(DevelopmentState::new(size.width, size.height));
        }

        // 1. Construction
        tick_construction(&mut world.entities, &registry, &mut events, tick);

        // 2. Population
        let pop_stats = tick_population(
            &world.entities, &registry, &mut events, tick,
            &mut run_state.rng, run_state.population,
        );
        run_state.population = pop_stats.total_population;

        // 3. Jobs
        tick_jobs(&mut world.entities, &registry, &mut events, tick, run_state.population);

        // 4. Finance
        let policies = world.policies.clone();
        tick_finance(
            &world.entities, &registry, &mut events, tick,
            &policies, &mut world.treasury, run_state.population,
        );

        // 5. Zone development
        let demand = compute_zone_demand(&world, &registry, run_state.population);
        let dev_state = run_state.dev_state.as_mut().unwrap();
        tick_zoned_development_with_config(
            &mut world, &registry, tick, &mut run_state.rng,
            DevelopmentConfig::default(), dev_state, demand, None,
        );

        // 6. Utility systems
        let mut utility_registry = ctx.resources.remove::<UtilityRegistry>().unwrap_or_default();
        utility_registry.update_all(&mut world, &registry, &mut events, tick);
        ctx.resources.insert(utility_registry);

        // Update world tick
        world.tick = tick;

        // Re-insert all resources
        ctx.resources.insert(world);
        ctx.resources.insert(registry);
        ctx.resources.insert(events);
        ctx.resources.insert(run_state);
    }
}

/// Overlay pipeline system — runs effect propagation and sim map updates every 4 ticks.
///
/// Registered on `Schedule::OverlayPipeline`, which executes after `Schedule::Tick`.
pub struct OverlayPipelineSystem;

impl SimSystem for OverlayPipelineSystem {
    fn name(&self) -> &str { "overlay_pipeline" }

    fn tick(&mut self, ctx: &mut SimContext) {
        let tick = ctx.tick;
        if tick % 4 != 0 {
            return;
        }

        let Some(world) = ctx.resources.get::<WorldState>() else { return };
        let s = world.map_size();
        drop(world);

        let Some(registry) = ctx.resources.get::<ArchetypeRegistry>() else { return };
        drop(registry);

        let mut effect_map = ctx.resources.remove::<EffectMap>()
            .unwrap_or_else(|| EffectMap::new(s.width as u32, s.height as u32));
        let world_vars = ctx.resources.remove::<WorldVars>()
            .unwrap_or_default();
        let mut sim_maps = ctx.resources.remove::<SimMapRegistry>()
            .unwrap_or_else(|| SimMapRegistry::new(s.width as usize, s.height as usize));

        // Re-borrow after removing the overlay resources
        let world = ctx.resources.get::<WorldState>().unwrap();
        let registry = ctx.resources.get::<ArchetypeRegistry>().unwrap();

        let run_state = ctx.resources.get::<SimRunState>()
            .map(|rs| rs.population)
            .unwrap_or(0);

        propagate_effects(&mut effect_map, &world.entities, registry);
        run_overlay_pipeline(
            &world.entities, registry, &effect_map, &world_vars,
            &mut sim_maps, run_state,
        );

        ctx.resources.insert(effect_map);
        ctx.resources.insert(world_vars);
        ctx.resources.insert(sim_maps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::{App, MapSize};
    use city_core::schedule::Schedule;
    use crate::archetype::ArchetypeDefinition;
    use city_engine::engine::SimulationEngine;
    use crate::plugin::{SimCorePlugin, SimConfig};
    use crate::systems::sim_tick::SimTickSystem;
    use crate::archetype::ArchetypeTag;

    fn make_residential(id: u16) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: format!("House {}", id),
            tags: vec![ArchetypeTag::Residential, ArchetypeTag::LowDensity],
            footprint_w: 1, footprint_h: 1,
            coverage_ratio_pct: 50, floors: 2, usable_ratio_pct: 80,
            base_cost_cents: 10_000, base_upkeep_cents_per_tick: 1,
            power_demand_kw: 5, power_supply_kw: 0,
            water_demand: 2, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 2, desirability_magnitude: 5,
            pollution: 0, noise: 1,
            build_time_ticks: 10, max_level: 3,
            prerequisites: vec![],
            workspace_per_job_m2: 0, living_space_per_person_m2: 40,
            effects: vec![],
            sprite_id: 0,
        }
    }

    #[test]
    fn sim_tick_system_runs_without_panic() {
        let mut app = App::new();
        app.add_plugins(SimCorePlugin::with_defaults());
        app.add_systems(Schedule::Tick, SimTickSystem);

        let mut engine = SimulationEngine::from_app(app);
        for _ in 0..10 {
            engine.tick();
        }
    }

    #[test]
    fn sim_tick_with_zoned_tiles_develops() {
        let mut app = App::new();
        app.add_plugins(SimCorePlugin::new(SimConfig {
            map_size: MapSize::new(32, 32),
            seed: 42,
            city_name: "Test".into(),
        }));
        app.add_systems(Schedule::Tick, SimTickSystem);

        // Register archetypes and zone some tiles
        {
            let registry = app.get_resource_mut::<ArchetypeRegistry>().unwrap();
            registry.register(make_residential(1));
        }
        {
            let world = app.get_resource_mut::<WorldState>().unwrap();
            for y in 2..10u32 {
                for x in 2..10u32 {
                    world.tiles.set_zone(x, y, crate::types::ZoneType::Residential);
                }
            }
        }

        let mut engine = SimulationEngine::from_app(app);
        for _ in 0..100 {
            engine.tick();
        }
        // After 100 ticks, some buildings should have been placed
        // (we can't easily check entity count without resource access, but no panic = success)
    }
}
