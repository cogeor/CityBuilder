// @townbuilder/runtime — Plugin manifest v2 types and validation
// Extends the original manifest with additional content types, author metadata,
// and engine version constraints for the expanded plugin system.

// ---- Content Type Enum ----

/** Extended content types that plugins can provide. */
export enum ContentType {
  Buildings = "buildings",
  Terrain = "terrain",
  Networks = "networks",
  Economy = "economy",
  Progression = "progression",
  Demographics = "demographics",
  Events = "events",
  Scenarios = "scenarios",
  Governance = "governance",
  AI = "ai",
  Tools = "tools",
}

/** All valid ContentType values for runtime validation. */
const ALL_CONTENT_TYPES: ReadonlySet<string> = new Set(Object.values(ContentType));

// ---- Manifest Interfaces ----

/** Original v1 manifest shape (subset of PluginManifest). */
export interface ManifestV1 {
  id: string;
  name: string;
  version: string;
  dependencies?: string[];
}

/** Extended v2 manifest with additional metadata and content types. */
export interface ManifestV2 extends ManifestV1 {
  manifestVersion: 2;
  contentTypes: ContentType[];
  engineVersion?: string;
  author?: string;
  description?: string;
}

/** Union of all supported manifest versions. */
export type PluginManifestAny = ManifestV1 | ManifestV2;

// ---- Type Guards ----

/**
 * Type guard that checks whether a manifest is a v2 manifest.
 * A v2 manifest must have manifestVersion === 2 and a contentTypes array.
 */
export function isManifestV2(manifest: PluginManifestAny): manifest is ManifestV2 {
  return (
    "manifestVersion" in manifest &&
    (manifest as ManifestV2).manifestVersion === 2 &&
    "contentTypes" in manifest &&
    Array.isArray((manifest as ManifestV2).contentTypes)
  );
}

// ---- Migration ----

/**
 * Migrate a v1 manifest to v2 format by adding default values.
 * The contentTypes array defaults to [Buildings] and manifestVersion is set to 2.
 */
export function migrateV1toV2(v1: ManifestV1): ManifestV2 {
  return {
    id: v1.id,
    name: v1.name,
    version: v1.version,
    dependencies: v1.dependencies ?? [],
    manifestVersion: 2,
    contentTypes: [ContentType.Buildings],
    author: undefined,
    description: undefined,
    engineVersion: undefined,
  };
}

// ---- Validation ----

/** Semver pattern: major.minor.patch with optional pre-release. */
const SEMVER_PATTERN = /^\d+\.\d+\.\d+(-[\w.]+)?$/;

/**
 * Validate an unknown value as a well-formed ManifestV2.
 * Returns an object with a valid flag and an array of error messages.
 */
export function validateManifestV2(
  manifest: unknown,
): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if (manifest === null || manifest === undefined || typeof manifest !== "object") {
    return { valid: false, errors: ["Manifest must be a non-null object"] };
  }

  const m = manifest as Record<string, unknown>;

  // Required string fields
  if (typeof m.id !== "string" || m.id.length === 0) {
    errors.push("'id' must be a non-empty string");
  }

  if (typeof m.name !== "string" || m.name.length === 0) {
    errors.push("'name' must be a non-empty string");
  }

  if (typeof m.version !== "string" || !SEMVER_PATTERN.test(m.version as string)) {
    errors.push("'version' must match semver pattern (e.g. 1.0.0)");
  }

  // manifestVersion must be 2
  if (m.manifestVersion !== 2) {
    errors.push("'manifestVersion' must be 2");
  }

  // contentTypes must be a non-empty array of valid ContentType values
  if (!Array.isArray(m.contentTypes)) {
    errors.push("'contentTypes' must be an array");
  } else if (m.contentTypes.length === 0) {
    errors.push("'contentTypes' must contain at least one content type");
  } else {
    for (const ct of m.contentTypes) {
      if (typeof ct !== "string" || !ALL_CONTENT_TYPES.has(ct)) {
        errors.push(`Invalid content type: "${ct}"`);
      }
    }
  }

  // Optional dependencies must be an array of strings if present
  if (m.dependencies !== undefined) {
    if (!Array.isArray(m.dependencies)) {
      errors.push("'dependencies' must be an array if specified");
    } else {
      for (const dep of m.dependencies) {
        if (typeof dep !== "string") {
          errors.push("Each dependency must be a string");
        }
      }
    }
  }

  // Optional engineVersion must be a non-empty string if present
  if (m.engineVersion !== undefined && m.engineVersion !== null) {
    if (typeof m.engineVersion !== "string" || m.engineVersion.length === 0) {
      errors.push("'engineVersion' must be a non-empty string if specified");
    }
  }

  // Optional author must be a string if present
  if (m.author !== undefined && m.author !== null) {
    if (typeof m.author !== "string") {
      errors.push("'author' must be a string if specified");
    }
  }

  // Optional description must be a string if present
  if (m.description !== undefined && m.description !== null) {
    if (typeof m.description !== "string") {
      errors.push("'description' must be a string if specified");
    }
  }

  return { valid: errors.length === 0, errors };
}
