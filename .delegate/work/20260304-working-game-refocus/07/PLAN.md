# Loop 07 Plan — Rust city-builder interaction stabilization

## Goal
Make Rust/WASM the authoritative engine path for zoning, placement, bulldozing, and zoned development in the prototype slice.

## Scope
1. Introduce a dedicated Rust buildings domain module for base prototype archetypes and zone compatibility.
2. Add a Rust simulation system that grows zoned tiles into structures.
3. Route command validation/application through registry-aware logic in Rust.
4. Integrate command + system behavior into simulation tick ordering.
5. Update/add tests for zoning/placement semantics and special buildings.

## Out of Scope
- UI tool redesign
- Python asset forge loops
- Determinism hardening outside this active slice
