//! Overlay pipeline — runs service, traffic, pollution bridge, and land value.
//!
//! All float logic quarantined here (keeps sim_tick integer-deterministic).
//! Runs every 4 ticks, gated by the caller.

use city_core::StatusFlags;
use crate::archetype::{ArchetypeRegistry, ArchetypeTag, EffectKind};
use city_engine::entity::EntityStore;

use crate::sim_map::{SimMap, SimMapRegistry};
use crate::systems::effects::EffectMap;
use crate::world_vars::WorldVars;

/// Run the full overlay pipeline: service → traffic → pollution bridge → land value.
pub fn run_overlay_pipeline(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    effect_map: &EffectMap,
    world_vars: &WorldVars,
    maps: &mut SimMapRegistry,
    population: u32,
) {
    compute_service_health(entities, registry, effect_map, world_vars, maps);
    compute_service_safety(effect_map, maps);
    compute_traffic_density(entities, registry, maps, population);
    bridge_pollution(effect_map, maps);
    compute_land_value(world_vars, maps);
    maps.swap();
}

/// ServiceHealth: hospital/clinic coverage with radius falloff.
fn compute_service_health(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    _effect_map: &EffectMap,
    world_vars: &WorldVars,
    maps: &mut SimMapRegistry,
) {
    maps.clear_next(SimMap::ServiceHealth);

    let width = maps.width();
    let height = maps.height();

    // Collect healthcare entities
    let mut hospitals: Vec<(i32, i32, u8, f32)> = Vec::new();
    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) {
            Some(f) => f,
            None => continue,
        };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }
        if !entities.get_enabled(handle).unwrap_or(true) { continue; }

        let arch_id = match entities.get_archetype(handle) {
            Some(id) => id,
            None => continue,
        };
        let def = match registry.get(arch_id) {
            Some(d) => d,
            None => continue,
        };

        let is_service = def.has_tag(ArchetypeTag::Service) || def.has_tag(ArchetypeTag::Civic);
        if !is_service || def.service_radius == 0 { continue; }

        let pos = match entities.get_pos(handle) {
            Some(p) => p,
            None => continue,
        };

        let beds_ratio = world_vars.beds_per_1000 / 1000.0;
        let capacity_score = if beds_ratio > 0.0 {
            def.job_capacity() as f32 / beds_ratio
        } else {
            def.job_capacity() as f32
        };

        hospitals.push((pos.x as i32, pos.y as i32, def.service_radius, capacity_score));
    }

    if !hospitals.is_empty() {
        let health_buf = maps.next_mut(SimMap::ServiceHealth);
        for ty in 0..height {
            for tx in 0..width {
                let i = ty * width + tx;
                let mut contribution = 0.0_f32;
                for &(hx, hy, radius, cap_score) in &hospitals {
                    let dist = ((tx as i32 - hx).abs() + (ty as i32 - hy).abs()) as u32;
                    if dist <= radius as u32 {
                        let falloff = 1.0 - dist as f32 / (radius as f32 + 1.0);
                        contribution += cap_score * falloff;
                    }
                }
                health_buf[i] = (contribution / 100.0).min(1.0_f32);
            }
        }
    }
}

/// ServiceSafety: blend of fire and police protection from EffectMap.
fn compute_service_safety(effect_map: &EffectMap, maps: &mut SimMapRegistry) {
    maps.clear_next(SimMap::ServiceSafety);

    let len = maps.width() * maps.height();
    let fire_layer = &effect_map.maps[EffectKind::FireProtection as usize];
    let police_layer = &effect_map.maps[EffectKind::PoliceProtection as usize];

    let safety_buf = maps.next_mut(SimMap::ServiceSafety);

    for i in 0..len {
        let fire_raw = fire_layer.get(i).copied().unwrap_or(0);
        let police_raw = police_layer.get(i).copied().unwrap_or(0);

        let fire_norm = (fire_raw as f32 / i16::MAX as f32).clamp(0.0, 1.0);
        let police_norm = (police_raw as f32 / i16::MAX as f32).clamp(0.0, 1.0);

        safety_buf[i] = 0.6 * police_norm + 0.4 * fire_norm;
    }
}

/// Traffic density: simple approximation based on population and residential entities.
fn compute_traffic_density(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    maps: &mut SimMapRegistry,
    _population: u32,
) {
    maps.clear_next(SimMap::TrafficDensity);

    let width = maps.width();

    // Collect residential and commercial/industrial positions
    let mut res_positions: Vec<(i32, i32, u32)> = Vec::new();
    let mut work_positions: Vec<(i32, i32, u32)> = Vec::new();

    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) {
            Some(f) => f,
            None => continue,
        };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) { continue; }

        let arch_id = match entities.get_archetype(handle) {
            Some(id) => id,
            None => continue,
        };
        let def = match registry.get(arch_id) {
            Some(d) => d,
            None => continue,
        };
        let pos = match entities.get_pos(handle) {
            Some(p) => p,
            None => continue,
        };

        if def.has_tag(ArchetypeTag::Residential) {
            res_positions.push((pos.x as i32, pos.y as i32, def.resident_capacity()));
        }
        if def.has_tag(ArchetypeTag::Commercial) || def.has_tag(ArchetypeTag::Industrial) {
            work_positions.push((pos.x as i32, pos.y as i32, def.job_capacity()));
        }
    }

    // Simple: each residential entity generates trips to nearest work
    let traffic_buf = maps.next_mut(SimMap::TrafficDensity);
    let road_capacity = 1000.0_f32;

    for &(rx, ry, pop) in &res_positions {
        if pop == 0 { continue; }
        // Add density at home tile
        let home_idx = (ry as usize) * width + (rx as usize);
        if home_idx < traffic_buf.len() {
            traffic_buf[home_idx] += pop as f32 / road_capacity;
        }
        // Find nearest work and add density there too
        if let Some(&(wx, wy, _)) = work_positions.iter()
            .min_by_key(|&&(wx, wy, _)| (wx - rx).abs() + (wy - ry).abs())
        {
            let work_idx = (wy as usize) * width + (wx as usize);
            if work_idx < traffic_buf.len() {
                traffic_buf[work_idx] += pop as f32 / road_capacity;
            }
        }
    }

    // Clamp to [0, 1]
    for v in traffic_buf.iter_mut() {
        *v = v.min(1.0);
    }
}

/// Bridge pollution from EffectMap (i16) to SimMap (f32 [0,1]).
fn bridge_pollution(effect_map: &EffectMap, maps: &mut SimMapRegistry) {
    maps.clear_next(SimMap::Pollution);

    let width = maps.width();
    let height = maps.height();
    let len = width * height;
    let poll_layer = &effect_map.maps[EffectKind::Pollution as usize];

    // Normalize: max observed pollution → 1.0
    let max_poll = poll_layer.iter().take(len).map(|&v| v.max(0)).max().unwrap_or(1).max(1);

    let poll_buf = maps.next_mut(SimMap::Pollution);
    for i in 0..len.min(poll_layer.len()) {
        poll_buf[i] = (poll_layer[i].max(0) as f32 / max_poll as f32).clamp(0.0, 1.0);
    }
}

/// Land value: product-of-ratios formula from WorldVars.
///
/// LV = needs_met × (1 - pollution) × (1 - congestion_penalty)
fn compute_land_value(world_vars: &WorldVars, maps: &mut SimMapRegistry) {
    let width = maps.width();
    let height = maps.height();
    let len = width * height;

    // Clone the input maps (already written to next buffer in earlier steps)
    let health = maps.next_mut(SimMap::ServiceHealth).clone();
    let safety = maps.next_mut(SimMap::ServiceSafety).clone();
    let pollution = maps.next_mut(SimMap::Pollution).clone();
    let traffic = maps.next_mut(SimMap::TrafficDensity).clone();

    maps.clear_next(SimMap::LandValue);
    let lv_buf = maps.next_mut(SimMap::LandValue);
    for i in 0..len {
        let needs_met = (health.get(i).copied().unwrap_or(0.0)
            + safety.get(i).copied().unwrap_or(0.0)) / 2.0;
        let poll = pollution.get(i).copied().unwrap_or(0.0);
        let cong = traffic.get(i).copied().unwrap_or(0.0) * world_vars.congestion_slope;

        lv_buf[i] = (needs_met * (1.0 - poll) * (1.0 - cong)).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::ArchetypeDefinition;

    fn make_hospital(id: u16) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: "Hospital".into(),
            tags: vec![ArchetypeTag::Service, ArchetypeTag::Civic],
            footprint_w: 2, footprint_h: 2,
            coverage_ratio_pct: 80, floors: 4, usable_ratio_pct: 85,
            base_cost_cents: 500_000, base_upkeep_cents_per_tick: 100,
            power_demand_kw: 50, power_supply_kw: 0,
            water_demand: 20, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 5,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 0, noise: 0,
            build_time_ticks: 1, max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 20, living_space_per_person_m2: 0,
            effects: vec![],
            sprite_id: 0,
        }
    }

    #[test]
    fn pipeline_produces_service_health() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_hospital(1));

        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);

        let effect_map = EffectMap::new(16, 16);
        let world_vars = WorldVars::default();
        let mut maps = SimMapRegistry::new(16, 16);

        run_overlay_pipeline(&entities, &registry, &effect_map, &world_vars, &mut maps, 0);

        let health = maps.current(SimMap::ServiceHealth)[5 * 16 + 5];
        assert!(health > 0.0, "expected ServiceHealth > 0 near hospital, got {health}");
    }

    #[test]
    fn pipeline_safety_zero_when_no_protection() {
        let registry = ArchetypeRegistry::new();
        let entities = EntityStore::new(64);
        let effect_map = EffectMap::new(8, 8);
        let world_vars = WorldVars::default();
        let mut maps = SimMapRegistry::new(8, 8);

        run_overlay_pipeline(&entities, &registry, &effect_map, &world_vars, &mut maps, 0);

        let safety = maps.current(SimMap::ServiceSafety);
        assert!(safety.iter().all(|&v| v == 0.0_f32));
    }

    #[test]
    fn pipeline_land_value_in_range() {
        let registry = ArchetypeRegistry::new();
        let entities = EntityStore::new(64);
        let effect_map = EffectMap::new(8, 8);
        let world_vars = WorldVars::default();
        let mut maps = SimMapRegistry::new(8, 8);

        run_overlay_pipeline(&entities, &registry, &effect_map, &world_vars, &mut maps, 0);

        let lv = maps.current(SimMap::LandValue);
        assert!(lv.iter().all(|&v| v >= 0.0 && v <= 1.0));
    }
}
