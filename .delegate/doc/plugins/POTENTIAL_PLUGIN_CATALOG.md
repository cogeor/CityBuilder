# TownBuilder Plugin Expansion Catalog

## Current Plugin Support Matrix

| Plugin ID | Content Types | Status | Dependencies |
|-----------|--------------|--------|--------------|
| base.buildings | buildings | Implemented | none |
| base.world | terrain, presets | Implemented | none |
| base.terrain | terrain | Implemented | none |
| base.networks | networks, roads | Implemented | none |
| base.economy | economy | Implemented | none |
| base.progression | scenarios, events | Implemented | none |

## Planned Extensions

| Plugin ID | Content Types | Priority | Risk | Dependencies |
|-----------|--------------|----------|------|--------------|
| base.demographics | demographics | High | Low | base.economy |
| base.governance | governance | Medium | Medium | base.economy |
| base.advanced-economy | economy | Medium | High | base.economy |
| base.multimodal | networks | Medium | High | base.networks |
| base.ai-advisor | ai | Low | Medium | base.progression |

## Rollout Order
1. base.demographics (low risk, high value)
2. base.governance (medium risk, gameplay depth)
3. base.advanced-economy (high risk, needs balancing)
4. base.multimodal (high risk, complex pathfinding)
5. base.ai-advisor (medium risk, depends on metrics)

## CI Compatibility Checks
- Schema validation on all manifest.json files
- Dependency resolution test on plugin set
- Content type conflict detection
- Save format compatibility verification
