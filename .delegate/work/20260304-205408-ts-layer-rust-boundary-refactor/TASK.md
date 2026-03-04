# TASK: Refactor all TS logic around Rust/WASM boundary and versioning

Created: 2026-03-04 20:54:08 +01:00
Source: prompt

## Summary
Refactor TypeScript runtime/UI logic so TS remains orchestration/visualization, Rust/WASM owns game rules and mutations, and plugin versioning is unambiguous with a real v1 baseline.

## Objective
Deliver a minimal, well-designed, extendable TS layer with typed contracts and clear plugin manifest version semantics.

## Scope
- packages/ui/src: remove untyped event channels and duplicate event patterns.
- packages/runtime/src/engine: isolate TS/WASM command translation boundary.
- packages/runtime/src/plugins: unify manifest/version model and remove v1/v2/custom ambiguity.
- .delegate/doc/contracts: document new versioning model.

## Acceptance Criteria
- [ ] UI event systems are strongly typed and shared.
- [ ] Plugin manifest versioning starts from canonical 1 with explicit source-format normalization.
- [ ] TS/WASM boundary is explicit and minimal.
- [ ] Typecheck and tests pass.

