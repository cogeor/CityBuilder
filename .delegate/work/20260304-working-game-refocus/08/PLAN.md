# Loop 08 Plan — Runtime command bridge to Rust engine

## Goal
Ensure UI/runtime command flow is Rust-command-native end-to-end, with TS acting as orchestration/messaging only.

## Scope
1. Add runtime-side Rust command protocol types and mapping helpers.
2. Change RuntimeFacade command API to accept canonical engine commands.
3. Update worker handshake/command messaging for WASM-backed lifecycle and error handling.
4. Refactor SimWorker to initialize and operate against injected/loaded WASM handles.
5. Update runtime worker/facade tests to validate the new contract.

## Out of Scope
- Gameplay validation logic (owned by Rust engine)
- UI tool implementation details
