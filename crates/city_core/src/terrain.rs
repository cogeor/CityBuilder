//! Terrain — registry-based terrain types.
//!
//! The engine uses `TerrainId` (u8) to identify terrain types.
//! Game plugins register terrain definitions at startup.

use serde::{Deserialize, Serialize};

/// Terrain type identifier. Game plugins define what each ID means.
pub type TerrainId = u8;

/// Definition of a terrain type, registered by game plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainDef {
    pub id: TerrainId,
    pub name: String,
    pub walkable: bool,
    pub buildable: bool,
    /// Sprite ID for this terrain (used by renderer).
    pub sprite_id: u16,
}

/// Registry of all terrain types. Acts as a Resource in the App.
#[derive(Debug, Default)]
pub struct TerrainRegistry {
    defs: Vec<TerrainDef>,
}

impl TerrainRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a terrain definition. Panics if ID already registered.
    pub fn register(&mut self, def: TerrainDef) {
        if self.defs.iter().any(|d| d.id == def.id) {
            panic!("Terrain ID {} '{}' already registered", def.id, def.name);
        }
        self.defs.push(def);
    }

    /// Look up a terrain definition by ID.
    pub fn get(&self, id: TerrainId) -> Option<&TerrainDef> {
        self.defs.iter().find(|d| d.id == id)
    }

    /// All registered terrain definitions.
    pub fn all(&self) -> &[TerrainDef] {
        &self.defs
    }

    pub fn count(&self) -> usize {
        self.defs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup() {
        let mut reg = TerrainRegistry::new();
        reg.register(TerrainDef {
            id: 0,
            name: "Grass".into(),
            walkable: true,
            buildable: true,
            sprite_id: 100,
        });
        reg.register(TerrainDef {
            id: 1,
            name: "Water".into(),
            walkable: false,
            buildable: false,
            sprite_id: 101,
        });

        assert_eq!(reg.count(), 2);
        assert_eq!(reg.get(0).unwrap().name, "Grass");
        assert!(reg.get(1).unwrap().walkable == false);
        assert!(reg.get(5).is_none());
    }

    #[test]
    #[should_panic(expected = "already registered")]
    fn duplicate_panics() {
        let mut reg = TerrainRegistry::new();
        let def = TerrainDef {
            id: 0,
            name: "Grass".into(),
            walkable: true,
            buildable: true,
            sprite_id: 100,
        };
        reg.register(def.clone());
        reg.register(def);
    }
}
