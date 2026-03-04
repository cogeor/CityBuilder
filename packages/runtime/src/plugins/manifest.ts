// @townbuilder/runtime — Plugin manifest types and validation
// Defines the schema for plugin manifests that declare building archetypes,
// terrain rules, economy configs, and other moddable content.

// ---- Content Type ----

/** The kind of content a plugin provides. */
export type PluginContentType =
  | "buildings"
  | "terrain"
  | "economy"
  | "networks"
  | "world";

/** When a plugin should be activated. */
export type PluginActivation = "onWorldLoad" | "onDemand";

// ---- Manifest Interface ----

/** Describes a plugin's metadata, dependencies, and content location. */
export interface PluginManifest {
  /** Unique identifier, e.g. "base.buildings". */
  readonly id: string;
  /** Human-readable display name. */
  readonly name: string;
  /** Semantic version string, e.g. "1.0.0". */
  readonly version: string;
  /** Short description of what the plugin provides. */
  readonly description: string;
  /** Plugin author name. */
  readonly author: string;
  /** IDs of other plugins this one depends on. */
  readonly dependencies: readonly string[];
  /** When the plugin should be activated. */
  readonly activation: PluginActivation;
  /** The kind of content this plugin provides. */
  readonly content_type: PluginContentType;
  /** Path to data files relative to the plugin root. */
  readonly data_path: string;
}

// ---- Plugin Entry ----

/** Runtime state for a registered plugin. */
export interface PluginEntry {
  /** The plugin's manifest metadata. */
  readonly manifest: PluginManifest;
  /** Whether the plugin's data has been loaded. */
  loaded: boolean;
  /** The loaded plugin data (type depends on content_type). */
  data: unknown;
}

// ---- Validation ----

/** Valid content type values. */
const VALID_CONTENT_TYPES: ReadonlyArray<string> = [
  "buildings",
  "terrain",
  "economy",
  "networks",
  "world",
];

/** Valid activation values. */
const VALID_ACTIVATIONS: ReadonlyArray<string> = ["onWorldLoad", "onDemand"];

/**
 * Type guard that validates an unknown value is a well-formed PluginManifest.
 * Checks that all required fields are present and have the correct types.
 */
export function validateManifest(
  manifest: unknown,
): manifest is PluginManifest {
  if (manifest === null || manifest === undefined || typeof manifest !== "object") {
    return false;
  }

  const m = manifest as Record<string, unknown>;

  // Required string fields
  if (typeof m.id !== "string" || m.id.length === 0) return false;
  if (typeof m.name !== "string" || m.name.length === 0) return false;
  if (typeof m.version !== "string" || m.version.length === 0) return false;
  if (typeof m.description !== "string") return false;
  if (typeof m.author !== "string") return false;
  if (typeof m.data_path !== "string") return false;

  // Activation enum
  if (typeof m.activation !== "string") return false;
  if (!VALID_ACTIVATIONS.includes(m.activation)) return false;

  // Content type enum
  if (typeof m.content_type !== "string") return false;
  if (!VALID_CONTENT_TYPES.includes(m.content_type)) return false;

  // Dependencies array
  if (!Array.isArray(m.dependencies)) return false;
  for (const dep of m.dependencies) {
    if (typeof dep !== "string") return false;
  }

  return true;
}
