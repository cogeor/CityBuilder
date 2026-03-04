//! Composable validation specifications for commands.
//!
//! Each spec implements the `CommandSpec` trait and validates a single
//! condition against the world state. Specs compose via `CompositeSpec`
//! to form complete validation pipelines per command type.

use crate::core::commands::{Command, CommandError};
use crate::core::world::WorldState;
use crate::core_types::*;

// ─── CommandSpec trait ──────────────────────────────────────────────────────

/// A single validation rule that checks a precondition against the world state.
pub trait CommandSpec {
    /// Returns `Ok(())` if the condition holds, or `Err(CommandError)` if not.
    fn check(&self, world: &WorldState) -> Result<(), CommandError>;
}

// ─── Individual Specs ───────────────────────────────────────────────────────

/// Checks that tile coordinates are within the map bounds.
pub struct InBoundsSpec {
    pub x: i16,
    pub y: i16,
}

impl CommandSpec for InBoundsSpec {
    fn check(&self, world: &WorldState) -> Result<(), CommandError> {
        if world.tiles.in_bounds(self.x, self.y) {
            Ok(())
        } else {
            Err(CommandError::OutOfBounds)
        }
    }
}

/// Checks that a tile is not water or otherwise unbuildable.
pub struct BuildableTerrainSpec {
    pub x: i16,
    pub y: i16,
}

impl CommandSpec for BuildableTerrainSpec {
    fn check(&self, world: &WorldState) -> Result<(), CommandError> {
        if world.is_buildable(self.x, self.y) {
            Ok(())
        } else {
            Err(CommandError::TileOccupied)
        }
    }
}

/// Checks that the treasury has enough funds to cover the cost.
pub struct FundsAvailableSpec {
    pub cost: MoneyCents,
}

impl CommandSpec for FundsAvailableSpec {
    fn check(&self, world: &WorldState) -> Result<(), CommandError> {
        if world.treasury >= self.cost {
            Ok(())
        } else {
            Err(CommandError::InsufficientFunds)
        }
    }
}

/// Checks that a rectangular footprint has no existing entities.
pub struct NoCollisionSpec {
    pub x: i16,
    pub y: i16,
    pub w: u8,
    pub h: u8,
}

impl CommandSpec for NoCollisionSpec {
    fn check(&self, world: &WorldState) -> Result<(), CommandError> {
        for handle in world.entities.iter_alive() {
            if let Some(pos) = world.entities.get_pos(handle) {
                if pos.x >= self.x
                    && pos.x < self.x + self.w as i16
                    && pos.y >= self.y
                    && pos.y < self.y + self.h as i16
                {
                    return Err(CommandError::TileOccupied);
                }
            }
        }
        Ok(())
    }
}

/// Checks that a tile HAS an entity (for demolish operations).
pub struct TileOccupiedSpec {
    pub x: i16,
    pub y: i16,
}

impl CommandSpec for TileOccupiedSpec {
    fn check(&self, world: &WorldState) -> Result<(), CommandError> {
        for handle in world.entities.iter_alive() {
            if let Some(pos) = world.entities.get_pos(handle) {
                if pos.x == self.x && pos.y == self.y {
                    return Ok(());
                }
            }
        }
        // Also consider a tile with zoning as "occupied" for demolish purposes
        if let Some(tile) = world.tiles.get(self.x, self.y) {
            if tile.zone != ZoneType::None {
                return Ok(());
            }
        }
        Err(CommandError::ValidationFailed(
            "tile has nothing to demolish".to_string(),
        ))
    }
}

/// Checks that an archetype ID is valid.
///
/// NOTE: Full registry validation requires access to an `ArchetypeRegistry`,
/// which is not currently part of `WorldState`. This spec performs a basic
/// non-zero check. For full validation, use with an `ArchetypeRegistry` query
/// once it is integrated into the world state.
pub struct ValidArchetypeSpec {
    pub archetype_id: ArchetypeId,
}

impl CommandSpec for ValidArchetypeSpec {
    fn check(&self, _world: &WorldState) -> Result<(), CommandError> {
        if self.archetype_id == 0 {
            Err(CommandError::ValidationFailed(
                "invalid archetype id".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

// ─── CompositeSpec ──────────────────────────────────────────────────────────

/// A composite specification that runs multiple specs in sequence.
/// Short-circuits on the first failure.
pub struct CompositeSpec {
    specs: Vec<Box<dyn CommandSpec>>,
}

impl CompositeSpec {
    /// Create a new empty composite spec.
    pub fn new() -> Self {
        CompositeSpec { specs: Vec::new() }
    }

    /// Add a spec to the pipeline. Builder pattern.
    pub fn add(mut self, spec: Box<dyn CommandSpec>) -> Self {
        self.specs.push(spec);
        self
    }
}

impl CommandSpec for CompositeSpec {
    fn check(&self, world: &WorldState) -> Result<(), CommandError> {
        for spec in &self.specs {
            spec.check(world)?;
        }
        Ok(())
    }
}

// ─── Factory Functions ──────────────────────────────────────────────────────

/// Create a composite spec for the build (PlaceEntity) command.
///
/// Validates: in-bounds, buildable terrain, no collision, valid archetype, funds.
pub fn build_spec(
    x: i16,
    y: i16,
    w: u8,
    h: u8,
    archetype_id: ArchetypeId,
    cost: MoneyCents,
) -> CompositeSpec {
    CompositeSpec::new()
        .add(Box::new(InBoundsSpec { x, y }))
        .add(Box::new(BuildableTerrainSpec { x, y }))
        .add(Box::new(NoCollisionSpec { x, y, w, h }))
        .add(Box::new(ValidArchetypeSpec { archetype_id }))
        .add(Box::new(FundsAvailableSpec { cost }))
}

/// Create a composite spec for the demolish (Bulldoze) command.
///
/// Validates: in-bounds, tile has something to demolish.
pub fn demolish_spec(x: i16, y: i16) -> CompositeSpec {
    CompositeSpec::new()
        .add(Box::new(InBoundsSpec { x, y }))
        .add(Box::new(TileOccupiedSpec { x, y }))
}

/// Create a composite spec for the zone (SetZoning) command.
///
/// Validates: in-bounds.
pub fn zone_spec(x: i16, y: i16) -> CompositeSpec {
    CompositeSpec::new().add(Box::new(InBoundsSpec { x, y }))
}

/// Validate a command using the canonical spec pipeline.
///
/// This is the single ownership point for command preconditions.
pub fn validate_command(world: &WorldState, cmd: &Command) -> Result<(), CommandError> {
    match cmd {
        Command::PlaceEntity {
            archetype_id,
            x,
            y,
            rotation: _,
        } => {
            // Footprint/cost are placeholders until archetype lookup is threaded into validation.
            build_spec(*x, *y, 1, 1, *archetype_id, 0).check(world)
        }
        Command::Bulldoze { x, y, .. } => zone_spec(*x, *y).check(world),
        Command::SetZoning { x, y, .. } => zone_spec(*x, *y).check(world),
        Command::RemoveEntity { handle }
        | Command::UpgradeEntity { handle, .. }
        | Command::ToggleEntity { handle, .. } => {
            if world.entities.is_valid(*handle) {
                Ok(())
            } else {
                Err(CommandError::InvalidEntity)
            }
        }
        Command::SetPolicy { .. } => Ok(()),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::world::WorldState;
    use crate::core_types::{MapSize, TerrainType, ZoneType};

    fn make_world() -> WorldState {
        WorldState::new(MapSize::new(16, 16), 42)
    }

    // ── InBoundsSpec ────────────────────────────────────────────────────

    #[test]
    fn in_bounds_spec_passes_for_valid_coords() {
        let world = make_world();
        let spec = InBoundsSpec { x: 0, y: 0 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn in_bounds_spec_passes_for_max_valid_coords() {
        let world = make_world();
        let spec = InBoundsSpec { x: 15, y: 15 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn in_bounds_spec_fails_for_out_of_bounds_x() {
        let world = make_world();
        let spec = InBoundsSpec { x: 16, y: 0 };
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn in_bounds_spec_fails_for_out_of_bounds_y() {
        let world = make_world();
        let spec = InBoundsSpec { x: 0, y: 16 };
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn in_bounds_spec_fails_for_negative_coords() {
        let world = make_world();
        let spec = InBoundsSpec { x: -1, y: 0 };
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    // ── FundsAvailableSpec ──────────────────────────────────────────────

    #[test]
    fn funds_available_spec_passes_when_sufficient() {
        let world = make_world(); // treasury = 500_000
        let spec = FundsAvailableSpec { cost: 100_000 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn funds_available_spec_passes_when_exactly_equal() {
        let world = make_world(); // treasury = 500_000
        let spec = FundsAvailableSpec { cost: 500_000 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn funds_available_spec_fails_when_insufficient() {
        let world = make_world(); // treasury = 500_000
        let spec = FundsAvailableSpec { cost: 500_001 };
        assert_eq!(spec.check(&world), Err(CommandError::InsufficientFunds));
    }

    // ── NoCollisionSpec ─────────────────────────────────────────────────

    #[test]
    fn no_collision_spec_passes_on_empty_tile() {
        let world = make_world();
        let spec = NoCollisionSpec {
            x: 5,
            y: 5,
            w: 1,
            h: 1,
        };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn no_collision_spec_fails_on_occupied_tile() {
        let mut world = make_world();
        world.place_entity(1, 5, 5, 0);
        let spec = NoCollisionSpec {
            x: 5,
            y: 5,
            w: 1,
            h: 1,
        };
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    #[test]
    fn no_collision_spec_passes_when_entity_outside_footprint() {
        let mut world = make_world();
        world.place_entity(1, 10, 10, 0);
        let spec = NoCollisionSpec {
            x: 0,
            y: 0,
            w: 3,
            h: 3,
        };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn no_collision_spec_fails_when_entity_in_footprint() {
        let mut world = make_world();
        world.place_entity(1, 2, 2, 0);
        let spec = NoCollisionSpec {
            x: 1,
            y: 1,
            w: 3,
            h: 3,
        };
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    // ── TileOccupiedSpec ────────────────────────────────────────────────

    #[test]
    fn tile_occupied_spec_passes_when_entity_present() {
        let mut world = make_world();
        world.place_entity(1, 5, 5, 0);
        let spec = TileOccupiedSpec { x: 5, y: 5 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn tile_occupied_spec_passes_when_zoned() {
        let mut world = make_world();
        world.tiles.set_zone(5, 5, ZoneType::Residential);
        let spec = TileOccupiedSpec { x: 5, y: 5 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn tile_occupied_spec_fails_when_empty() {
        let world = make_world();
        let spec = TileOccupiedSpec { x: 5, y: 5 };
        assert!(spec.check(&world).is_err());
    }

    // ── ValidArchetypeSpec ──────────────────────────────────────────────

    #[test]
    fn valid_archetype_spec_passes_for_nonzero_id() {
        let world = make_world();
        let spec = ValidArchetypeSpec { archetype_id: 1 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn valid_archetype_spec_fails_for_zero_id() {
        let world = make_world();
        let spec = ValidArchetypeSpec { archetype_id: 0 };
        assert!(spec.check(&world).is_err());
    }

    // ── BuildableTerrainSpec ────────────────────────────────────────────

    #[test]
    fn buildable_terrain_spec_passes_on_grass() {
        let world = make_world();
        let spec = BuildableTerrainSpec { x: 5, y: 5 };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn buildable_terrain_spec_fails_on_water() {
        let mut world = make_world();
        world.tiles.set_terrain(5, 5, TerrainType::Water);
        let spec = BuildableTerrainSpec { x: 5, y: 5 };
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    // ── CompositeSpec ───────────────────────────────────────────────────

    #[test]
    fn composite_spec_passes_when_all_pass() {
        let world = make_world();
        let spec = CompositeSpec::new()
            .add(Box::new(InBoundsSpec { x: 5, y: 5 }))
            .add(Box::new(FundsAvailableSpec { cost: 100 }));
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn composite_spec_short_circuits_on_first_failure() {
        let world = make_world();
        // First spec fails (out of bounds), second would pass
        let spec = CompositeSpec::new()
            .add(Box::new(InBoundsSpec { x: 99, y: 99 }))
            .add(Box::new(FundsAvailableSpec { cost: 100 }));
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn composite_spec_second_fails() {
        let world = make_world(); // treasury = 500_000
        // First passes, second fails
        let spec = CompositeSpec::new()
            .add(Box::new(InBoundsSpec { x: 5, y: 5 }))
            .add(Box::new(FundsAvailableSpec { cost: 1_000_000 }));
        assert_eq!(spec.check(&world), Err(CommandError::InsufficientFunds));
    }

    #[test]
    fn composite_spec_empty_passes() {
        let world = make_world();
        let spec = CompositeSpec::new();
        assert!(spec.check(&world).is_ok());
    }

    // ── Factory: build_spec ─────────────────────────────────────────────

    #[test]
    fn build_spec_composes_all_required_checks() {
        let world = make_world();
        let spec = build_spec(5, 5, 1, 1, 1, 100);
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn build_spec_fails_out_of_bounds() {
        let world = make_world();
        let spec = build_spec(99, 99, 1, 1, 1, 100);
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn build_spec_fails_unbuildable_terrain() {
        let mut world = make_world();
        world.tiles.set_terrain(5, 5, TerrainType::Water);
        let spec = build_spec(5, 5, 1, 1, 1, 100);
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    #[test]
    fn build_spec_fails_collision() {
        let mut world = make_world();
        world.place_entity(1, 5, 5, 0);
        let spec = build_spec(5, 5, 1, 1, 1, 100);
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    #[test]
    fn build_spec_fails_insufficient_funds() {
        let world = make_world(); // treasury = 500_000
        let spec = build_spec(5, 5, 1, 1, 1, 1_000_000);
        assert_eq!(spec.check(&world), Err(CommandError::InsufficientFunds));
    }

    // ── Factory: demolish_spec ──────────────────────────────────────────

    #[test]
    fn demolish_spec_passes_when_entity_present() {
        let mut world = make_world();
        world.place_entity(1, 5, 5, 0);
        let spec = demolish_spec(5, 5);
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn demolish_spec_fails_out_of_bounds() {
        let world = make_world();
        let spec = demolish_spec(99, 99);
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn demolish_spec_fails_when_tile_empty() {
        let world = make_world();
        let spec = demolish_spec(5, 5);
        assert!(spec.check(&world).is_err());
    }

    // ── Factory: zone_spec ──────────────────────────────────────────────

    #[test]
    fn zone_spec_passes_in_bounds() {
        let world = make_world();
        let spec = zone_spec(5, 5);
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn zone_spec_fails_out_of_bounds() {
        let world = make_world();
        let spec = zone_spec(99, 99);
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn validate_command_place_entity_uses_build_specs() {
        let mut world = make_world();
        world.tiles.set_terrain(2, 2, TerrainType::Water);
        let cmd = Command::PlaceEntity {
            archetype_id: 1,
            x: 2,
            y: 2,
            rotation: 0,
        };
        assert_eq!(validate_command(&world, &cmd), Err(CommandError::TileOccupied));
    }

    #[test]
    fn validate_command_entity_commands_require_valid_handle() {
        let world = make_world();
        let cmd = Command::RemoveEntity {
            handle: EntityHandle::INVALID,
        };
        assert_eq!(validate_command(&world, &cmd), Err(CommandError::InvalidEntity));
    }
}
