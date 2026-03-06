//! Zone — registry-based zone types.
//!
//! Same pattern as terrain: `ZoneId` (u8) + `ZoneDef` + `ZoneRegistry`.
//! Game plugins register zone definitions at startup.

use serde::{Deserialize, Serialize};

/// Zone type identifier. Game plugins define what each ID means.
pub type ZoneId = u8;

/// Sentinel value: no zone assigned.
pub const ZONE_NONE: ZoneId = 0;

/// Definition of a zone type, registered by game plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDef {
    pub id: ZoneId,
    pub name: String,
    /// Display color for overlay rendering (RGBA).
    pub color: [u8; 4],
}

/// Registry of all zone types. Acts as a Resource in the App.
#[derive(Debug, Default)]
pub struct ZoneRegistry {
    defs: Vec<ZoneDef>,
}

impl ZoneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: ZoneDef) {
        if self.defs.iter().any(|d| d.id == def.id) {
            panic!("Zone ID {} '{}' already registered", def.id, def.name);
        }
        self.defs.push(def);
    }

    pub fn get(&self, id: ZoneId) -> Option<&ZoneDef> {
        self.defs.iter().find(|d| d.id == id)
    }

    pub fn all(&self) -> &[ZoneDef] {
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
        let mut reg = ZoneRegistry::new();
        reg.register(ZoneDef {
            id: 1,
            name: "Residential".into(),
            color: [0, 200, 0, 160],
        });
        assert_eq!(reg.get(1).unwrap().name, "Residential");
    }
}
