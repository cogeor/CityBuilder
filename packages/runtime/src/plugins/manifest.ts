// @townbuilder/runtime -- Canonical plugin manifest contract
// Minimal, extendable manifest model with compatibility adapters for legacy shapes.

/** The kind of content a plugin contributes. */
export type PluginContentType =
  | "buildings"
  | "terrain"
  | "economy"
  | "networks"
  | "world"
  | "styles"
  | "shader_presets"
  | "localization"
  | "systems";

/** Runtime execution tier. Web defaults to data-only. */
export type PluginRuntimeKind = "data" | "script" | "wasm";

/** When a plugin should be activated. */
export type PluginActivation = "onWorldLoad" | "onDemand";

/** Canonical runtime manifest schema version. */
export type PluginManifestSchemaVersion = "v1";

/** Source format detected during normalization. */
export type PluginManifestSourceFormat = "canonical_v1" | "legacy_v1" | "custom";

/** Canonical plugin manifest used by runtime internals. */
export interface PluginManifest {
  /** Runtime schema version for normalized manifests. */
  readonly schema_version: PluginManifestSchemaVersion;
  /** Input format origin retained for compatibility/debugging. */
  readonly source_format: PluginManifestSourceFormat;
  readonly id: string;
  readonly name: string;
  readonly version: string;
  readonly description: string;
  readonly author: string;
  readonly dependencies: readonly string[];
  readonly activation: PluginActivation;
  readonly runtime: PluginRuntimeKind;
  readonly contributes: Readonly<Record<string, readonly string[]>>;
  /** Optional compatibility range (future use). */
  readonly engine_compat?: string;
  /** Legacy v1 compatibility fields retained for migration. */
  readonly content_type?: PluginContentType;
  readonly data_path?: string;
}

/** Runtime state for a registered plugin. */
export interface PluginEntry {
  readonly manifest: PluginManifest;
  loaded: boolean;
  data: unknown;
}

const VALID_CONTENT_TYPES: ReadonlySet<string> = new Set([
  "buildings",
  "terrain",
  "economy",
  "networks",
  "world",
  "styles",
  "shader_presets",
  "localization",
  "systems",
]);

const VALID_ACTIVATIONS: ReadonlySet<string> = new Set(["onWorldLoad", "onDemand"]);
const VALID_RUNTIMES: ReadonlySet<string> = new Set(["data", "script", "wasm"]);
const SEMVER_PATTERN = /^\d+\.\d+\.\d+(-[\w.]+)?$/;

function detectSourceFormat(m: Record<string, unknown>): PluginManifestSourceFormat {
  if (m.schema_version === "v1") {
    return "canonical_v1";
  }

  if (
    typeof m.plugin_id === "string" ||
    typeof m.plugin_version === "string" ||
    typeof m.content_type === "string" ||
    typeof m.data_path === "string"
  ) {
    return "legacy_v1";
  }

  return "custom";
}

function normalizeContributes(m: Record<string, unknown>): Record<string, readonly string[]> {
  const out: Record<string, readonly string[]> = {};

  if (m.contributes && typeof m.contributes === "object" && !Array.isArray(m.contributes)) {
    const contributes = m.contributes as Record<string, unknown>;
    for (const [key, value] of Object.entries(contributes)) {
      if (!VALID_CONTENT_TYPES.has(key) && key.length === 0) continue;
      if (Array.isArray(value) && value.every((v) => typeof v === "string")) {
        out[key] = value as string[];
      }
    }
  }

  if (typeof m.content_type === "string" && VALID_CONTENT_TYPES.has(m.content_type)) {
    const path = typeof m.data_path === "string" ? m.data_path : "";
    out[m.content_type] = path.length > 0 ? [path] : [];
  }

  if (Array.isArray(m.contentTypes)) {
    for (const ct of m.contentTypes) {
      if (typeof ct === "string" && VALID_CONTENT_TYPES.has(ct) && !(ct in out)) {
        out[ct] = [];
      }
    }
  }

  if (Array.isArray(m.provides)) {
    for (const provided of m.provides) {
      if (typeof provided === "string") {
        const normalized = provided === "archetypes" ? "buildings" : provided;
        if (VALID_CONTENT_TYPES.has(normalized) && !(normalized in out)) {
          out[normalized] = [];
        }
      }
    }
  }

  if (Array.isArray(m.archetypes)) {
    const entries = (m.archetypes as unknown[]).filter((v): v is string => typeof v === "string");
    if (entries.length > 0) {
      out.buildings = entries;
    }
  }

  return out;
}

/**
 * Normalize multiple manifest shapes (legacy + v2 + citypack-like) into the
 * canonical runtime manifest.
 */
export function normalizeManifest(raw: unknown): PluginManifest | null {
  if (raw === null || raw === undefined || typeof raw !== "object") {
    return null;
  }
  const m = raw as Record<string, unknown>;

  const id = typeof m.id === "string" ? m.id : typeof m.plugin_id === "string" ? m.plugin_id : "";
  const name = typeof m.name === "string" ? m.name : "";
  const version =
    typeof m.version === "string"
      ? m.version
      : typeof m.plugin_version === "string"
        ? m.plugin_version
        : "";
  const description = typeof m.description === "string" ? m.description : "";
  const author = typeof m.author === "string" ? m.author : "";

  const dependencies = Array.isArray(m.dependencies)
    ? (m.dependencies as unknown[]).filter((d): d is string => typeof d === "string")
    : [];

  let activation: PluginActivation = "onWorldLoad";
  if (typeof m.activation === "string" && VALID_ACTIVATIONS.has(m.activation)) {
    activation = m.activation as PluginActivation;
  } else if (
    m.activation &&
    typeof m.activation === "object" &&
    (m.activation as Record<string, unknown>).onWorldLoad === false
  ) {
    activation = "onDemand";
  }

  let runtime: PluginRuntimeKind = "data";
  if (typeof m.runtime === "string" && VALID_RUNTIMES.has(m.runtime)) {
    runtime = m.runtime as PluginRuntimeKind;
  } else if (Array.isArray(m.capabilities)) {
    const caps = (m.capabilities as unknown[]).filter((c): c is string => typeof c === "string");
    if (caps.includes("wasm")) runtime = "wasm";
    else if (caps.includes("script")) runtime = "script";
  }

  const contributes = normalizeContributes(m);

  const normalized: PluginManifest = {
    schema_version: "v1",
    source_format: detectSourceFormat(m),
    id,
    name,
    version,
    description,
    author,
    dependencies,
    activation,
    runtime,
    contributes,
    engine_compat: typeof m.engine_compat === "string" ? m.engine_compat : undefined,
  };
  if (typeof m.content_type === "string" && VALID_CONTENT_TYPES.has(m.content_type)) {
    (normalized as { content_type?: PluginContentType }).content_type = m.content_type as PluginContentType;
  }
  if (typeof m.data_path === "string") {
    (normalized as { data_path?: string }).data_path = m.data_path;
  }
  return normalized;
}

/** Type guard for canonical manifest validity. */
export function validateManifest(manifest: unknown): manifest is PluginManifest {
  if (manifest === null || manifest === undefined || typeof manifest !== "object") {
    return false;
  }
  const raw = manifest as Record<string, unknown>;

  // If legacy fields are present, they must be well-typed.
  if ("content_type" in raw && raw.content_type !== undefined) {
    if (typeof raw.content_type !== "string" || !VALID_CONTENT_TYPES.has(raw.content_type)) {
      return false;
    }
  }
  if ("activation" in raw && typeof raw.activation === "string" && !VALID_ACTIVATIONS.has(raw.activation)) {
    return false;
  }
  if ("dependencies" in raw && !Array.isArray(raw.dependencies)) {
    return false;
  }
  if (Array.isArray(raw.dependencies) && raw.dependencies.some((d) => typeof d !== "string")) {
    return false;
  }

  const normalized = normalizeManifest(manifest);
  if (!normalized) return false;
  if (normalized.id.length === 0 || normalized.name.length === 0) return false;
  if (normalized.schema_version !== "v1") return false;
  if (!SEMVER_PATTERN.test(normalized.version)) return false;
  if (!VALID_ACTIVATIONS.has(normalized.activation)) return false;
  if (!VALID_RUNTIMES.has(normalized.runtime)) return false;
  if (!Array.isArray(normalized.dependencies)) return false;
  if (normalized.dependencies.some((d) => d.length === 0)) return false;
  return Object.values(normalized.contributes).every(
    (arr) => Array.isArray(arr) && arr.every((v) => typeof v === "string"),
  );
}
