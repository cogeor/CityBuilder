//! RON-based content loader for archetype definitions.

use crate::archetype::ArchetypeDefinition;

/// Load archetype definitions from a RON string.
pub fn load_archetypes_ron(src: &str) -> Result<Vec<ArchetypeDefinition>, ron::error::SpannedError> {
    ron::from_str(src)
}

/// Load archetypes from the default content file.
/// On native: reads from filesystem. On WASM: uses embedded content.
pub fn load_default_archetypes() -> Vec<ArchetypeDefinition> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = "plugins/base.world/content/buildings.ron";
        match std::fs::read_to_string(path) {
            Ok(content) => load_archetypes_ron(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path, e)),
            Err(_) => {
                // Fallback to embedded content
                let content = include_str!("../../../plugins/base.world/content/buildings.ron");
                load_archetypes_ron(content)
                    .unwrap_or_else(|e| panic!("Failed to parse embedded buildings.ron: {}", e))
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let content = include_str!("../../../plugins/base.world/content/buildings.ron");
        load_archetypes_ron(content)
            .unwrap_or_else(|e| panic!("Failed to parse embedded buildings.ron: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_file_parses() {
        let content = include_str!("../../../plugins/base.world/content/buildings.ron");
        let archetypes = load_archetypes_ron(content).expect("buildings.ron should parse");
        assert!(archetypes.len() >= 5, "Expected at least 5 archetypes, got {}", archetypes.len());
    }
}
