//! Scenario setup — shared between headless and visual binaries.

use city_core::{App, MapSize, StatusFlags};
use city_sim::archetype::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
use city_engine::engine::SimulationEngine;
use city_sim::network::RoadNetwork;
use city_render::instance::GpuInstance;
use city_render::renderer::{self, FrameData};
use city_sim::plugin::{SimCorePlugin, SimConfig};
use city_sim::tilemap::{TileKind, TileValue};
use city_sim::types::{ZoneDensity, ZoneType};
use city_sim::world::WorldState;

// ─── Tile → Pattern mapping ─────────────────────────────────────────────────

/// Map a tile to its GPU pattern ID.
///
/// Pattern ID layout (matches TileVisualRegistry):
///   0-4:   Terrain (Grass, Water, Sand, Forest, Rock)
///   7:     Road
///  11-16:  Zones (empty — Residential, Commercial, Industrial, Civic, Park, Transport)
///  21-24:  Buildings (occupied — Residential, Commercial, Industrial, Civic)
pub fn tile_to_pattern(tile: &TileValue) -> u32 {
    match tile.kind {
        TileKind::Road => 7,
        TileKind::Building => match tile.zone {
            ZoneType::Residential => 21,
            ZoneType::Commercial  => 22,
            ZoneType::Industrial  => 23,
            ZoneType::Civic       => 24,
            _ => 0,
        },
        _ => match tile.zone {
            ZoneType::None        => 0,
            ZoneType::Residential => 11,
            ZoneType::Commercial  => 12,
            ZoneType::Industrial  => 13,
            ZoneType::Civic       => 14,
            ZoneType::Park        => 15,
            ZoneType::Transport   => 16,
        },
    }
}

/// Build GPU instances from the current engine state.
pub fn build_instances(engine: &SimulationEngine, max_dim: u16) -> Vec<GpuInstance> {
    let world = engine.get_resource::<WorldState>().unwrap();
    let w = world.tiles.width();
    let h = world.tiles.height();
    let mut tiles = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let color_id = match world.tiles.get(x, y) {
                Some(tile) => tile_to_pattern(&tile),
                None => 0,
            };
            tiles.push((x as i16, y as i16, color_id));
        }
    }
    renderer::build_terrain_instances(&tiles, max_dim)
}

/// Map archetype tag to sprite ID: 1=house, 2=shop, 3=factory, 4=civic.
fn archetype_to_sprite(registry: &ArchetypeRegistry, arch_id: u16) -> u32 {
    if let Some(def) = registry.get(arch_id) {
        if def.has_tag(ArchetypeTag::Residential) { return 1; }
        if def.has_tag(ArchetypeTag::Commercial) { return 2; }
        if def.has_tag(ArchetypeTag::Industrial) { return 3; }
        if def.has_tag(ArchetypeTag::Civic) { return 4; }
    }
    1 // fallback to house
}

/// Build complete frame data (terrain + sprites) from the current engine state.
pub fn build_frame_data(engine: &SimulationEngine, max_dim: u16) -> FrameData {
    let terrain = build_instances(engine, max_dim);

    let world = engine.get_resource::<WorldState>().unwrap();
    let registry = engine.get_resource::<ArchetypeRegistry>().unwrap();

    let mut buildings = Vec::new();
    for handle in world.entities.iter_alive() {
        let flags = match world.entities.get_flags(handle) {
            Some(f) => f,
            None => continue,
        };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
        if let (Some(pos), Some(arch_id)) = (world.entities.get_pos(handle), world.entities.get_archetype(handle)) {
            let sprite_id = archetype_to_sprite(registry, arch_id);
            buildings.push((pos.x, pos.y, sprite_id));
        }
    }
    let sprites = renderer::build_sprite_instances(&buildings, max_dim);

    FrameData { terrain, sprites }
}

// ─── Archetypes ──────────────────────────────────────────────────────────────

/// Register archetypes with short build times for fast visible growth.
pub fn register_fast_archetypes(registry: &mut ArchetypeRegistry) {
    registry.register(ArchetypeDefinition {
        id: 1, name: "Small House".into(),
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
    });

    registry.register(ArchetypeDefinition {
        id: 3, name: "Shop".into(),
        tags: vec![ArchetypeTag::Commercial, ArchetypeTag::LowDensity],
        footprint_w: 1, footprint_h: 1,
        coverage_ratio_pct: 70, floors: 1, usable_ratio_pct: 85,
        base_cost_cents: 8_000, base_upkeep_cents_per_tick: 1,
        power_demand_kw: 10, power_supply_kw: 0,
        water_demand: 3, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 3, desirability_magnitude: 3,
        pollution: 0, noise: 2,
        build_time_ticks: 8, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 25, living_space_per_person_m2: 0,
        effects: vec![],
    });

    registry.register(ArchetypeDefinition {
        id: 4, name: "Factory".into(),
        tags: vec![ArchetypeTag::Industrial, ArchetypeTag::LowDensity],
        footprint_w: 1, footprint_h: 1,
        coverage_ratio_pct: 60, floors: 1, usable_ratio_pct: 80,
        base_cost_cents: 12_000, base_upkeep_cents_per_tick: 2,
        power_demand_kw: 15, power_supply_kw: 0,
        water_demand: 5, water_supply: 0,
        water_coverage_radius: 0, is_water_pipe: false,
        service_radius: 0,
        desirability_radius: 4, desirability_magnitude: -10,
        pollution: 5, noise: 4,
        build_time_ticks: 12, max_level: 3,
        prerequisites: vec![],
        workspace_per_job_m2: 50, living_space_per_person_m2: 0,
        effects: vec![],
    });
}

// ─── Town layout ─────────────────────────────────────────────────────────────

/// Build a compact town on a 64x64 map.
///
/// Layout:
///   Roads: horizontal at y=10,20,30,40; vertical at x=10,30,50
///   Residential: (11-29, 11-19) and (31-49, 11-19) — 2 blocks, 342 tiles
///   Commercial:  (11-29, 21-29) — 1 block, 171 tiles
///   Industrial:  (31-49, 21-29) — 1 block, 171 tiles
///   Pre-placed: 4 houses (completed), starting pop moving in
pub fn build_compact_town(world: &mut WorldState) {
    world.treasury = 50_000_000;

    let w = world.tiles.width();
    let h = world.tiles.height();

    // Roads
    for &ry in &[10u32, 20, 30, 40] {
        for x in 8..52u32 {
            if x < w && ry < h {
                world.tiles.set_kind(x, ry, TileKind::Road);
            }
        }
    }
    for &rx in &[10u32, 30, 50] {
        for y in 8..42u32 {
            if rx < w && y < h {
                world.tiles.set_kind(rx, y, TileKind::Road);
            }
        }
    }

    // Mark road-adjacent tiles with ROAD_ACCESS
    for y in 0..h {
        for x in 0..w {
            if let Some(tile) = world.tiles.get(x, y) {
                if tile.kind == TileKind::Road {
                    for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                            use city_sim::tilemap::TileFlags;
                            world.tiles.set_flags(nx as u32, ny as u32, TileFlags::ROAD_ACCESS);
                        }
                    }
                }
            }
        }
    }

    // Residential zones
    for y in 11..20u32 {
        for x in 11..30u32 {
            world.tiles.set_zone(x, y, ZoneType::Residential);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }
    for y in 11..20u32 {
        for x in 31..50u32 {
            world.tiles.set_zone(x, y, ZoneType::Residential);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Commercial zone
    for y in 21..30u32 {
        for x in 11..30u32 {
            world.tiles.set_zone(x, y, ZoneType::Commercial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Industrial zone
    for y in 21..30u32 {
        for x in 31..50u32 {
            world.tiles.set_zone(x, y, ZoneType::Industrial);
            world.tiles.set_density(x, y, ZoneDensity::Low);
        }
    }

    // Pre-place 4 completed houses for immediate housing capacity
    for &(x, y) in &[(12i16, 12i16), (14, 12), (16, 12), (18, 12)] {
        if let Some(handle) = world.place_entity(1, x, y, 0) {
            world.entities.set_construction_progress(handle, 0xFFFF);
            world.entities.set_flags(handle, StatusFlags::POWERED | StatusFlags::WATER_CONNECTED);
            world.tiles.set_kind(x as u32, y as u32, TileKind::Building);
        }
    }
}

// ─── Engine builder ──────────────────────────────────────────────────────────

/// Create a SimulationEngine with the compact 64x64 town scenario.
pub fn create_compact_engine() -> (SimulationEngine, MapSize) {
    let map_size = MapSize::new(64, 64);

    let mut app = App::new();
    app.add_plugins(SimCorePlugin::new(SimConfig {
        map_size,
        seed: 42,
        city_name: "Compact Town".into(),
    }));

    {
        let registry = app.get_resource_mut::<ArchetypeRegistry>().unwrap();
        register_fast_archetypes(registry);
    }

    app.insert_resource(RoadNetwork::new());

    {
        let world = app.get_resource_mut::<WorldState>().unwrap();
        build_compact_town(world);
    }

    (SimulationEngine::from_app(app), map_size)
}
