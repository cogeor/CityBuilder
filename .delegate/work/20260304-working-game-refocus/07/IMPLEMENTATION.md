# Loop 07 Implementation

## Files Changed
- packages/engine-wasm/src/core/buildings.rs (new)
- packages/engine-wasm/src/sim/systems/buildings.rs (new)
- packages/engine-wasm/src/core/mod.rs
- packages/engine-wasm/src/sim/systems/mod.rs
- packages/engine-wasm/src/sim/mod.rs
- packages/engine-wasm/src/core/commands_spec.rs
- packages/engine-wasm/src/core/commands.rs
- packages/engine-wasm/src/sim/tick.rs
- packages/engine-wasm/src/api/mod.rs

## What Was Implemented
1. Added modular Rust building domain support:
   - Base prototype archetype constants and registration helper.
   - Zone compatibility helpers and special-building classification.

2. Added zoned development simulation system:
   - Periodically attempts placements on zoned tiles using registry archetypes.
   - Enforces footprint/buildability/zone/collision checks.
   - Charges treasury via archetype level-1 cost on successful placement.

3. Hardened command pipeline to Rust registry-aware validation:
   - `validate_command_with_registry` for strict placement preconditions.
   - `apply_command_with_registry` for strict apply semantics.
   - Engine tick and direct apply path now use registry-aware command APIs.

4. Improved bulldoze semantics:
   - Entity removal uses footprint rectangle overlap, not only anchor tile.

5. Updated engine bootstrap:
   - Base city-builder archetypes are registered on new/load game handles.

6. Added/updated tests:
   - Zone mismatch vs match behavior for registry-aware placement.
   - Engine tick tests for zone-first placement and zoned auto-development.
   - API tests adapted to zone/special-building semantics.
