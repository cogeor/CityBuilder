/** Node in a supply chain graph */
export interface SupplyChainNode {
  id: string;
  name: string;
  inputResources: string[];
  outputResource: string;
  processingTicks: number;
  requiredWorkers: number;
}

export const SUPPLY_CHAIN_NODES: SupplyChainNode[] = [
  { id: "raw_materials", name: "Raw Materials", inputResources: [], outputResource: "raw", processingTicks: 10, requiredWorkers: 5 },
  { id: "manufacturing", name: "Manufacturing", inputResources: ["raw"], outputResource: "goods", processingTicks: 20, requiredWorkers: 10 },
  { id: "retail", name: "Retail", inputResources: ["goods"], outputResource: "consumer_goods", processingTicks: 5, requiredWorkers: 3 },
];

/** Get a supply chain node by its id */
export function getNodeById(id: string): SupplyChainNode | undefined {
  return SUPPLY_CHAIN_NODES.find(n => n.id === id);
}

/** Get the input resource dependencies for a node (returns inputResources) */
export function getNodeDependencies(id: string): string[] {
  const node = getNodeById(id);
  return node?.inputResources ?? [];
}

/** Validate a supply chain for consistency — checks that all input references exist as outputs */
export function validateSupplyChain(nodes: SupplyChainNode[]): { valid: boolean; errors: string[] } {
  const errors: string[] = [];
  const outputSet = new Set(nodes.map(n => n.outputResource));

  for (const node of nodes) {
    for (const input of node.inputResources) {
      if (!outputSet.has(input)) {
        errors.push(`Node "${node.id}" requires input "${input}" which is not produced by any node`);
      }
    }
    if (node.processingTicks <= 0) {
      errors.push(`Node "${node.id}" has invalid processingTicks: ${node.processingTicks}`);
    }
    if (node.requiredWorkers <= 0) {
      errors.push(`Node "${node.id}" has invalid requiredWorkers: ${node.requiredWorkers}`);
    }
  }

  return { valid: errors.length === 0, errors };
}
