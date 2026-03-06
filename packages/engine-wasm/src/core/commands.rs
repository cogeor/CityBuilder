//! Command types and processor for player actions.
//!
//! Commands are the sole entry point for mutating canonical state
//! outside the simulation tick. Each command is validated before
//! application; invalid commands are rejected with an error.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::commands_spec;
use crate::core::math_util::rects_overlap;
use crate::core::network::{RoadGraph, RoadType};
use crate::core::tilemap::TileMap;
use crate::core::world::{CityPolicies, WorldState};
use crate::core_types::*;
use crate::sim::speed::SimSpeed;
use serde::{Deserialize, Serialize};

/// A player command that mutates the canonical world state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    PlaceEntity {
        archetype_id: ArchetypeId,
        x: i16,
        y: i16,
        rotation: u8,
    },
    RemoveEntity {
        handle: EntityHandle,
    },
    UpgradeEntity {
        handle: EntityHandle,
        target_level: u8,
    },
    SetPolicy {
        key: PolicyKey,
        value: i32,
    },
    Bulldoze {
        x: i16,
        y: i16,
        w: u8,
        h: u8,
    },
    ToggleEntity {
        handle: EntityHandle,
        enabled: bool,
    },
    SetZoning {
        x: i16,
        y: i16,
        w: u8,
        h: u8,
        zone: ZoneType,
        /// Density tier to paint; defaults to Low when absent.
        #[serde(default)]
        density: ZoneDensity,
    },
    SetTerrain {
        x: i16,
        y: i16,
        w: u8,
        h: u8,
        terrain: TerrainType,
    },
    SetRoadLine {
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        road_type: RoadType,
    },
    RemoveRoad {
        x: i16,
        y: i16,
    },
    /// Pause or change the simulation speed.
    SetSimSpeed { speed: SimSpeed },
}

/// Policy keys that can be changed via SetPolicy command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PolicyKey {
    ResidentialTax = 0,
    CommercialTax = 1,
    IndustrialTax = 2,
    PoliceBudget = 3,
    FireBudget = 4,
    HealthBudget = 5,
    EducationBudget = 6,
    TransportBudget = 7,
}

/// Errors returned when command validation fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandError {
    OutOfBounds,
    TileOccupied,
    InvalidEntity,
    InsufficientFunds,
    InvalidValue,
    /// Generic validation failure with a descriptive message.
    ValidationFailed(String),
    /// Archetype ID is not registered in the active registry.
    InvalidArchetype,
    /// Tile zone does not match the zone required by the archetype.
    WrongZone,
    /// Terrain type prevents construction (e.g. water, mountain).
    TerrainNotBuildable,
    /// Building requires an adjacent road but none is present.
    NoRoadAccess,
}

/// Result of command application.
pub type CommandResult = Result<CommandEffect, CommandError>;

/// What changed as a result of a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandEffect {
    EntityPlaced { handle: EntityHandle },
    EntityRemoved { handle: EntityHandle },
    EntityUpgraded { handle: EntityHandle, new_level: u8 },
    PolicyChanged { key: PolicyKey, old_value: i32, new_value: i32 },
    TilesBulldozed { count: u32 },
    EntityToggled { handle: EntityHandle, enabled: bool },
    ZoningApplied { count: u32 },
    TerrainApplied { count: u32 },
    RoadLineApplied { count: u32 },
    RoadRemoved { x: i16, y: i16 },
    /// Simulation speed was changed.
    SimSpeedChanged { speed: SimSpeed },
}

/// Apply a command to the world state after validation.
pub fn apply_command(world: &mut WorldState, cmd: &Command) -> CommandResult {
    apply_command_with_registry(world, None, None, cmd)
}

/// Apply a command with registry-aware validation and footprint logic.
pub fn apply_command_with_registry(
    world: &mut WorldState,
    registry: Option<&ArchetypeRegistry>,
    road_graph: Option<&mut RoadGraph>,
    cmd: &Command,
) -> CommandResult {
    commands_spec::validate_command_with_registry(world, registry, cmd)?;

    match cmd {
        Command::PlaceEntity {
            archetype_id,
            x,
            y,
            rotation,
        } => {
            use crate::core::archetypes::ArchetypeTag;
            use crate::core::tilemap::TileFlags;

            if let Some(registry) = registry {
                if let Some(def) = registry.get(*archetype_id) {
                    world.treasury -= def.cost_at_level(1);
                }
            }
            // Place entity
            let handle = match world.place_entity(*archetype_id, *x, *y, *rotation) {
                Some(h) => h,
                None => return Err(CommandError::TileOccupied),
            };
            // Stamp CONDUCTOR on all tiles of the footprint when archetype is a power line.
            if let Some(registry) = registry {
                if let Some(def) = registry.get(*archetype_id) {
                    if def.has_tag(ArchetypeTag::PowerLine) {
                        for dy in 0..def.footprint_h as i16 {
                            for dx in 0..def.footprint_w as i16 {
                                let tx = (*x + dx) as u32;
                                let ty = (*y + dy) as u32;
                                world.tiles.set_flags(tx, ty, TileFlags::CONDUCTOR);
                            }
                        }
                    }
                }
            }
            Ok(CommandEffect::EntityPlaced { handle })
        }

        Command::RemoveEntity { handle } => {
            world.remove_entity(*handle);
            Ok(CommandEffect::EntityRemoved { handle: *handle })
        }

        Command::UpgradeEntity {
            handle,
            target_level,
        } => {
            if *target_level == 0 {
                return Err(CommandError::InvalidValue);
            }
            world.entities.set_level(*handle, *target_level);
            Ok(CommandEffect::EntityUpgraded {
                handle: *handle,
                new_level: *target_level,
            })
        }

        Command::SetPolicy { key, value } => {
            let clamped = (*value).clamp(0, 200) as u8;
            let old_value = get_policy_value(&world.policies, *key) as i32;
            set_policy_value(&mut world.policies, *key, clamped);
            Ok(CommandEffect::PolicyChanged {
                key: *key,
                old_value,
                new_value: clamped as i32,
            })
        }

        Command::Bulldoze { x, y, w, h } => {
            // Remove all entities in the rectangle
            let mut removed = 0u32;
            let handles: Vec<EntityHandle> = world.entities.iter_alive().collect();
            for handle in handles {
                if let Some(pos) = world.entities.get_pos(handle) {
                    let (fw, fh) = registry
                        .and_then(|reg| {
                            world
                                .entities
                                .get_archetype(handle)
                                .and_then(|id| reg.get(id))
                                .map(|def| (def.footprint_w as i16, def.footprint_h as i16))
                        })
                        .unwrap_or((1, 1));
                    if rects_overlap(
                        *x,
                        *y,
                        *w as i16,
                        *h as i16,
                        pos.x,
                        pos.y,
                        fw,
                        fh,
                    )
                    {
                        world.entities.free(handle);
                        removed += 1;
                    }
                }
            }
            // Clear zoning in the rectangle
            for dy in 0..*h as i16 {
                for dx in 0..*w as i16 {
                    let cx = *x + dx;
                    let cy = *y + dy;
                    if cx >= 0 && cy >= 0 {
                        world.tiles.set_zone(cx as u32, cy as u32, ZoneType::None);
                    }
                }
            }
            Ok(CommandEffect::TilesBulldozed { count: removed })
        }

        Command::ToggleEntity { handle, enabled } => {
            world.entities.set_enabled(*handle, *enabled);
            Ok(CommandEffect::EntityToggled {
                handle: *handle,
                enabled: *enabled,
            })
        }

        Command::SetZoning { x, y, w, h, zone, density } => {
            let mut count = 0u32;
            for dy in 0..*h as i16 {
                for dx in 0..*w as i16 {
                    let cx = *x + dx;
                    let cy = *y + dy;
                    // Skip water tiles — zone paint must not apply to non-buildable terrain
                    if cx < 0 || cy < 0 || !world.is_buildable(cx, cy) {
                        continue;
                    }
                    if world.tiles.set_zone(cx as u32, cy as u32, *zone) {
                        world.tiles.set_density(cx as u32, cy as u32, *density);
                        count += 1;
                    }
                }
            }
            // City-builder UX default: any zoning rect (even 1 tile wide) creates
            // collector roads on the perimeter to keep new districts connected.
            // Changed from `w >= 4 && h >= 4` to fire for any valid rect.
            if *w >= 1 || *h >= 1 {
                let Some(road_graph) = road_graph else {
                    return Ok(CommandEffect::ZoningApplied { count });
                };
                let x_min = *x;
                let y_min = *y;
                let x_max = *x + (*w as i16) - 1;
                let y_max = *y + (*h as i16) - 1;
                let road_type = RoadType::Local;

                for tx in x_min..x_max {
                    try_add_road_segment(world, road_graph, tx, y_min, tx + 1, y_min, road_type);
                    try_add_road_segment(world, road_graph, tx, y_max, tx + 1, y_max, road_type);
                }
                for ty in y_min..y_max {
                    try_add_road_segment(world, road_graph, x_min, ty, x_min, ty + 1, road_type);
                    try_add_road_segment(world, road_graph, x_max, ty, x_max, ty + 1, road_type);
                }
            }
            Ok(CommandEffect::ZoningApplied { count })
        }
        Command::SetTerrain {
            x,
            y,
            w,
            h,
            terrain,
        } => {
            let mut count = 0u32;
            for dy in 0..*h as i16 {
                for dx in 0..*w as i16 {
                    let cx = *x + dx;
                    let cy = *y + dy;
                    if cx >= 0 && cy >= 0 && world.tiles.set_terrain(cx as u32, cy as u32, *terrain) {
                        count += 1;
                    }
                }
            }
            Ok(CommandEffect::TerrainApplied { count })
        }
        Command::SetRoadLine {
            x0,
            y0,
            x1,
            y1,
            road_type,
        } => {
            let Some(road_graph) = road_graph else {
                return Err(CommandError::ValidationFailed(
                    "road graph unavailable".to_string(),
                ));
            };
            let mut count = 0u32;
            let mut prev = TileCoord::new(*x0, *y0);

            if x0 == x1 {
                let min_y = (*y0).min(*y1);
                let max_y = (*y0).max(*y1);
                for y in (min_y + 1)..=max_y {
                    let next = TileCoord::new(*x0, y);
                    if try_add_road_segment(
                        world,
                        road_graph,
                        prev.x,
                        prev.y,
                        next.x,
                        next.y,
                        *road_type,
                    ) {
                        count += 1;
                    }
                    prev = next;
                }
            } else {
                let min_x = (*x0).min(*x1);
                let max_x = (*x0).max(*x1);
                for x in (min_x + 1)..=max_x {
                    let next = TileCoord::new(x, *y0);
                    if try_add_road_segment(
                        world,
                        road_graph,
                        prev.x,
                        prev.y,
                        next.x,
                        next.y,
                        *road_type,
                    ) {
                        count += 1;
                    }
                    prev = next;
                }
            }

            Ok(CommandEffect::RoadLineApplied { count })
        }

        Command::SetSimSpeed { speed } => {
            // Speed is engine-level state; the actual change is applied by
            // SimulationEngine::apply_command which intercepts this command
            // before reaching here. When called via the bare world API,
            // return the effect with no world-state mutation.
            Ok(CommandEffect::SimSpeedChanged { speed: *speed })
        }

        Command::RemoveRoad { x, y } => {
            use crate::core::tilemap::{TileFlags, TileKind};

            let Some(road_graph) = road_graph else {
                return Err(CommandError::ValidationFailed(
                    "road graph unavailable".to_string(),
                ));
            };

            let pos = TileCoord::new(*x, *y);

            // Collect all neighbors of this road node before removing edges.
            let neighbors: Vec<TileCoord> = road_graph
                .neighbors(pos)
                .iter()
                .map(|(n, _)| *n)
                .collect();

            // Remove all edges from this position.
            for neighbor in &neighbors {
                road_graph.remove_segment(pos, *neighbor);
            }

            // Clear CONDUCTOR flag and tile kind on the removed road tile.
            if *x >= 0 && *y >= 0 && world.tiles.in_bounds(*x as u32, *y as u32) {
                world.tiles.clear_flags(*x as u32, *y as u32, TileFlags::CONDUCTOR);
                if let Some(t) = world.tiles.get_mut(*x as u32, *y as u32) {
                    if t.kind == TileKind::Road {
                        t.kind = TileKind::Empty;
                    }
                }
            }

            // Update ROAD_ACCESS on the four cardinal neighbors.
            update_road_access(&mut world.tiles, road_graph, *x, *y);

            // Also update ROAD_ACCESS for all former neighbors (their connectivity changed).
            for neighbor in &neighbors {
                update_road_access(&mut world.tiles, road_graph, neighbor.x, neighbor.y);
            }

            Ok(CommandEffect::RoadRemoved { x: *x, y: *y })
        }
    }
}

fn try_add_road_segment(
    world: &mut WorldState,
    road_graph: &mut RoadGraph,
    ax: i16,
    ay: i16,
    bx: i16,
    by: i16,
    road_type: RoadType,
) -> bool {
    use crate::core::tilemap::TileFlags;

    if ax < 0 || ay < 0 || !world.tiles.in_bounds(ax as u32, ay as u32)
        || bx < 0 || by < 0 || !world.tiles.in_bounds(bx as u32, by as u32)
    {
        return false;
    }
    if !world.is_buildable(ax, ay) || !world.is_buildable(bx, by) {
        return false;
    }
    if road_graph.add_segment(TileCoord::new(ax, ay), TileCoord::new(bx, by), road_type) {
        world.tiles.set_flags(ax as u32, ay as u32, TileFlags::CONDUCTOR);
        world.tiles.set_flags(bx as u32, by as u32, TileFlags::CONDUCTOR);
        update_road_access(&mut world.tiles, road_graph, ax, ay);
        update_road_access(&mut world.tiles, road_graph, bx, by);
        true
    } else {
        false
    }
}

/// Update the `ROAD_ACCESS` flag on all four cardinal neighbors of `(x, y)`.
///
/// Sets `ROAD_ACCESS` on a neighbor when `road_graph.has_road_access` returns true
/// for that neighbor's position, clears it when false.
fn update_road_access(tiles: &mut TileMap, road_graph: &RoadGraph, x: i16, y: i16) {
    use crate::core::tilemap::TileFlags;

    let neighbors: [(i16, i16); 4] = [
        (x,     y - 1), // N
        (x,     y + 1), // S
        (x + 1, y    ), // E
        (x - 1, y    ), // W
    ];

    for (nx, ny) in neighbors {
        if nx < 0 || ny < 0 {
            continue;
        }
        let (unx, uny) = (nx as u32, ny as u32);
        if !tiles.in_bounds(unx, uny) {
            continue;
        }
        if road_graph.has_road_access(nx, ny) {
            tiles.set_flags(unx, uny, TileFlags::ROAD_ACCESS);
        } else {
            tiles.clear_flags(unx, uny, TileFlags::ROAD_ACCESS);
        }
    }
}

/// Get the current value of a policy.
fn get_policy_value(policies: &CityPolicies, key: PolicyKey) -> u8 {
    match key {
        PolicyKey::ResidentialTax => policies.residential_tax_pct,
        PolicyKey::CommercialTax => policies.commercial_tax_pct,
        PolicyKey::IndustrialTax => policies.industrial_tax_pct,
        PolicyKey::PoliceBudget => policies.police_budget_pct,
        PolicyKey::FireBudget => policies.fire_budget_pct,
        PolicyKey::HealthBudget => policies.health_budget_pct,
        PolicyKey::EducationBudget => policies.education_budget_pct,
        PolicyKey::TransportBudget => policies.transport_budget_pct,
    }
}

/// Set a policy value.
fn set_policy_value(policies: &mut CityPolicies, key: PolicyKey, value: u8) {
    match key {
        PolicyKey::ResidentialTax => policies.residential_tax_pct = value,
        PolicyKey::CommercialTax => policies.commercial_tax_pct = value,
        PolicyKey::IndustrialTax => policies.industrial_tax_pct = value,
        PolicyKey::PoliceBudget => policies.police_budget_pct = value,
        PolicyKey::FireBudget => policies.fire_budget_pct = value,
        PolicyKey::HealthBudget => policies.health_budget_pct = value,
        PolicyKey::EducationBudget => policies.education_budget_pct = value,
        PolicyKey::TransportBudget => policies.transport_budget_pct = value,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network::RoadGraph;
    use crate::core::tilemap::TileFlags;
    use crate::core::world::WorldState;

    fn make_world() -> WorldState {
        WorldState::new(MapSize::new(16, 16), 42)
    }

    // ── PlaceEntity ─────────────────────────────────────────────────────

    #[test]
    fn place_entity_success() {
        let mut world = make_world();
        let cmd = Command::PlaceEntity {
            archetype_id: 1,
            x: 5,
            y: 5,
            rotation: 0,
        };
        let result = apply_command(&mut world, &cmd);
        assert!(result.is_ok());
        match result.unwrap() {
            CommandEffect::EntityPlaced { handle } => {
                assert!(world.entities.is_valid(handle));
                assert_eq!(world.entities.get_archetype(handle), Some(1));
                assert_eq!(world.entities.get_pos(handle), Some(TileCoord::new(5, 5)));
            }
            _ => panic!("Expected EntityPlaced effect"),
        }
        assert_eq!(world.entities.count(), 1);
    }

    #[test]
    fn place_entity_out_of_bounds() {
        let mut world = make_world();
        let cmd = Command::PlaceEntity {
            archetype_id: 1,
            x: 16,
            y: 0,
            rotation: 0,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn place_entity_out_of_bounds_negative() {
        let mut world = make_world();
        let cmd = Command::PlaceEntity {
            archetype_id: 1,
            x: -1,
            y: 0,
            rotation: 0,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn place_entity_on_water_terrain_not_buildable() {
        let mut world = make_world();
        world.tiles.set_terrain(3, 3, TerrainType::Water);
        let cmd = Command::PlaceEntity {
            archetype_id: 1,
            x: 3,
            y: 3,
            rotation: 0,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::TerrainNotBuildable));
    }

    // ── RemoveEntity ────────────────────────────────────────────────────

    #[test]
    fn remove_entity_success() {
        let mut world = make_world();
        let handle = world.place_entity(1, 5, 5, 0).unwrap();
        let cmd = Command::RemoveEntity { handle };
        let result = apply_command(&mut world, &cmd);
        assert!(result.is_ok());
        match result.unwrap() {
            CommandEffect::EntityRemoved { handle: h } => {
                assert_eq!(h, handle);
            }
            _ => panic!("Expected EntityRemoved effect"),
        }
        assert!(!world.entities.is_valid(handle));
        assert_eq!(world.entities.count(), 0);
    }

    #[test]
    fn remove_entity_invalid_handle() {
        let mut world = make_world();
        let cmd = Command::RemoveEntity {
            handle: EntityHandle::INVALID,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::InvalidEntity));
    }

    #[test]
    fn remove_entity_stale_handle() {
        let mut world = make_world();
        let handle = world.place_entity(1, 5, 5, 0).unwrap();
        world.remove_entity(handle);
        let cmd = Command::RemoveEntity { handle };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::InvalidEntity));
    }

    // ── UpgradeEntity ───────────────────────────────────────────────────

    #[test]
    fn upgrade_entity_success() {
        let mut world = make_world();
        let handle = world.place_entity(1, 5, 5, 0).unwrap();
        let cmd = Command::UpgradeEntity {
            handle,
            target_level: 3,
        };
        let result = apply_command(&mut world, &cmd);
        assert!(result.is_ok());
        match result.unwrap() {
            CommandEffect::EntityUpgraded {
                handle: h,
                new_level,
            } => {
                assert_eq!(h, handle);
                assert_eq!(new_level, 3);
            }
            _ => panic!("Expected EntityUpgraded effect"),
        }
        assert_eq!(world.entities.get_level(handle), Some(3));
    }

    #[test]
    fn upgrade_entity_invalid_handle() {
        let mut world = make_world();
        let cmd = Command::UpgradeEntity {
            handle: EntityHandle::INVALID,
            target_level: 2,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::InvalidEntity));
    }

    #[test]
    fn upgrade_entity_level_zero_invalid_value() {
        let mut world = make_world();
        let handle = world.place_entity(1, 5, 5, 0).unwrap();
        let cmd = Command::UpgradeEntity {
            handle,
            target_level: 0,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::InvalidValue));
    }

    // ── SetPolicy ───────────────────────────────────────────────────────

    #[test]
    fn set_policy_success() {
        let mut world = make_world();
        let cmd = Command::SetPolicy {
            key: PolicyKey::ResidentialTax,
            value: 15,
        };
        let result = apply_command(&mut world, &cmd);
        assert!(result.is_ok());
        match result.unwrap() {
            CommandEffect::PolicyChanged {
                key,
                old_value,
                new_value,
            } => {
                assert_eq!(key, PolicyKey::ResidentialTax);
                assert_eq!(old_value, 9); // default
                assert_eq!(new_value, 15);
            }
            _ => panic!("Expected PolicyChanged effect"),
        }
        assert_eq!(world.policies.residential_tax_pct, 15);
    }

    #[test]
    fn set_policy_clamps_negative_to_zero() {
        let mut world = make_world();
        let cmd = Command::SetPolicy {
            key: PolicyKey::CommercialTax,
            value: -50,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::PolicyChanged { new_value, .. } => {
                assert_eq!(new_value, 0);
            }
            _ => panic!("Expected PolicyChanged effect"),
        }
        assert_eq!(world.policies.commercial_tax_pct, 0);
    }

    #[test]
    fn set_policy_clamps_above_200() {
        let mut world = make_world();
        let cmd = Command::SetPolicy {
            key: PolicyKey::PoliceBudget,
            value: 999,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::PolicyChanged {
                old_value,
                new_value,
                ..
            } => {
                assert_eq!(old_value, 100); // default
                assert_eq!(new_value, 200);
            }
            _ => panic!("Expected PolicyChanged effect"),
        }
        assert_eq!(world.policies.police_budget_pct, 200);
    }

    #[test]
    fn set_policy_old_value_recorded() {
        let mut world = make_world();
        // Set initial value
        world.policies.fire_budget_pct = 75;
        let cmd = Command::SetPolicy {
            key: PolicyKey::FireBudget,
            value: 120,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::PolicyChanged {
                old_value,
                new_value,
                ..
            } => {
                assert_eq!(old_value, 75);
                assert_eq!(new_value, 120);
            }
            _ => panic!("Expected PolicyChanged effect"),
        }
    }

    #[test]
    fn set_policy_all_keys() {
        let mut world = make_world();
        let keys = [
            (PolicyKey::ResidentialTax, 10),
            (PolicyKey::CommercialTax, 20),
            (PolicyKey::IndustrialTax, 30),
            (PolicyKey::PoliceBudget, 50),
            (PolicyKey::FireBudget, 60),
            (PolicyKey::HealthBudget, 70),
            (PolicyKey::EducationBudget, 80),
            (PolicyKey::TransportBudget, 90),
        ];
        for (key, value) in keys {
            let cmd = Command::SetPolicy { key, value };
            assert!(apply_command(&mut world, &cmd).is_ok());
        }
        assert_eq!(world.policies.residential_tax_pct, 10);
        assert_eq!(world.policies.commercial_tax_pct, 20);
        assert_eq!(world.policies.industrial_tax_pct, 30);
        assert_eq!(world.policies.police_budget_pct, 50);
        assert_eq!(world.policies.fire_budget_pct, 60);
        assert_eq!(world.policies.health_budget_pct, 70);
        assert_eq!(world.policies.education_budget_pct, 80);
        assert_eq!(world.policies.transport_budget_pct, 90);
    }

    // ── Bulldoze ────────────────────────────────────────────────────────

    #[test]
    fn bulldoze_removes_entities_in_rect() {
        let mut world = make_world();
        let h1 = world.place_entity(1, 2, 2, 0).unwrap();
        let h2 = world.place_entity(2, 3, 3, 0).unwrap();
        let h3 = world.place_entity(3, 10, 10, 0).unwrap(); // outside rect

        let cmd = Command::Bulldoze {
            x: 2,
            y: 2,
            w: 3,
            h: 3,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::TilesBulldozed { count } => {
                assert_eq!(count, 2);
            }
            _ => panic!("Expected TilesBulldozed effect"),
        }
        assert!(!world.entities.is_valid(h1));
        assert!(!world.entities.is_valid(h2));
        assert!(world.entities.is_valid(h3));
    }

    #[test]
    fn bulldoze_clears_zoning() {
        let mut world = make_world();
        world.tiles.set_zone(2, 2, ZoneType::Residential);
        world.tiles.set_zone(3, 3, ZoneType::Commercial);

        let cmd = Command::Bulldoze {
            x: 2,
            y: 2,
            w: 3,
            h: 3,
        };
        apply_command(&mut world, &cmd).unwrap();

        assert_eq!(world.tiles.get(2, 2).unwrap().zone, ZoneType::None);
        assert_eq!(world.tiles.get(3, 3).unwrap().zone, ZoneType::None);
    }

    #[test]
    fn bulldoze_empty_rect_returns_zero() {
        let mut world = make_world();
        let cmd = Command::Bulldoze {
            x: 0,
            y: 0,
            w: 3,
            h: 3,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::TilesBulldozed { count } => {
                assert_eq!(count, 0);
            }
            _ => panic!("Expected TilesBulldozed effect"),
        }
    }

    #[test]
    fn bulldoze_out_of_bounds() {
        let mut world = make_world();
        let cmd = Command::Bulldoze {
            x: -1,
            y: 0,
            w: 3,
            h: 3,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::OutOfBounds));
    }

    // ── ToggleEntity ────────────────────────────────────────────────────

    #[test]
    fn toggle_entity_success() {
        let mut world = make_world();
        let handle = world.place_entity(1, 5, 5, 0).unwrap();
        assert_eq!(world.entities.get_enabled(handle), Some(true));

        let cmd = Command::ToggleEntity {
            handle,
            enabled: false,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::EntityToggled { handle: h, enabled } => {
                assert_eq!(h, handle);
                assert!(!enabled);
            }
            _ => panic!("Expected EntityToggled effect"),
        }
        assert_eq!(world.entities.get_enabled(handle), Some(false));
    }

    #[test]
    fn toggle_entity_invalid_handle() {
        let mut world = make_world();
        let cmd = Command::ToggleEntity {
            handle: EntityHandle::INVALID,
            enabled: false,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::InvalidEntity));
    }

    // ── SetZoning ───────────────────────────────────────────────────────

    #[test]
    fn set_zoning_success() {
        let mut world = make_world();
        let cmd = Command::SetZoning {
            x: 2,
            y: 2,
            w: 3,
            h: 3,
            zone: ZoneType::Residential,
            density: ZoneDensity::Low,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::ZoningApplied { count } => {
                assert_eq!(count, 9); // 3x3
            }
            _ => panic!("Expected ZoningApplied effect"),
        }
        // Verify all tiles in range are zoned
        for dy in 0..3u32 {
            for dx in 0..3u32 {
                assert_eq!(
                    world.tiles.get(2 + dx, 2 + dy).unwrap().zone,
                    ZoneType::Residential
                );
            }
        }
    }

    #[test]
    fn set_zoning_out_of_bounds() {
        let mut world = make_world();
        let cmd = Command::SetZoning {
            x: 20,
            y: 20,
            w: 1,
            h: 1,
            zone: ZoneType::Industrial,
            density: ZoneDensity::Low,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn set_zoning_partial_edge_counts_in_bounds_only() {
        let mut world = make_world();
        // Start at (14,14) with 4x4 rect on a 16x16 map: only 2x2 in bounds
        let cmd = Command::SetZoning {
            x: 14,
            y: 14,
            w: 4,
            h: 4,
            zone: ZoneType::Commercial,
            density: ZoneDensity::Low,
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::ZoningApplied { count } => {
                // Only (14,14), (15,14), (14,15), (15,15) are in bounds
                assert_eq!(count, 4);
            }
            _ => panic!("Expected ZoningApplied effect"),
        }
    }

    // ── SetZoning road perimeter ────────────────────────────────────────

    #[test]
    fn set_zoning_small_zone_creates_perimeter_roads() {
        // w=3, h=3 → the zoning code auto-creates perimeter roads for any size
        let mut world = make_world();
        let mut road_graph = crate::core::network::RoadGraph::new();
        let cmd = Command::SetZoning {
            x: 4, y: 4, w: 3, h: 3,
            zone: ZoneType::Residential,
            density: ZoneDensity::Low,
        };
        apply_command_with_registry(&mut world, None, Some(&mut road_graph), &cmd).unwrap();
        // The road graph should have at least one segment after zoning
        assert!(road_graph.edge_count() > 0, "auto-perimeter road should be created for any size zone");
    }

    #[test]
    fn set_zoning_perimeter_roads_idempotent() {
        let mut world = make_world();
        let mut road_graph = crate::core::network::RoadGraph::new();
        let cmd = Command::SetZoning {
            x: 2, y: 2, w: 6, h: 6,
            zone: ZoneType::Residential,
            density: ZoneDensity::Low,
        };
        // Apply twice
        apply_command_with_registry(&mut world, None, Some(&mut road_graph), &cmd).unwrap();
        let count_after_first = road_graph.edge_count();
        apply_command_with_registry(&mut world, None, Some(&mut road_graph), &cmd).unwrap();
        let count_after_second = road_graph.edge_count();
        assert_eq!(count_after_first, count_after_second, "road count should not double on idempotent rezoning");
    }

    // ── Round-trip: place then remove ───────────────────────────────────

    #[test]
    fn place_then_remove_round_trip() {
        let mut world = make_world();

        // Place
        let place_cmd = Command::PlaceEntity {
            archetype_id: 42,
            x: 7,
            y: 9,
            rotation: 2,
        };
        let handle = match apply_command(&mut world, &place_cmd).unwrap() {
            CommandEffect::EntityPlaced { handle } => handle,
            _ => panic!("Expected EntityPlaced"),
        };
        assert_eq!(world.entities.count(), 1);
        assert!(world.entities.is_valid(handle));

        // Remove
        let remove_cmd = Command::RemoveEntity { handle };
        match apply_command(&mut world, &remove_cmd).unwrap() {
            CommandEffect::EntityRemoved { handle: h } => {
                assert_eq!(h, handle);
            }
            _ => panic!("Expected EntityRemoved"),
        }
        assert_eq!(world.entities.count(), 0);
        assert!(!world.entities.is_valid(handle));
    }

    // ── Multiple commands in sequence ───────────────────────────────────

    #[test]
    fn multiple_commands_in_sequence() {
        let mut world = make_world();

        // 1. Place three entities
        let h1 = match apply_command(
            &mut world,
            &Command::PlaceEntity {
                archetype_id: 1,
                x: 0,
                y: 0,
                rotation: 0,
            },
        )
        .unwrap()
        {
            CommandEffect::EntityPlaced { handle } => handle,
            _ => panic!("Expected EntityPlaced"),
        };

        let h2 = match apply_command(
            &mut world,
            &Command::PlaceEntity {
                archetype_id: 2,
                x: 1,
                y: 1,
                rotation: 0,
            },
        )
        .unwrap()
        {
            CommandEffect::EntityPlaced { handle } => handle,
            _ => panic!("Expected EntityPlaced"),
        };

        let h3 = match apply_command(
            &mut world,
            &Command::PlaceEntity {
                archetype_id: 3,
                x: 10,
                y: 10,
                rotation: 0,
            },
        )
        .unwrap()
        {
            CommandEffect::EntityPlaced { handle } => handle,
            _ => panic!("Expected EntityPlaced"),
        };
        assert_eq!(world.entities.count(), 3);

        // 2. Upgrade h1
        apply_command(
            &mut world,
            &Command::UpgradeEntity {
                handle: h1,
                target_level: 5,
            },
        )
        .unwrap();
        assert_eq!(world.entities.get_level(h1), Some(5));

        // 3. Toggle h3 off
        apply_command(
            &mut world,
            &Command::ToggleEntity {
                handle: h3,
                enabled: false,
            },
        )
        .unwrap();
        assert_eq!(world.entities.get_enabled(h3), Some(false));

        // 4. Set zoning
        apply_command(
            &mut world,
            &Command::SetZoning {
                x: 4,
                y: 4,
                w: 2,
                h: 2,
                zone: ZoneType::Industrial,
                density: ZoneDensity::Low,
            },
        )
        .unwrap();
        assert_eq!(
            world.tiles.get(4, 4).unwrap().zone,
            ZoneType::Industrial
        );

        // 5. Set policy
        apply_command(
            &mut world,
            &Command::SetPolicy {
                key: PolicyKey::IndustrialTax,
                value: 12,
            },
        )
        .unwrap();
        assert_eq!(world.policies.industrial_tax_pct, 12);

        // 6. Bulldoze area containing h1 and h2
        let result = apply_command(
            &mut world,
            &Command::Bulldoze {
                x: 0,
                y: 0,
                w: 5,
                h: 5,
            },
        )
        .unwrap();
        match result {
            CommandEffect::TilesBulldozed { count } => {
                assert_eq!(count, 2); // h1 and h2
            }
            _ => panic!("Expected TilesBulldozed"),
        }
        assert!(!world.entities.is_valid(h1));
        assert!(!world.entities.is_valid(h2));
        assert!(world.entities.is_valid(h3));
        assert_eq!(world.entities.count(), 1);

        // 7. Remove the remaining entity
        apply_command(&mut world, &Command::RemoveEntity { handle: h3 }).unwrap();
        assert_eq!(world.entities.count(), 0);
    }

    // ── update_road_access via SetRoadLine ──────────────────────────────

    #[test]
    fn set_road_line_sets_road_access_on_neighbors() {
        let mut world = make_world();
        let mut road_graph = RoadGraph::new();

        // Place a road segment from (5,5) to (5,6) (vertical).
        let cmd = Command::SetRoadLine {
            x0: 5, y0: 5, x1: 5, y1: 6,
            road_type: crate::core::network::RoadType::Local,
        };
        apply_command_with_registry(&mut world, None, Some(&mut road_graph), &cmd).unwrap();

        // Tile (4,5) is a cardinal neighbor of road node (5,5): should have ROAD_ACCESS.
        let tile = world.tiles.get(4, 5).unwrap();
        assert!(tile.flags.contains(TileFlags::ROAD_ACCESS), "west neighbor of (5,5) should have ROAD_ACCESS");

        // Tile (6,5) is a cardinal neighbor of road node (5,5): should have ROAD_ACCESS.
        let tile = world.tiles.get(6, 5).unwrap();
        assert!(tile.flags.contains(TileFlags::ROAD_ACCESS), "east neighbor of (5,5) should have ROAD_ACCESS");
    }

    // ── RemoveRoad removes road and clears CONDUCTOR, updates ROAD_ACCESS ─

    #[test]
    fn remove_road_clears_conductor_and_tile_kind() {
        let mut world = make_world();
        let mut road_graph = RoadGraph::new();

        // Add a road segment.
        let set_cmd = Command::SetRoadLine {
            x0: 3, y0: 3, x1: 3, y1: 4,
            road_type: crate::core::network::RoadType::Local,
        };
        apply_command_with_registry(&mut world, None, Some(&mut road_graph), &set_cmd).unwrap();

        // Verify CONDUCTOR is set on road tiles.
        assert!(world.tiles.get(3, 3).unwrap().flags.contains(TileFlags::CONDUCTOR));
        assert!(world.tiles.get(3, 4).unwrap().flags.contains(TileFlags::CONDUCTOR));

        // Remove road at (3,3).
        let remove_cmd = Command::RemoveRoad { x: 3, y: 3 };
        let result = apply_command_with_registry(&mut world, None, Some(&mut road_graph), &remove_cmd).unwrap();
        assert_eq!(result, CommandEffect::RoadRemoved { x: 3, y: 3 });

        // CONDUCTOR flag should be cleared on (3,3).
        assert!(!world.tiles.get(3, 3).unwrap().flags.contains(TileFlags::CONDUCTOR));

        // Road graph should no longer have a node at (3,3).
        assert!(!road_graph.has_road_at(TileCoord::new(3, 3)));
    }

    #[test]
    fn remove_road_updates_road_access_on_neighbors() {
        let mut world = make_world();
        let mut road_graph = RoadGraph::new();

        // Build road at (5,5)-(5,6).
        let set_cmd = Command::SetRoadLine {
            x0: 5, y0: 5, x1: 5, y1: 6,
            road_type: crate::core::network::RoadType::Local,
        };
        apply_command_with_registry(&mut world, None, Some(&mut road_graph), &set_cmd).unwrap();

        // Neighbor (4,5) should have ROAD_ACCESS due to road node at (5,5).
        assert!(world.tiles.get(4, 5).unwrap().flags.contains(TileFlags::ROAD_ACCESS));

        // Remove road at (5,5).
        let remove_cmd = Command::RemoveRoad { x: 5, y: 5 };
        apply_command_with_registry(&mut world, None, Some(&mut road_graph), &remove_cmd).unwrap();

        // (4,5) no longer adjacent to any road — ROAD_ACCESS should be cleared.
        assert!(!world.tiles.get(4, 5).unwrap().flags.contains(TileFlags::ROAD_ACCESS));
    }

    #[test]
    fn remove_road_without_graph_returns_error() {
        let mut world = make_world();
        let cmd = Command::RemoveRoad { x: 5, y: 5 };
        let result = apply_command(&mut world, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn remove_road_out_of_bounds_returns_error() {
        let mut world = make_world();
        let mut road_graph = RoadGraph::new();
        let cmd = Command::RemoveRoad { x: -1, y: 0 };
        let result = apply_command_with_registry(&mut world, None, Some(&mut road_graph), &cmd);
        assert_eq!(result, Err(CommandError::OutOfBounds));
    }
}
