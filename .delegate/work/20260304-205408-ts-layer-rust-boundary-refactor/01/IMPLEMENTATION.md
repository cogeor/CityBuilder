# Implementation Log

## Task 1: Shared typed event infrastructure and UI migration

Completed: 2026-03-04 21:01:00 +01:00

### Changes

- `packages/ui/src/shared/typed_events.ts`: Added a reusable typed event hub (`TypedEventHub`) and generic listener contract.
- `packages/ui/src/shell/shell.ts`: Replaced ad-hoc `any` payload event bus with typed `ShellEventPayloads`.
- `packages/ui/src/tools/tool_manager.ts`: Introduced typed payload map for tool events and migrated emit/listener wiring.
- `packages/ui/src/input/camera_controller.ts`: Typed all emitted camera/click events via payload map.
- `packages/ui/src/inspectors/building_inspector.ts`: Typed inspector events (`open/close/pin/unpin/upgrade/demolish`).
- `packages/ui/src/inspectors/budget_panel.ts`: Typed budget/tax/view mode change events.
- `packages/ui/src/overlays/overlay_panel.ts`: Typed overlay and minimap click events.
- `packages/ui/src/devtools/devtools.ts`: Typed toggle/panel/command events and removed `Record<string, any>` tile payloads.
- UI barrel exports updated (`packages/ui/src/**/index.ts`, `packages/ui/src/index.ts`) to surface new payload types.

### Verification

- [x] UI classes compile with no `data: any` event handlers.

---

## Task 2: Canonical plugin manifest versioning (`v1`) and compatibility metadata

Completed: 2026-03-04 21:01:00 +01:00

### Changes

- `packages/runtime/src/plugins/manifest.ts`:
  - Added canonical schema fields to normalized runtime manifests:
    - `schema_version: "v1"`
    - `source_format: "canonical_v1" | "legacy_v1" | "custom"`
  - Added source-format detection during normalization.
  - Kept legacy/custom input support while unifying runtime internals on canonical `v1`.
- `packages/runtime/src/plugins/index.ts`: Removed `manifest_v2` exports and exposed canonical schema/source-format types.
- `packages/runtime/src/plugins/__tests__/loader.test.ts`:
  - Updated fixtures for canonical schema fields.
  - Added assertions for legacy/custom source format classification.
- Removed obsolete split-manifest artifacts:
  - `packages/runtime/src/plugins/manifest_v2.ts`
  - `packages/runtime/src/plugins/__tests__/manifest_v2.test.ts`

### Verification

- [x] Plugin normalization and loader tests pass against canonical `v1` model.

---

## Task 3: Tight TS?Rust command boundary

Completed: 2026-03-04 21:01:00 +01:00

### Changes

- `packages/runtime/src/engine/commands.ts`: Reduced to canonical Rust command contract types only.
- `packages/runtime/src/engine/interaction_bridge.ts`: Added explicit TS adapter module for translating UI tool intent into engine commands.
- `packages/runtime/src/engine/index.ts`: Exported `translateToolInteraction` and compatibility alias.
- `packages/runtime/src/runtime_facade.ts`: Tightened command input type to `Readonly<EngineCommand>`.

### Verification

- [x] Engine boundary compiles and runtime tests continue to pass.

---

## Task 4: Versioning docs update

Completed: 2026-03-04 21:01:00 +01:00

### Changes

- `.delegate/doc/contracts/PROTOCOL_AND_VERSION_CONTRACT.md`:
  - Added plugin manifest schema ownership.
  - Defined canonical baseline as `v1` with source-format normalization policy (`canonical_v1`, `legacy_v1`, `custom`).
  - Clarified future `v2` requirements.

### Verification

- [x] Documentation reflects implemented versioning contract.
