//! Unified spatial-effects propagation for the simulation.
//!
//! `EffectMap` is the single source of truth for all per-tile overlay data
//! (pollution, land value, crime, fire/police protection, etc.).
//! `propagate_effects` iterates completed buildings and writes into it.
//! `smooth_pass` applies the SimCity-style 4-neighbour weighted average
//! that spreads service coverage spatially.
//! `compute_crime_map` implements the SimCity CrimeScan formula.

use crate::core::archetypes::{ArchetypeRegistry, EffectKind, EFFECT_KIND_COUNT};
use crate::core::entity::EntityStore;
use crate::core_types::StatusFlags;

// ─── EffectMap ────────────────────────────────────────────────────────────────

/// Per-tile overlay maps, one layer per `EffectKind`.
///
/// Each layer is a flat `Vec<i16>` in row-major order.
/// Positive = beneficial effect; negative = harmful.
#[derive(Debug, Clone)]
pub struct EffectMap {
    pub maps: [Vec<i16>; EFFECT_KIND_COUNT],
    pub width: u32,
    pub height: u32,
}

impl EffectMap {
    /// Create a new zeroed effect map for a `width × height` tile grid.
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width * height) as usize;
        EffectMap {
            maps: std::array::from_fn(|_| vec![0i16; len]),
            width,
            height,
        }
    }

    /// Zero all layers.
    pub fn clear(&mut self) {
        for layer in &mut self.maps {
            layer.iter_mut().for_each(|v| *v = 0);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }

    #[inline]
    fn idx(&self, x: u32, y: u32) -> Option<usize> {
        if x < self.width && y < self.height {
            Some((y * self.width + x) as usize)
        } else {
            None
        }
    }

    /// Accumulate `delta` at `(x, y)` for `kind`, clamping to `[i16::MIN, i16::MAX]`.
    #[inline]
    pub fn add(&mut self, kind: EffectKind, x: u32, y: u32, delta: i32) {
        if let Some(i) = self.idx(x, y) {
            let layer = &mut self.maps[kind as usize];
            layer[i] = (layer[i] as i32 + delta).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
    }

    /// Get the value at `(x, y)` for `kind`.
    #[inline]
    pub fn get(&self, kind: EffectKind, x: u32, y: u32) -> i16 {
        self.idx(x, y)
            .map(|i| self.maps[kind as usize][i])
            .unwrap_or(0)
    }
}

// ─── smooth_pass ──────────────────────────────────────────────────────────────

/// Apply N passes of a 4-neighbour weighted average over a layer.
///
/// Formula per cell: `(N + S + E + W + center) >> 2`.
/// Equivalent to SimCity `DoSmooth` used for fire/police/pollution smoothing.
pub fn smooth_pass(layer: &mut Vec<i16>, width: u32, height: u32, passes: u8) {
    let w = width as usize;
    let h = height as usize;
    let mut scratch = layer.clone();
    for _ in 0..passes {
        for y in 0..h {
            for x in 0..w {
                let center = layer[y * w + x] as i32;
                let n = if y > 0 { layer[(y - 1) * w + x] as i32 } else { center };
                let s = if y + 1 < h { layer[(y + 1) * w + x] as i32 } else { center };
                let e = if x + 1 < w { layer[y * w + x + 1] as i32 } else { center };
                let west = if x > 0 { layer[y * w + x - 1] as i32 } else { center };
                scratch[y * w + x] = ((n + s + e + west + center) >> 2) as i16;
            }
        }
        layer.copy_from_slice(&scratch);
    }
}

// ─── propagate_effects ───────────────────────────────────────────────────────

/// Rebuild effect maps from scratch using current entity state.
///
/// 1. Clear all layers.
/// 2. For each completed, enabled entity: iterate its `effects` vec.
/// 3. For each `Effect`, write `value` at entity origin; attenuate linearly
///    out to `radius` tiles (manhattan diamond).
/// 4. Apply `smooth_pass` to FireProtection and PoliceProtection layers
///    (3 passes each), matching SimCity `DoSPZone` + `DoSmooth` behaviour.
pub fn propagate_effects(
    effect_map: &mut EffectMap,
    entities: &EntityStore,
    registry: &ArchetypeRegistry,
) {
    effect_map.clear();

    let w = effect_map.width;
    let h = effect_map.height;

    for handle in entities.iter_alive() {
        let flags = match entities.get_flags(handle) {
            Some(f) => f,
            None => continue,
        };
        if flags.contains(StatusFlags::UNDER_CONSTRUCTION) {
            continue;
        }
        let enabled = entities.get_enabled(handle).unwrap_or(true);
        if !enabled {
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
        let pos = match entities.get_pos(handle) {
            Some(p) => p,
            None => continue,
        };

        // Also synthesise effects from legacy scalar fields for backwards compat.
        let mut legacy: Vec<crate::core::archetypes::Effect> = Vec::new();
        if def.pollution > 0 {
            legacy.push(crate::core::archetypes::Effect {
                kind: EffectKind::Pollution,
                value: def.pollution as i32,
                radius: (def.pollution).saturating_mul(2),
            });
        }
        if def.noise > 0 {
            legacy.push(crate::core::archetypes::Effect {
                kind: EffectKind::Noise,
                value: def.noise as i32,
                radius: def.noise,
            });
        }
        if def.desirability_radius > 0 && def.desirability_magnitude != 0 {
            legacy.push(crate::core::archetypes::Effect {
                kind: EffectKind::LandValue,
                value: def.desirability_magnitude as i32,
                radius: def.desirability_radius,
            });
        }

        let all_effects = def.effects.iter().copied().chain(legacy.into_iter());

        for effect in all_effects {
            let r = effect.radius as i32;
            let ox = pos.x as i32;
            let oy = pos.y as i32;

            for dy in -r..=r {
                let rem = r - dy.abs();
                for dx in -rem..=rem {
                    let tx = ox + dx;
                    let ty = oy + dy;
                    if tx < 0 || ty < 0 || tx >= w as i32 || ty >= h as i32 {
                        continue;
                    }
                    // Linear attenuation: full value at centre, zero at edge.
                    let dist = dx.abs() + dy.abs();
                    let att = if r == 0 {
                        effect.value
                    } else {
                        effect.value * (r - dist + 1) / (r + 1)
                    };
                    effect_map.add(effect.kind, tx as u32, ty as u32, att);
                }
            }
        }
    }

    // Smooth fire/police protection layers (SimCity DoSmooth x3 equivalent).
    let fire_layer = &mut effect_map.maps[EffectKind::FireProtection as usize];
    smooth_pass(fire_layer, w, h, 3);
    let police_layer = &mut effect_map.maps[EffectKind::PoliceProtection as usize];
    smooth_pass(police_layer, w, h, 3);
}

// ─── compute_crime_map ────────────────────────────────────────────────────────

/// Compute the crime overlay using the SimCity CrimeScan formula.
///
/// `crime = clamp(128 − land_value + pop_density − police_protection, 0, 300)`
///
/// Only computed for tiles where `land_value > 0` (i.e. tiles with road access).
/// Writes directly into `effect_map.maps[EffectKind::Crime]`.
pub fn compute_crime_map(
    effect_map: &mut EffectMap,
    land_value: &[i16],
    pop_density: &[u16],
) {
    let len = effect_map.len();
    let police = effect_map.maps[EffectKind::PoliceProtection as usize].clone();
    let crime_layer = &mut effect_map.maps[EffectKind::Crime as usize];
    for i in 0..len.min(land_value.len()).min(pop_density.len()) {
        let lv = land_value[i].max(0) as i32;
        if lv == 0 {
            // No road access / undeveloped — no crime contribution.
            crime_layer[i] = 0;
            continue;
        }
        let z = (128 - lv)
            + pop_density[i] as i32
            - police[i] as i32;
        crime_layer[i] = z.clamp(0, 300) as i16;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::{ArchetypeDefinition, ArchetypeRegistry, ArchetypeTag, Effect};
    use crate::core::entity::EntityStore;
    use crate::core_types::*;

    fn make_industrial(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: "Factory".to_string(),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 80,
            floors: 1,
            usable_ratio_pct: 90,
            base_cost_cents: 200_000,
            base_upkeep_cents_per_tick: 20,
            power_demand_kw: 50,
            power_supply_kw: 0,
            water_demand: 10,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 20,
            noise: 10,
            build_time_ticks: 1,
            max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 20,
            living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    fn make_police_station(id: ArchetypeId) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id,
            name: "Police Station".to_string(),
            tags: vec![ArchetypeTag::Civic, ArchetypeTag::Service],
            footprint_w: 2,
            footprint_h: 2,
            coverage_ratio_pct: 60,
            floors: 2,
            usable_ratio_pct: 80,
            base_cost_cents: 300_000,
            base_upkeep_cents_per_tick: 30,
            power_demand_kw: 10,
            power_supply_kw: 0,
            water_demand: 5,
            water_supply: 0,
            water_coverage_radius: 0,
            is_water_pipe: false,
            service_radius: 8,
            desirability_radius: 0,
            desirability_magnitude: 0,
            pollution: 0,
            noise: 5,
            build_time_ticks: 1,
            max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 25,
            living_space_per_person_m2: 0,
            effects: vec![Effect {
                kind: EffectKind::PoliceProtection,
                value: 500,
                radius: 8,
            }],
        }
    }

    #[test]
    fn effect_map_new_zeroed() {
        let em = EffectMap::new(8, 8);
        assert_eq!(em.get(EffectKind::Pollution, 0, 0), 0);
        assert_eq!(em.get(EffectKind::Crime, 4, 4), 0);
    }

    #[test]
    fn effect_map_add_and_get() {
        let mut em = EffectMap::new(8, 8);
        em.add(EffectKind::Pollution, 2, 3, 50);
        assert_eq!(em.get(EffectKind::Pollution, 2, 3), 50);
        em.add(EffectKind::Pollution, 2, 3, 30);
        assert_eq!(em.get(EffectKind::Pollution, 2, 3), 80);
    }

    #[test]
    fn effect_map_clear() {
        let mut em = EffectMap::new(8, 8);
        em.add(EffectKind::LandValue, 1, 1, 100);
        em.clear();
        assert_eq!(em.get(EffectKind::LandValue, 1, 1), 0);
    }

    #[test]
    fn smooth_pass_spreads_value() {
        let w = 5u32;
        let h = 5u32;
        let mut layer = vec![0i16; (w * h) as usize];
        // Set centre tile to 100
        layer[(2 * w + 2) as usize] = 100;
        smooth_pass(&mut layer, w, h, 1);
        // Centre neighbours should have non-zero values after smoothing
        assert!(layer[(1 * w + 2) as usize] > 0); // north
        assert!(layer[(3 * w + 2) as usize] > 0); // south
    }

    #[test]
    fn propagate_effects_pollution_written() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_industrial(1));
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 4, 4, 0).unwrap();
        // Mark as completed (not under construction)
        entities.set_flags(h, StatusFlags::NONE);
        let mut em = EffectMap::new(16, 16);
        propagate_effects(&mut em, &entities, &registry);
        // Origin tile should have pollution
        assert!(em.get(EffectKind::Pollution, 4, 4) > 0);
        // Adjacent tile within radius (pollution=20, radius=40 → huge radius, but map is 16)
        assert!(em.get(EffectKind::Pollution, 5, 4) > 0);
    }

    #[test]
    fn propagate_effects_police_protection_smoothed() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_police_station(1));
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 4, 4, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        let mut em = EffectMap::new(16, 16);
        propagate_effects(&mut em, &entities, &registry);
        // Police protection should be spread around the station
        assert!(em.get(EffectKind::PoliceProtection, 4, 4) > 0);
    }

    #[test]
    fn propagate_effects_skips_under_construction() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_industrial(1));
        let mut entities = EntityStore::new(64);
        // Entity starts UNDER_CONSTRUCTION by default
        entities.alloc(1, 4, 4, 0).unwrap();
        let mut em = EffectMap::new(16, 16);
        propagate_effects(&mut em, &entities, &registry);
        assert_eq!(em.get(EffectKind::Pollution, 4, 4), 0);
    }

    #[test]
    fn compute_crime_map_basic() {
        let mut em = EffectMap::new(4, 4);
        let land_value = vec![50i16; 16]; // non-zero = road access
        let pop_density = vec![20u16; 16];
        compute_crime_map(&mut em, &land_value, &pop_density);
        // crime = clamp(128 - 50 + 20 - 0, 0, 300) = 98
        assert_eq!(em.get(EffectKind::Crime, 0, 0), 98);
    }

    #[test]
    fn compute_crime_map_zero_land_value_skipped() {
        let mut em = EffectMap::new(4, 4);
        let land_value = vec![0i16; 16]; // zero = no road access
        let pop_density = vec![50u16; 16];
        compute_crime_map(&mut em, &land_value, &pop_density);
        assert_eq!(em.get(EffectKind::Crime, 0, 0), 0);
    }
}
