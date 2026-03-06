// @townbuilder/base.economy — Barrel export
export {
  type TaxBracket,
  type WorkspaceDensity,
  type GrowthModifier,
  type DepartmentConfig,
  TAX_BRACKETS,
  WORKSPACE_DENSITY,
  GROWTH_MODIFIERS,
  DEPARTMENTS,
  getTaxBracket,
  getWorkspaceDensity,
  getDepartment,
  validateEconomyConfig,
} from "./economy.js";

export {
  type SupplyChainNode,
  SUPPLY_CHAIN_NODES,
  getNodeById,
  getNodeDependencies,
  validateSupplyChain,
} from "./advanced.js";
