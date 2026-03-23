//! Spatial effects propagation — pollution, noise, desirability, service coverage.
//!
//! `EffectMap` stores per-tile i16 overlay data for each `EffectKind`.
//! `propagate_effects` rebuilds from entity state each call.
//! `smooth_pass` applies SimCity-style 4-neighbour weighted averaging.

use city_core::StatusFlags;
use crate::archetype::{ArchetypeRegistry, Effect, EffectKind, EFFECT_KIND_COUNT};
use city_engine::entity::EntityStore;

/// Per-tile overlay maps, one layer per `EffectKind`.
#[derive(Debug, Clone)]
pub struct EffectMap {
    pub maps: [Vec<i16>; EFFECT_KIND_COUNT],
    pub width: u32,
    pub height: u32,
}

impl EffectMap {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width * height) as usize;
        EffectMap {
            maps: std::array::from_fn(|_| vec![0i16; len]),
            width,
            height,
        }
    }

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

    #[inline]
    pub fn add(&mut self, kind: EffectKind, x: u32, y: u32, delta: i32) {
        if let Some(i) = self.idx(x, y) {
            let layer = &mut self.maps[kind as usize];
            layer[i] = (layer[i] as i32 + delta).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
    }

    #[inline]
    pub fn get(&self, kind: EffectKind, x: u32, y: u32) -> i16 {
        self.idx(x, y)
            .map(|i| self.maps[kind as usize][i])
            .unwrap_or(0)
    }
}

/// Apply N passes of 4-neighbour weighted average (SimCity DoSmooth).
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

/// Rebuild effect maps from current entity state.
///
/// Clears all layers, iterates completed entities, spreads effects
/// linearly out to radius (Manhattan diamond), then smooths service layers.
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

        // Synthesise effects from legacy scalar fields
        let mut legacy: Vec<Effect> = Vec::new();
        if def.pollution > 0 {
            legacy.push(Effect {
                kind: EffectKind::Pollution,
                value: def.pollution as i32,
                radius: (def.pollution).saturating_mul(2),
            });
        }
        if def.noise > 0 {
            legacy.push(Effect {
                kind: EffectKind::Noise,
                value: def.noise as i32,
                radius: def.noise,
            });
        }
        if def.desirability_radius > 0 && def.desirability_magnitude != 0 {
            legacy.push(Effect {
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

    // Smooth fire/police protection layers
    let fire_layer = &mut effect_map.maps[EffectKind::FireProtection as usize];
    smooth_pass(fire_layer, w, h, 3);
    let police_layer = &mut effect_map.maps[EffectKind::PoliceProtection as usize];
    smooth_pass(police_layer, w, h, 3);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::{ArchetypeDefinition, ArchetypeTag};

    fn make_industrial(id: u16) -> ArchetypeDefinition {
        ArchetypeDefinition {
            id, name: "Factory".into(),
            tags: vec![ArchetypeTag::Industrial],
            footprint_w: 2, footprint_h: 2,
            coverage_ratio_pct: 80, floors: 1, usable_ratio_pct: 90,
            base_cost_cents: 200_000, base_upkeep_cents_per_tick: 20,
            power_demand_kw: 50, power_supply_kw: 0,
            water_demand: 10, water_supply: 0,
            water_coverage_radius: 0, is_water_pipe: false,
            service_radius: 0,
            desirability_radius: 0, desirability_magnitude: 0,
            pollution: 20, noise: 10,
            build_time_ticks: 1, max_level: 1,
            prerequisites: vec![],
            workspace_per_job_m2: 20, living_space_per_person_m2: 0,
            effects: vec![],
        }
    }

    #[test]
    fn effect_map_new_zeroed() {
        let em = EffectMap::new(8, 8);
        assert_eq!(em.get(EffectKind::Pollution, 0, 0), 0);
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
        layer[(2 * w + 2) as usize] = 100;
        smooth_pass(&mut layer, w, h, 1);
        assert!(layer[(1 * w + 2) as usize] > 0);
        assert!(layer[(3 * w + 2) as usize] > 0);
    }

    #[test]
    fn propagate_effects_pollution_written() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_industrial(1));
        let mut entities = EntityStore::new(64);
        let h = entities.alloc(1, 4, 4, 0).unwrap();
        entities.set_flags(h, StatusFlags::NONE);
        let mut em = EffectMap::new(16, 16);
        propagate_effects(&mut em, &entities, &registry);
        assert!(em.get(EffectKind::Pollution, 4, 4) > 0);
    }

    #[test]
    fn propagate_effects_skips_under_construction() {
        let mut registry = ArchetypeRegistry::new();
        registry.register(make_industrial(1));
        let mut entities = EntityStore::new(64);
        entities.alloc(1, 4, 4, 0).unwrap();
        let mut em = EffectMap::new(16, 16);
        propagate_effects(&mut em, &entities, &registry);
        assert_eq!(em.get(EffectKind::Pollution, 4, 4), 0);
    }
}
