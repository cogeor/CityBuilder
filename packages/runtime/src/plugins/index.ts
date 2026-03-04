// @townbuilder/runtime — plugins barrel export
export {
  // Types
  type PluginContentType,
  type PluginRuntimeKind,
  type PluginActivation,
  type PluginManifestSchemaVersion,
  type PluginManifestSourceFormat,
  type PluginManifest,
  type PluginEntry,
  normalizeManifest,

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

// Plugin host abstraction
export {
  type IPluginSource,
  type PluginActivationReport,
  PluginHost,
  InMemoryPluginSource,
} from "./host.js";

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
