# Loop 08 Implementation

## Files Changed
- packages/runtime/src/engine/commands.ts (new)
- packages/runtime/src/engine/index.ts (new)
- packages/runtime/src/index.ts
- packages/runtime/src/runtime_facade.ts
- packages/runtime/src/__tests__/runtime_facade.test.ts
- packages/runtime/src/messaging/types.ts
- packages/runtime/src/messaging/worker_manager.ts
- packages/runtime/src/workers/sim.worker.ts
- packages/runtime/src/workers/__tests__/sim.worker.test.ts
- packages/runtime/src/workers/index.ts

## What Was Implemented
1. Added runtime `engine` module defining canonical Rust/WASM command envelopes and tool->command mapping helpers.
2. Updated `RuntimeFacade.sendCommand` to accept `EngineCommand` and preserve command kind from enum key names.
3. Updated handshake wire contract to include init params and explicit success/failure fields.
4. Improved `WorkerManager` startup behavior:
   - sends seed/size in handshake,
   - rejects startup when worker handshake reports failure.
5. Refactored `SimWorker` to be WASM-authoritative:
   - async module loading/injection,
   - handshake-driven game initialization,
   - parsing of tick/command JSON responses from Rust,
   - save/load paths bridged through wasm module loader contract.
6. Updated/cleaned runtime tests to match new message/command contracts.
