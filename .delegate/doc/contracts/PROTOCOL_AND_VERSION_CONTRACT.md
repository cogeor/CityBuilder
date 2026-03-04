# Protocol and Version Contract

Date: 2026-03-03
Scope: Runtime messaging, simulation commands/diffs/events, and version ownership

## 1. Canonical Envelopes

## CommandEnvelope

```text
command_id: u64
client_tick_hint: u64
kind: enum
payload: bytes | typed struct
schema_version: u16
```

## DiffEnvelope

```text
tick: u64
diff_seq: u32
entity_diffs: list
tile_diffs: list
metric_diffs: list
schema_version: u16
```

## EventEnvelope

```text
tick: u64
event_seq: u32
severity: enum
kind: enum
payload: typed struct
schema_version: u16
```

## 2. Version Ownership

- `engine_version`: owned by engine runtime maintainers.
- `content_version`: owned by plugin/content maintainers.
- `save_schema_version`: owned by engine-io maintainers.
- `wire_schema_version`: owned by runtime messaging maintainers.
- `plugin_manifest_schema_version`: owned by runtime plugin maintainers.

## 2.1 Plugin Manifest Baseline

- Runtime canonical manifest schema starts at `v1` (real baseline).
- Runtime internals only operate on normalized `v1` manifests.
- Incoming plugin manifests may come from:
  - `canonical_v1`: already in runtime schema.
  - `legacy_v1`: historical fields (`plugin_id`, `plugin_version`, `content_type`, `data_path`).
  - `custom`: non-canonical shapes adapted during normalization.
- `v2` is not a separate runtime manifest contract yet. Any future `v2` must include:
  - explicit migration from normalized `v1`,
  - compatibility fixtures in CI,
  - documented cutover window.

## 3. Change Rules

- Backward-compatible additions: minor bump.
- Breaking wire/save/API changes: major bump + migration doc required.
- Multi-version support window:
  - Save: current + previous 2 schema versions.
  - Wire: current + previous 1 schema version.

## 4. Enforcement

- CI must run compatibility fixtures for all supported versions.
- Mismatch without migration path is a release blocker.
