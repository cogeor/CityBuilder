//! Composable validation specifications for commands.
//!
//! Each spec implements the `CommandSpec` trait and validates a single
//! condition against the world state. Specs compose via `CompositeSpec`
//! to form complete validation pipelines per command type.

use crate::core::archetypes::{ArchetypeRegistry, Prerequisite};
use crate::core::buildings::zone_for_archetype;
use crate::core::commands::{Command, CommandError};
use crate::core::math_util::rects_overlap;
use crate::core::tilemap::TileFlags;
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
        if self.x >= 0 && self.y >= 0
            && world.tiles.in_bounds(self.x as u32, self.y as u32)
        {
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
            Err(CommandError::TerrainNotBuildable)
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
pub struct NoCollisionSpec<'r> {
    pub x: i16,
    pub y: i16,
    pub w: u8,
    pub h: u8,
    pub registry: &'r ArchetypeRegistry,
}

impl<'r> CommandSpec for NoCollisionSpec<'r> {
    fn check(&self, world: &WorldState) -> Result<(), CommandError> {
        for handle in world.entities.iter_alive() {
            let Some(pos) = world.entities.get_pos(handle) else {
                continue;
            };
            let (ew, eh) = world
                .entities
                .get_archetype(handle)
                .and_then(|id| self.registry.get(id))
                .map(|def| (def.footprint_w as i16, def.footprint_h as i16))
                .unwrap_or((1, 1));
            if rects_overlap(
                self.x,
                self.y,
                self.w as i16,
                self.h as i16,
                pos.x,
                pos.y,
                ew,
                eh,
            ) {
                return Err(CommandError::TileOccupied);
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
        if self.x >= 0 && self.y >= 0 {
            if let Some(tile) = world.tiles.get(self.x as u32, self.y as u32) {
                if tile.zone != ZoneType::None {
                    return Ok(());
                }
            }
        }
        Err(CommandError::ValidationFailed(
            "tile has nothing to demolish".to_string(),
        ))
    }
}

/// Checks that an archetype ID is valid (present in the registry).
pub struct ValidArchetypeSpec<'r> {
    pub archetype_id: ArchetypeId,
    pub registry: &'r ArchetypeRegistry,
}

impl<'r> CommandSpec for ValidArchetypeSpec<'r> {
    fn check(&self, _world: &WorldState) -> Result<(), CommandError> {
        if self.registry.get(self.archetype_id).is_some() {
            Ok(())
        } else {
            Err(CommandError::InvalidArchetype)
        }
    }
}

// ─── CompositeSpec ──────────────────────────────────────────────────────────

/// A composite specification that runs multiple specs in sequence.
/// Short-circuits on the first failure.
pub struct CompositeSpec<'r> {
    specs: Vec<Box<dyn CommandSpec + 'r>>,
}

impl<'r> CompositeSpec<'r> {
    /// Create a new empty composite spec.
    pub fn new() -> Self {
        CompositeSpec { specs: Vec::new() }
    }

    /// Add a spec to the pipeline. Builder pattern.
    pub fn add(mut self, spec: Box<dyn CommandSpec + 'r>) -> Self {
        self.specs.push(spec);
        self
    }
}

impl<'r> CommandSpec for CompositeSpec<'r> {
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
pub fn build_spec<'r>(
    x: i16,
    y: i16,
    w: u8,
    h: u8,
    archetype_id: ArchetypeId,
    cost: MoneyCents,
    registry: &'r ArchetypeRegistry,
) -> CompositeSpec<'r> {
    CompositeSpec::new()
        .add(Box::new(InBoundsSpec { x, y }))
        .add(Box::new(BuildableTerrainSpec { x, y }))
        .add(Box::new(NoCollisionSpec { x, y, w, h, registry }))
        .add(Box::new(ValidArchetypeSpec { archetype_id, registry }))
        .add(Box::new(FundsAvailableSpec { cost }))
}

/// Create a composite spec for the demolish (Bulldoze) command.
///
/// Validates: in-bounds, tile has something to demolish.
pub fn demolish_spec(x: i16, y: i16) -> CompositeSpec<'static> {
    CompositeSpec::new()
        .add(Box::new(InBoundsSpec { x, y }))
        .add(Box::new(TileOccupiedSpec { x, y }))
}

/// Create a composite spec for the zone (SetZoning) command.
///
/// Validates: in-bounds.
pub fn zone_spec(x: i16, y: i16) -> CompositeSpec<'static> {
    CompositeSpec::new().add(Box::new(InBoundsSpec { x, y }))
}

/// Validate a command using the canonical spec pipeline.
///
/// This is the single ownership point for command preconditions.
pub fn validate_command(world: &WorldState, cmd: &Command) -> Result<(), CommandError> {
    validate_command_with_registry(world, None, cmd)
}

/// Registry-aware command validation.
///
/// When a registry is provided, placement preconditions become footprint,
/// zone, and treasury aware for city-builder semantics.
pub fn validate_command_with_registry(
    world: &WorldState,
    registry: Option<&ArchetypeRegistry>,
    cmd: &Command,
) -> Result<(), CommandError> {
    match cmd {
        Command::PlaceEntity {
            archetype_id,
            x,
            y,
            rotation: _,
        } => {
            let Some(registry) = registry else {
                // Degraded path: no registry provided, perform basic checks only.
                if *archetype_id == 0 {
                    return Err(CommandError::InvalidArchetype);
                }
                return CompositeSpec::new()
                    .add(Box::new(InBoundsSpec { x: *x, y: *y }))
                    .add(Box::new(BuildableTerrainSpec { x: *x, y: *y }))
                    .check(world);
            };
            let Some(def) = registry.get(*archetype_id) else {
                return Err(CommandError::InvalidArchetype);
            };
            if *x < 0 || *y < 0 {
                return Err(CommandError::OutOfBounds);
            }

            for dy in 0..def.footprint_h as i16 {
                for dx in 0..def.footprint_w as i16 {
                    let tx = *x + dx;
                    let ty = *y + dy;
                    if tx < 0 || ty < 0 || !world.tiles.in_bounds(tx as u32, ty as u32) {
                        return Err(CommandError::OutOfBounds);
                    }
                    if !world.is_buildable(tx, ty) {
                        return Err(CommandError::TileOccupied);
                    }
                }
            }

            // Utility, Service, and Transport archetypes return None from zone_for_archetype
            // and intentionally skip zone validation — they can be placed in any zone or
            // on unzoned land, matching SimCity civic building rules.
            if let Some(zone) = zone_for_archetype(def) {
                for dy in 0..def.footprint_h as i16 {
                    for dx in 0..def.footprint_w as i16 {
                        let tx = *x + dx;
                        let ty = *y + dy;
                        // tx and ty are guaranteed >= 0 from the in_bounds loop above
                        let Some(tile) = world.tiles.get(tx as u32, ty as u32) else {
                            return Err(CommandError::OutOfBounds);
                        };
                        if tile.zone != zone {
                            return Err(CommandError::WrongZone);
                        }
                    }
                }
            }

            // Enforce archetype prerequisites.
            for prereq in &def.prerequisites {
                match prereq {
                    Prerequisite::RoadAccess => {
                        // Road access: any footprint tile must have TileFlags::ROAD_ACCESS
                        // (set by the road system when a road is adjacent to the tile).
                        let mut has_access = false;
                        'road_check: for dy in 0..def.footprint_h as i16 {
                            for dx in 0..def.footprint_w as i16 {
                                let tx = *x + dx;
                                let ty = *y + dy;
                                if let Some(tile) = world.tiles.get(tx as u32, ty as u32) {
                                    if tile.flags.contains(TileFlags::ROAD_ACCESS) {
                                        has_access = true;
                                        break 'road_check;
                                    }
                                }
                            }
                        }
                        if !has_access {
                            return Err(CommandError::NoRoadAccess);
                        }
                    }
                    Prerequisite::PowerConnection | Prerequisite::WaterConnection => {
                        // Stub: checked by utility systems after placement, not at validation time.
                    }
                }
            }

            for handle in world.entities.iter_alive() {
                let Some(pos) = world.entities.get_pos(handle) else {
                    continue;
                };
                let (w, h) = world
                    .entities
                    .get_archetype(handle)
                    .and_then(|id| registry.get(id))
                    .map(|def| (def.footprint_w as i16, def.footprint_h as i16))
                    .unwrap_or((1, 1));
                if rects_overlap(
                    *x,
                    *y,
                    def.footprint_w as i16,
                    def.footprint_h as i16,
                    pos.x,
                    pos.y,
                    w,
                    h,
                ) {
                    return Err(CommandError::TileOccupied);
                }
            }

            let cost = def.cost_at_level(1);
            if world.treasury < cost {
                return Err(CommandError::InsufficientFunds);
            }
            Ok(())
        }
        Command::Bulldoze { x, y, .. } => zone_spec(*x, *y).check(world),
        Command::SetZoning { x, y, .. } => zone_spec(*x, *y).check(world),
        Command::SetTerrain { x, y, .. } => zone_spec(*x, *y).check(world),
        Command::SetRoadLine { x0, y0, x1, y1, .. } => {
            zone_spec(*x0, *y0).check(world)?;
            zone_spec(*x1, *y1).check(world)?;
            if x0 != x1 && y0 != y1 {
                return Err(CommandError::ValidationFailed(
                    "road lines must be axis-aligned".to_string(),
                ));
            }
            Ok(())
        }
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
        Command::RemoveRoad { x, y } => zone_spec(*x, *y).check(world),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::archetypes::ArchetypeRegistry;
    use crate::core::buildings::{
        register_base_city_builder_archetypes, ARCH_RES_SMALL_HOUSE, ARCH_UTIL_POWER_PLANT,
    };
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
        let registry = ArchetypeRegistry::new(); // empty — fallback (1,1)
        let spec = NoCollisionSpec {
            x: 5,
            y: 5,
            w: 1,
            h: 1,
            registry: &registry,
        };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn no_collision_spec_fails_on_occupied_tile() {
        let mut world = make_world();
        world.place_entity(1, 5, 5, 0);
        let registry = ArchetypeRegistry::new(); // empty — fallback (1,1)
        let spec = NoCollisionSpec {
            x: 5,
            y: 5,
            w: 1,
            h: 1,
            registry: &registry,
        };
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    #[test]
    fn no_collision_spec_passes_when_entity_outside_footprint() {
        let mut world = make_world();
        world.place_entity(1, 10, 10, 0);
        let registry = ArchetypeRegistry::new(); // empty — fallback (1,1)
        let spec = NoCollisionSpec {
            x: 0,
            y: 0,
            w: 3,
            h: 3,
            registry: &registry,
        };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn no_collision_spec_fails_when_entity_in_footprint() {
        let mut world = make_world();
        world.place_entity(1, 2, 2, 0);
        let registry = ArchetypeRegistry::new(); // empty — fallback (1,1)
        let spec = NoCollisionSpec {
            x: 1,
            y: 1,
            w: 3,
            h: 3,
            registry: &registry,
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
    fn valid_archetype_spec_fails_for_zero_id_unregistered() {
        let world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        // Zero is never registered
        let spec = ValidArchetypeSpec { archetype_id: 0, registry: &registry };
        assert_eq!(
            spec.check(&world),
            Err(CommandError::InvalidArchetype)
        );
    }

    #[test]
    fn valid_archetype_spec_passes_for_registered_id() {
        let world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        let spec = ValidArchetypeSpec {
            archetype_id: ARCH_RES_SMALL_HOUSE,
            registry: &registry,
        };
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn valid_archetype_spec_rejects_unregistered_id() {
        let world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        // ID 9999 is not registered
        let spec = ValidArchetypeSpec { archetype_id: 9999, registry: &registry };
        assert_eq!(
            spec.check(&world),
            Err(CommandError::InvalidArchetype)
        );
    }

    // ── NoCollisionSpec: multi-tile footprint overlap ────────────────────

    #[test]
    fn no_collision_spec_detects_multi_tile_footprint_overlap() {
        let mut world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        // Place the 3x3 power plant archetype at (0,0)
        world.place_entity(ARCH_UTIL_POWER_PLANT, 0, 0, 0);
        // Attempt a 1x1 placement at (2,2) — inside the 3x3 footprint
        let spec = NoCollisionSpec { x: 2, y: 2, w: 1, h: 1, registry: &registry };
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    #[test]
    fn no_collision_spec_passes_adjacent_to_multi_tile_entity() {
        let mut world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        world.place_entity(ARCH_UTIL_POWER_PLANT, 0, 0, 0);
        // Placement at (3,0) is adjacent but not overlapping the 3x3 footprint
        let spec = NoCollisionSpec { x: 3, y: 0, w: 1, h: 1, registry: &registry };
        assert!(spec.check(&world).is_ok());
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
        assert_eq!(spec.check(&world), Err(CommandError::TerrainNotBuildable));
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
        let mut world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        // ARCH_RES_SMALL_HOUSE requires Residential zoning
        world.tiles.set_zone(5, 5, crate::core_types::ZoneType::Residential);
        let spec = build_spec(5, 5, 1, 1, ARCH_RES_SMALL_HOUSE, 100, &registry);
        assert!(spec.check(&world).is_ok());
    }

    #[test]
    fn build_spec_fails_out_of_bounds() {
        let world = make_world();
        let registry = ArchetypeRegistry::new();
        let spec = build_spec(99, 99, 1, 1, ARCH_RES_SMALL_HOUSE, 100, &registry);
        assert_eq!(spec.check(&world), Err(CommandError::OutOfBounds));
    }

    #[test]
    fn build_spec_fails_unbuildable_terrain() {
        let mut world = make_world();
        world.tiles.set_terrain(5, 5, TerrainType::Water);
        let registry = ArchetypeRegistry::new();
        let spec = build_spec(5, 5, 1, 1, ARCH_RES_SMALL_HOUSE, 100, &registry);
        assert_eq!(spec.check(&world), Err(CommandError::TerrainNotBuildable));
    }

    #[test]
    fn build_spec_fails_collision() {
        let mut world = make_world();
        world.place_entity(ARCH_RES_SMALL_HOUSE, 5, 5, 0);
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        world.tiles.set_zone(5, 5, crate::core_types::ZoneType::Residential);
        let spec = build_spec(5, 5, 1, 1, ARCH_RES_SMALL_HOUSE, 100, &registry);
        assert_eq!(spec.check(&world), Err(CommandError::TileOccupied));
    }

    #[test]
    fn build_spec_fails_insufficient_funds() {
        let world = make_world(); // treasury = 500_000
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        let spec = build_spec(5, 5, 1, 1, ARCH_RES_SMALL_HOUSE, 1_000_000, &registry);
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
        assert_eq!(validate_command(&world, &cmd), Err(CommandError::TerrainNotBuildable));
    }

    #[test]
    fn validate_command_entity_commands_require_valid_handle() {
        let world = make_world();
        let cmd = Command::RemoveEntity {
            handle: EntityHandle::INVALID,
        };
        assert_eq!(validate_command(&world, &cmd), Err(CommandError::InvalidEntity));
    }

    #[test]
    fn validate_with_registry_rejects_zone_mismatch_for_residential() {
        let mut world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        world.tiles.set_zone(4, 4, ZoneType::Industrial);
        let cmd = Command::PlaceEntity {
            archetype_id: ARCH_RES_SMALL_HOUSE,
            x: 4,
            y: 4,
            rotation: 0,
        };
        assert_eq!(
            validate_command_with_registry(&world, Some(&registry), &cmd),
            Err(CommandError::WrongZone)
        );
    }

    #[test]
    fn validate_with_registry_accepts_matching_zone() {
        let mut world = make_world();
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        world.tiles.set_zone(4, 4, ZoneType::Residential);
        let cmd = Command::PlaceEntity {
            archetype_id: ARCH_RES_SMALL_HOUSE,
            x: 4,
            y: 4,
            rotation: 0,
        };
        assert!(validate_command_with_registry(&world, Some(&registry), &cmd).is_ok());
    }
}
