//! ArchetypeContentPlugin — registers game archetypes into the ArchetypeRegistry.

use city_core::{App, Plugin};
use city_sim::archetype::ArchetypeRegistry;

use crate::scenario::register_fast_archetypes;

/// Registers the built-in archetypes (houses, shops, factories) with the
/// [`ArchetypeRegistry`] resource. Must be added after any plugin that inserts
/// the registry (e.g. `SimCorePlugin`).
pub struct ArchetypeContentPlugin;

impl Plugin for ArchetypeContentPlugin {
    fn build(&self, app: &mut App) {
        let registry = app
            .get_resource_mut::<ArchetypeRegistry>()
            .expect("ArchetypeRegistry not found — add SimCorePlugin before ArchetypeContentPlugin");
        register_fast_archetypes(registry);
    }
}
