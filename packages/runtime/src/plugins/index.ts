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

// Plugin validation pipeline
export * from "./validators/index.js";
