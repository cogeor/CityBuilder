// @townbuilder/runtime — plugins barrel export
export {
  // Types
  type PluginContentType,
  type PluginActivation,
  type PluginManifest,
  type PluginEntry,

  // Validation
  validateManifest,
} from "./manifest.js";

export {
  // Classes
  PluginRegistry,

  // Functions
  resolveDependencies,
  validateDependencies,
} from "./loader.js";

// Plugin manifest v2 — extended content types
export {
  ContentType,
  type ManifestV1,
  type ManifestV2,
  type PluginManifestAny,
  isManifestV2,
  migrateV1toV2,
  validateManifestV2,
} from "./manifest_v2.js";

// Plugin capability model
export {
  PluginCapability,
  type CapabilityRequirement,
  EXCLUSIVE_CAPABILITIES,
  validateCapabilities,
  hasCapability,
  CapabilityManager,
} from "./capabilities.js";

// Extension points and runtime extension registry
export {
  ExtensionPointType,
  ExtensionLifecycle,
  type ExtensionPoint,
  ExtensionRegistry,
} from "./extension_points.js";

// Plugin load-order and override policy
export {
  PluginPrecedence,
  type PluginLoadEntry,
  type LoadCheckpoint,
  LoadPolicyManager,
  comparePrec,
} from "./load_policy.js";

// Plugin validation pipeline
export * from "./validators/index.js";
