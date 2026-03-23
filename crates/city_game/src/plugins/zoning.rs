//! ZoningPlugin — registers zone definitions as the single source of truth.

use city_core::{App, Plugin};
use city_sim::types::ZoneType;

/// Zone definition with display metadata.
#[derive(Debug, Clone)]
pub struct ZoneDef {
    pub zone_type: ZoneType,
    pub display_name: &'static str,
    pub color_rgb: [f32; 3],
}

/// Registry of zone definitions.
#[derive(Debug, Default)]
pub struct ZoneDefRegistry {
    zones: Vec<ZoneDef>,
}

impl ZoneDefRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, def: ZoneDef) {
        self.zones.push(def);
    }

    pub fn get(&self, zone_type: ZoneType) -> Option<&ZoneDef> {
        self.zones.iter().find(|z| z.zone_type == zone_type)
    }

    pub fn all(&self) -> &[ZoneDef] { &self.zones }
}

pub struct ZoningPlugin;

impl Plugin for ZoningPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = ZoneDefRegistry::new();
        registry.register(ZoneDef {
            zone_type: ZoneType::Residential,
            display_name: "Residential",
            color_rgb: [0.3, 0.7, 0.3], // green
        });
        registry.register(ZoneDef {
            zone_type: ZoneType::Commercial,
            display_name: "Commercial",
            color_rgb: [0.3, 0.3, 0.8], // blue
        });
        registry.register(ZoneDef {
            zone_type: ZoneType::Industrial,
            display_name: "Industrial",
            color_rgb: [0.7, 0.6, 0.2], // yellow
        });
        app.insert_resource(registry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_core::App;

    #[test]
    fn zone_def_registry_has_three_zones() {
        let mut app = App::new();
        ZoningPlugin.build(&mut app);
        let registry = app
            .get_resource::<ZoneDefRegistry>()
            .expect("ZoneDefRegistry should be inserted by ZoningPlugin");
        assert_eq!(registry.all().len(), 3);
        assert!(registry.get(ZoneType::Residential).is_some());
        assert!(registry.get(ZoneType::Commercial).is_some());
        assert!(registry.get(ZoneType::Industrial).is_some());
    }
}
