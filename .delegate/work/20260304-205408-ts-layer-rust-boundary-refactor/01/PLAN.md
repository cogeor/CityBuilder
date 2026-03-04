# Loop 01: TS-wide typed contracts + plugin versioning unification + boundary cleanup

## Overview

Replace ad-hoc TS event channels and ambiguous plugin manifest version semantics with a single typed architecture that keeps Rust/WASM authoritative for gameplay mutations.

## Tasks

### Task 1: Shared typed event infrastructure and UI migration

**Goal:** Remove `data: any` event buses from UI modules and replace with typed event maps and one shared emitter utility.

**Files:**
| Action | Path |
|--------|------|
| CREATE | `packages/ui/src/shared/typed_events.ts` |
| MODIFY | `packages/ui/src/shell/shell.ts` |
| MODIFY | `packages/ui/src/tools/tool_manager.ts` |
| MODIFY | `packages/ui/src/input/camera_controller.ts` |
| MODIFY | `packages/ui/src/inspectors/building_inspector.ts` |
| MODIFY | `packages/ui/src/inspectors/budget_panel.ts` |
| MODIFY | `packages/ui/src/overlays/overlay_panel.ts` |
| MODIFY | `packages/ui/src/devtools/devtools.ts` |
| MODIFY | `packages/ui/src/index.ts` |

**Steps:**
1. Add a minimal generic typed emitter utility.
2. Convert each UI manager to typed event payload maps.
3. Preserve public behavior while eliminating `any` in event APIs.

**Verify:** `npm run typecheck` and UI unit tests.

### Task 2: Canonical plugin manifest versioning (`v1`) and compatibility input metadata

**Goal:** Eliminate confusing split between runtime manifest models by making one canonical version with explicit normalized source markers.

**Files:**
| Action | Path |
|--------|------|
| MODIFY | `packages/runtime/src/plugins/manifest.ts` |
| MODIFY | `packages/runtime/src/plugins/index.ts` |
| MODIFY | `packages/runtime/src/plugins/validators/*.ts` |
| MODIFY | `packages/runtime/src/plugins/__tests__/*.ts` |
| DELETE | `packages/runtime/src/plugins/manifest_v2.ts` |
| DELETE | `packages/runtime/src/plugins/__tests__/manifest_v2.test.ts` |

**Steps:**
1. Add `schema_version: "v1"` and `source_format` metadata to normalized manifests.
2. Keep legacy/custom import support through normalization only.
3. Remove public v2 manifest exports and obsolete tests.

**Verify:** runtime test suite for plugin loader/validators.

### Task 3: Tight TS?Rust command boundary

**Goal:** Keep TS as input-translation/orchestration layer and make command mapping contracts explicit and minimal.

**Files:**
| Action | Path |
|--------|------|
| MODIFY | `packages/runtime/src/engine/commands.ts` |
| MODIFY | `packages/runtime/src/runtime_facade.ts` |
| MODIFY | `web/src/main.ts` |
| MODIFY | `packages/runtime/src/__tests__/runtime_facade.test.ts` |

**Steps:**
1. Introduce explicit boundary naming in engine command mapping APIs.
2. Route UI tool command conversion via boundary mapper only.
3. Keep all world mutation payloads as engine commands.

**Verify:** runtime tests and web tests pass.

### Task 4: Versioning docs update

**Goal:** Document real v1 baseline and migration policy for future v2/custom content.

**Files:**
| Action | Path |
|--------|------|
| MODIFY | `.delegate/doc/contracts/PROTOCOL_AND_VERSION_CONTRACT.md` |

**Steps:**
1. Add plugin manifest schema ownership/version section.
2. Define baseline as canonical v1 and source-format normalization policy.

**Verify:** docs reflect implemented contracts.

## Acceptance Criteria

- [ ] UI managers use typed event payload maps with no `data: any` handlers.
- [ ] Plugin manifest API exposes one canonical runtime schema (`v1`) and explicit compatibility normalization metadata.
- [ ] TS/WASM mutation contract remains minimal and explicit.
- [ ] Typecheck and tests pass.
