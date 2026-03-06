//! TerrainPlugin — registers base terrain types for the game.

use city_core::{App, Plugin};
use city_core::terrain::{TerrainDef, TerrainRegistry};

/// Registers the standard terrain types: grass, water, sand, forest, rock.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = TerrainRegistry::new();

        registry.register(TerrainDef {
            id: 0,
            name: "Grass".into(),
            walkable: true,
            buildable: true,
            sprite_id: 0,
        });
        registry.register(TerrainDef {
            id: 1,
            name: "Water".into(),
            walkable: false,
            buildable: false,
            sprite_id: 1,
        });
        registry.register(TerrainDef {
            id: 2,
            name: "Sand".into(),
            walkable: true,
            buildable: true,
            sprite_id: 2,
        });
        registry.register(TerrainDef {
            id: 3,
            name: "Forest".into(),
            walkable: true,
            buildable: false,
            sprite_id: 3,
        });
        registry.register(TerrainDef {
            id: 4,
            name: "Rock".into(),
            walkable: false,
            buildable: false,
            sprite_id: 4,
        });

        app.insert_resource(registry);
    }
}
