//! Service coverage systems: ServiceHealth and ServiceSafety maps.
//!
//! `service_system` is a pure function that computes two SimMap layers each tick:
//!
//! * **ServiceHealth** — hospital/clinic radius-falloff coverage, normalised
//!   against the `beds_per_1000` anchor from `WorldVars`.
//! * **ServiceSafety** — weighted blend of `FireProtection` and
//!   `PoliceProtection` raw effect values, normalised to [0, 1].

use crate::core::archetypes::{ArchetypeRegistry, ArchetypeTag, EffectKind};
use crate::core::entity::EntityStore;
use crate::core::world_vars::WorldVars;
use crate::core_types::StatusFlags;
use crate::sim::systems::effects::EffectMap;
use crate::sim::sim_map::{SimMap, SimMapRegistry};

// ─── service_system ──────────────────────────────────────────────────────────

/// Compute `SimMap::ServiceHealth` and `SimMap::ServiceSafety` for one tick.
///
/// ## ServiceHealth
///
/// Scans all alive, completed, enabled entities that have either the
/// `Service` or `Civic` archetype tag **and** a `service_radius > 0`.
/// For each such "healthcare entity" every tile within its service radius
/// (Manhattan distance) accumulates a weighted contribution:
///
/// ```text
/// capacity_score = job_capacity() as f32 / (beds_per_1000 / 1000.0)
/// contribution  += capacity_score * (1.0 - dist / (service_radius + 1))
/// service_health[tile] = (contribution / 100.0).min(1.0)
/// ```
///
/// The `/100.0` normalisation means a reasonably-sized hospital covering a
/// medium zone yields values in the `0.8–1.0` range.
///
/// ## ServiceSafety
///
/// Reads the already-propagated `EffectMap` layers for `FireProtection` and
/// `PoliceProtection`, normalises them to `[0, 1]`, and blends with
/// `police_officers_per_1000`-derived weights:
///
/// ```text
/// fire_norm   = (fire_raw   / i16::MAX).clamp(0, 1)
/// police_norm = (police_raw / i16::MAX).clamp(0, 1)
/// service_safety = 0.6 * police_norm + 0.4 * fire_norm
/// ```
pub fn service_system(
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
    effect_map: &EffectMap,
    world_vars: &WorldVars,
    maps: &mut SimMapRegistry,
) {
    maps.clear_next(SimMap::ServiceHealth);
    maps.clear_next(SimMap::ServiceSafety);

    let width = maps.width();
    let height = maps.height();

    // ── ServiceHealth ────────────────────────────────────────────────────────

    // Collect healthcare entities: (pos_x, pos_y, service_radius, capacity_score)
    let mut hospitals: Vec<(i32, i32, u8, f32)> = Vec::new();

    for handle in entities.iter_alive() {
        // Skip under-construction or disabled entities.
        let flags = match entities.get_flags(handle) {
            Some(f) => f,
            None => continue,
        };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
            continue;
        }
        if !entities.get_enabled(handle).unwrap_or(true) {
            continue;
        }

        let arch_id = match entities.get_archetype(handle) {
            Some(id) => id,
            None => continue,
        };
        let def = match registry.get(arch_id) {
            Some(d) => d,
            None => continue,
        };

        // Only service/civic buildings with a service radius qualify.
        let is_service = def.has_tag(ArchetypeTag::Service) || def.has_tag(ArchetypeTag::Civic);
        if !is_service || def.service_radius == 0 {
            continue;
        }

        let pos = match entities.get_pos(handle) {
            Some(p) => p,
            None => continue,
        };

        // capacity_score: job_capacity proxies bed count.
        // Dividing by (beds_per_1000 / 1000) converts to "population this
        // building can serve".
        let beds_ratio = world_vars.beds_per_1000 / 1000.0;
        let capacity_score = if beds_ratio > 0.0 {
            def.job_capacity() as f32 / beds_ratio
        } else {
            def.job_capacity() as f32
        };

        hospitals.push((
            pos.x as i32,
            pos.y as i32,
            def.service_radius,
            capacity_score,
        ));
    }

    // For each tile accumulate contributions from all hospitals in range.
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

    // ── ServiceSafety ────────────────────────────────────────────────────────

    let fire_layer   = &effect_map.maps[EffectKind::FireProtection   as usize];
    let police_layer = &effect_map.maps[EffectKind::PoliceProtection as usize];

    let safety_buf = maps.next_mut(SimMap::ServiceSafety);
    let len = width * height;

    for i in 0..len {
        let fire_raw   = fire_layer.get(i).copied().unwrap_or(0);
        let police_raw = police_layer.get(i).copied().unwrap_or(0);

        let fire_norm   = (fire_raw   as f32 / i16::MAX as f32).clamp(0.0, 1.0);
        let police_norm = (police_raw as f32 / i16::MAX as f32).clamp(0.0, 1.0);

        // Weight blend: 0.6 police + 0.4 fire
        // (mirrors police_officers_per_1000 / (police_officers_per_1000 +
        //  fire_staff_per_1000) ≈ 2.4 / 3.8 ≈ 0.63 → rounded to 0.6/0.4)
        safety_buf[i] = 0.6 * police_norm + 0.4 * fire_norm;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag};
    use crate::core::entity::EntityStore;
    use crate::core::world_vars::WorldVars;
    use crate::sim::systems::effects::EffectMap;
    use crate::sim::sim_map::{SimMap, SimMapRegistry};
    use crate::core_types::StatusFlags;

    fn make_hospital(id: crate::core_types::ArchetypeId, service_radius: u8) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: "Hospital".to_string(),
            tags: vec![ArchetypeTag::Service, ArchetypeTag::Civic],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 80,
            floors: 4,
            usable_ratio_pct: 85,
            base_cost_cents: 500_000,
            base_upkeep_cents_per_tick: 100,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 20,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 0,
            build_time_ticks: 1,
            max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 20,          // gives non-zero job_capacity
            living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    /// A hospital placed at (5,5) with service_radius=5 should produce
    /// ServiceHealth > 0 at distance 0 (the hospital tile itself).
    #[test]
    fn health_coverage_increases_near_hospital() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_hospital(1, 5));

        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 5, 5, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE); // mark completed

        let effect_map = EffectMap::new(16, 16);
        let world_vars = WorldVars::default();
        let mut maps = SimMapRegistry::new(16, 16);

        service_system(&entities, &registry, &effect_map, &world_vars, &mut maps);

        // Swap so we can read the freshly-written values.
        maps.swap();

        let idx = 5 * 16 + 5;
        let health = maps.current(SimMap::ServiceHealth)[idx];
        assert!(
            health > 0.0,
            "expected ServiceHealth > 0.0 at hospital tile, got {health}"
        );
    }

    /// When EffectMap is all zeros, ServiceSafety must be all 0.0.
    #[test]
    fn safety_zero_when_no_protection() {
        let registry = ArchetypeRegistry::new();
        let entities = EntityStore::new(64);
        let effect_map = EffectMap::new(8, 8); // all zeros
        let world_vars = WorldVars::default();
        let mut maps = SimMapRegistry::new(8, 8);

        service_system(&entities, &registry, &effect_map, &world_vars, &mut maps);
        maps.swap();

        let safety = maps.current(SimMap::ServiceSafety);
        assert!(
            safety.iter().all(|&v| v == 0.0_f32),
            "expected all ServiceSafety == 0.0 when EffectMap is zeroed"
        );
    }
}
