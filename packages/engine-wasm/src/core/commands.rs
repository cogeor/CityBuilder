//! Command types and processor for player actions.
//!
//! Commands are the sole entry point for mutating canonical state
//! outside the simulation tick. Each command is validated before
//! application; invalid commands are rejected with an error.

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::world::{CityPolicies, WorldState};
use crate::core::commands_spec;
use crate::core_types::*;
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
    },
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
}

/// Result of command application.
pub type CommandResult = Result<CommandEffect, CommandError>;

/// What changed as a result of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEffect {
    EntityPlaced { handle: EntityHandle },
    EntityRemoved { handle: EntityHandle },
    EntityUpgraded { handle: EntityHandle, new_level: u8 },
    PolicyChanged { key: PolicyKey, old_value: i32, new_value: i32 },
    TilesBulldozed { count: u32 },
    EntityToggled { handle: EntityHandle, enabled: bool },
    ZoningApplied { count: u32 },
}

/// Apply a command to the world state after validation.
pub fn apply_command(world: &mut WorldState, cmd: &Command) -> CommandResult {
    apply_command_with_registry(world, None, cmd)
}

/// Apply a command with registry-aware validation and footprint logic.
pub fn apply_command_with_registry(
    world: &mut WorldState,
    registry: Option<&ArchetypeRegistry>,
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
            if let Some(registry) = registry {
                if let Some(def) = registry.get(*archetype_id) {
                    world.treasury -= def.cost_at_level(1);
                }
            }
            // Place entity
            match world.place_entity(*archetype_id, *x, *y, *rotation) {
                Some(handle) => Ok(CommandEffect::EntityPlaced { handle }),
                None => Err(CommandError::TileOccupied),
            }
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
                    world.tiles.set_zone(*x + dx, *y + dy, ZoneType::None);
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

        Command::SetZoning { x, y, w, h, zone } => {
            let mut count = 0u32;
            for dy in 0..*h as i16 {
                for dx in 0..*w as i16 {
                    if world.tiles.set_zone(*x + dx, *y + dy, *zone) {
                        count += 1;
                    }
                }
            }
            Ok(CommandEffect::ZoningApplied { count })
        }
    }
}

fn rects_overlap(
    ax: i16,
    ay: i16,
    aw: i16,
    ah: i16,
    bx: i16,
    by: i16,
    bw: i16,
    bh: i16,
) -> bool {
    let a_right = ax + aw;
    let a_bottom = ay + ah;
    let b_right = bx + bw;
    let b_bottom = by + bh;
    ax < b_right && a_right > bx && ay < b_bottom && a_bottom > by
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
    fn place_entity_on_water_tile_occupied() {
        let mut world = make_world();
        world.tiles.set_terrain(3, 3, TerrainType::Water);
        let cmd = Command::PlaceEntity {
            archetype_id: 1,
            x: 3,
            y: 3,
            rotation: 0,
        };
        assert_eq!(apply_command(&mut world, &cmd), Err(CommandError::TileOccupied));
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
        };
        let result = apply_command(&mut world, &cmd).unwrap();
        match result {
            CommandEffect::ZoningApplied { count } => {
                assert_eq!(count, 9); // 3x3
            }
            _ => panic!("Expected ZoningApplied effect"),
        }
        // Verify all tiles in range are zoned
        for dy in 0..3i16 {
            for dx in 0..3i16 {
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
}
