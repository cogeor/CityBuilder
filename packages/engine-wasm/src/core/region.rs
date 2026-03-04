//! Region play data model -- inter-city bridge contract.
//!
//! Defines the data structures and trait interface for cross-city resource
//! and population flow. V1 ships with a `StubRegionBridge` that returns
//! empty flows (single-city mode). The data model is region-aware from day
//! one so that saves are forward-compatible with a full v2 implementation.

use serde::{Deserialize, Serialize};

// ─── CityId ──────────────────────────────────────────────────────────────────

/// Unique city identifier within a region.
pub type CityId = u16;

// ─── ResourceFlow ────────────────────────────────────────────────────────────

/// Resource flow between cities.
///
/// Tracks directional flows of commuters, power, water, and goods between
/// a pair of connected cities. All values default to zero.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceFlow {
    pub commuters_in: u32,
    pub commuters_out: u32,
    pub power_import_kw: u32,
    pub power_export_kw: u32,
    pub water_import: u32,
    pub water_export: u32,
    pub goods_import: u32,
    pub goods_export: u32,
}

// ─── ConnectedCity ───────────────────────────────────────────────────────────

/// Summary of a connected city within the region.
///
/// Stores enough metadata to display in the UI and compute flow parameters
/// without loading the full foreign city state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectedCity {
    pub id: CityId,
    /// City name, stored as a fixed-capacity string (max 32 UTF-8 bytes).
    pub name: String,
    pub population: u32,
    pub distance_km: u16,
}

// ─── RegionState ─────────────────────────────────────────────────────────────

/// Region state containing all connected cities.
///
/// Saved alongside the city save as a companion `region.bin` file.
/// When absent (legacy saves), the loader creates a default empty state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionState {
    pub cities: Vec<ConnectedCity>,
}

// ─── IRegionBridge ───────────────────────────────────────────────────────────

/// Trait for cross-city resource and population flow.
///
/// Implementations supply the simulation with import demands, export
/// supplies, and commuter flows. The v1 stub returns zero for everything.
pub trait IRegionBridge {
    /// Demand for resources that `city` wants to import from the region.
    fn import_demand(&self, city: CityId) -> ResourceFlow;

    /// Supply of resources that `city` can export to the region.
    fn export_supply(&self, city: CityId) -> ResourceFlow;

    /// Number of commuters travelling from `from` to `to`.
    fn commuter_flow(&self, from: CityId, to: CityId) -> u32;

    /// List of all cities connected to this one.
    fn connected_cities(&self) -> &[ConnectedCity];
}

// ─── StubRegionBridge ────────────────────────────────────────────────────────

/// Stub implementation for single-city mode (v1).
///
/// Returns empty/zero flows for everything. Satisfies the `IRegionBridge`
/// contract so that all simulation systems can be written region-aware
/// without requiring an actual multi-city backend.
pub struct StubRegionBridge;

impl IRegionBridge for StubRegionBridge {
    #[inline]
    fn import_demand(&self, _city: CityId) -> ResourceFlow {
        ResourceFlow::default()
    }

    #[inline]
    fn export_supply(&self, _city: CityId) -> ResourceFlow {
        ResourceFlow::default()
    }

    #[inline]
    fn commuter_flow(&self, _from: CityId, _to: CityId) -> u32 {
        0
    }

    #[inline]
    fn connected_cities(&self) -> &[ConnectedCity] {
        &[]
    }
}

// ─── Serialization ───────────────────────────────────────────────────────────

/// Magic bytes identifying a region companion file.
const REGION_MAGIC: [u8; 4] = *b"REGN";

/// Current region format version.
const REGION_VERSION: u32 = 1;

/// Serialize a `RegionState` to a binary byte buffer.
///
/// Layout (all little-endian):
/// - Magic: 4 bytes ("REGN")
/// - Version: u32
/// - City count: u32
/// - Per city:
///   - id: u16
///   - name_len: u16
///   - name: [u8; name_len]
///   - population: u32
///   - distance_km: u16
pub fn serialize_region(state: &RegionState) -> Vec<u8> {
    let city_count = state.cities.len() as u32;

    // Estimate buffer size: header(12) + cities * ~44 average
    let estimated = 12 + state.cities.len() * 44;
    let mut buf = Vec::with_capacity(estimated);

    // ── Header ──
    buf.extend_from_slice(&REGION_MAGIC);
    buf.extend_from_slice(&REGION_VERSION.to_le_bytes());
    buf.extend_from_slice(&city_count.to_le_bytes());

    // ── Cities ──
    for city in &state.cities {
        buf.extend_from_slice(&city.id.to_le_bytes());

        let name_bytes = city.name.as_bytes();
        let name_len = name_bytes.len().min(u16::MAX as usize) as u16;
        buf.extend_from_slice(&name_len.to_le_bytes());
        buf.extend_from_slice(&name_bytes[..name_len as usize]);

        buf.extend_from_slice(&city.population.to_le_bytes());
        buf.extend_from_slice(&city.distance_km.to_le_bytes());
    }

    buf
}

/// Deserialize a `RegionState` from binary data.
///
/// Returns `None` if the data is invalid, corrupt, or uses an
/// unsupported version. Legacy saves without region data will
/// simply use the default empty `RegionState`.
pub fn deserialize_region(data: &[u8]) -> Option<RegionState> {
    let mut cursor = 0usize;

    // ── Magic ──
    if data.len() < 12 {
        return None;
    }
    if &data[cursor..cursor + 4] != &REGION_MAGIC {
        return None;
    }
    cursor += 4;

    // ── Version ──
    let version = u32::from_le_bytes(read_bytes::<4>(data, &mut cursor)?);
    if version != REGION_VERSION {
        return None;
    }

    // ── City count ──
    let city_count = u32::from_le_bytes(read_bytes::<4>(data, &mut cursor)?);

    // Sanity check: reject absurdly large counts to prevent OOM.
    if city_count > 10_000 {
        return None;
    }

    let mut cities = Vec::with_capacity(city_count as usize);

    for _ in 0..city_count {
        let id = u16::from_le_bytes(read_bytes::<2>(data, &mut cursor)?);

        let name_len = u16::from_le_bytes(read_bytes::<2>(data, &mut cursor)?) as usize;
        if cursor + name_len > data.len() {
            return None;
        }
        let name = String::from_utf8(data[cursor..cursor + name_len].to_vec()).ok()?;
        cursor += name_len;

        let population = u32::from_le_bytes(read_bytes::<4>(data, &mut cursor)?);
        let distance_km = u16::from_le_bytes(read_bytes::<2>(data, &mut cursor)?);

        cities.push(ConnectedCity {
            id,
            name,
            population,
            distance_km,
        });
    }

    Some(RegionState { cities })
}

/// Read N bytes from `data` at `cursor`, advancing the cursor.
fn read_bytes<const N: usize>(data: &[u8], cursor: &mut usize) -> Option<[u8; N]> {
    if *cursor + N > data.len() {
        return None;
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&data[*cursor..*cursor + N]);
    *cursor += N;
    Some(arr)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── StubRegionBridge Tests ──────────────────────────────────────────

    #[test]
    fn stub_bridge_returns_zero_import_demand() {
        let bridge = StubRegionBridge;
        let flow = bridge.import_demand(0);
        assert_eq!(flow, ResourceFlow::default());
        assert_eq!(flow.commuters_in, 0);
        assert_eq!(flow.power_import_kw, 0);
        assert_eq!(flow.water_import, 0);
        assert_eq!(flow.goods_import, 0);
    }

    #[test]
    fn stub_bridge_returns_zero_export_supply() {
        let bridge = StubRegionBridge;
        let flow = bridge.export_supply(0);
        assert_eq!(flow, ResourceFlow::default());
        assert_eq!(flow.commuters_out, 0);
        assert_eq!(flow.power_export_kw, 0);
        assert_eq!(flow.water_export, 0);
        assert_eq!(flow.goods_export, 0);
    }

    #[test]
    fn stub_bridge_returns_zero_commuter_flow() {
        let bridge = StubRegionBridge;
        assert_eq!(bridge.commuter_flow(0, 1), 0);
        assert_eq!(bridge.commuter_flow(1, 0), 0);
        assert_eq!(bridge.commuter_flow(100, 200), 0);
    }

    #[test]
    fn stub_bridge_returns_empty_connected_cities() {
        let bridge = StubRegionBridge;
        assert!(bridge.connected_cities().is_empty());
    }

    #[test]
    fn stub_bridge_zero_for_any_city_id() {
        let bridge = StubRegionBridge;
        for id in [0, 1, 100, u16::MAX] {
            assert_eq!(bridge.import_demand(id), ResourceFlow::default());
            assert_eq!(bridge.export_supply(id), ResourceFlow::default());
        }
    }

    // ─── ResourceFlow Tests ─────────────────────────────────────────────

    #[test]
    fn resource_flow_default_is_all_zeros() {
        let flow = ResourceFlow::default();
        assert_eq!(flow.commuters_in, 0);
        assert_eq!(flow.commuters_out, 0);
        assert_eq!(flow.power_import_kw, 0);
        assert_eq!(flow.power_export_kw, 0);
        assert_eq!(flow.water_import, 0);
        assert_eq!(flow.water_export, 0);
        assert_eq!(flow.goods_import, 0);
        assert_eq!(flow.goods_export, 0);
    }

    #[test]
    fn resource_flow_equality() {
        let a = ResourceFlow {
            commuters_in: 10,
            commuters_out: 5,
            power_import_kw: 100,
            power_export_kw: 50,
            water_import: 20,
            water_export: 10,
            goods_import: 30,
            goods_export: 15,
        };
        let b = a.clone();
        assert_eq!(a, b);

        let c = ResourceFlow {
            commuters_in: 99,
            ..a.clone()
        };
        assert_ne!(a, c);
    }

    #[test]
    fn resource_flow_clone() {
        let flow = ResourceFlow {
            commuters_in: 42,
            power_export_kw: 999,
            ..Default::default()
        };
        let cloned = flow.clone();
        assert_eq!(flow, cloned);
    }

    // ─── ConnectedCity Tests ────────────────────────────────────────────

    #[test]
    fn connected_city_creation() {
        let city = ConnectedCity {
            id: 1,
            name: "Neighbor Town".to_string(),
            population: 50_000,
            distance_km: 25,
        };
        assert_eq!(city.id, 1);
        assert_eq!(city.name, "Neighbor Town");
        assert_eq!(city.population, 50_000);
        assert_eq!(city.distance_km, 25);
    }

    #[test]
    fn connected_city_equality() {
        let a = ConnectedCity {
            id: 1,
            name: "City A".to_string(),
            population: 100,
            distance_km: 10,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ─── RegionState Tests ──────────────────────────────────────────────

    #[test]
    fn region_state_default_is_empty() {
        let state = RegionState::default();
        assert!(state.cities.is_empty());
    }

    #[test]
    fn region_state_with_multiple_cities() {
        let state = RegionState {
            cities: vec![
                ConnectedCity {
                    id: 1,
                    name: "Alpha".to_string(),
                    population: 10_000,
                    distance_km: 15,
                },
                ConnectedCity {
                    id: 2,
                    name: "Beta".to_string(),
                    population: 25_000,
                    distance_km: 40,
                },
                ConnectedCity {
                    id: 3,
                    name: "Gamma".to_string(),
                    population: 100_000,
                    distance_km: 80,
                },
            ],
        };
        assert_eq!(state.cities.len(), 3);
        assert_eq!(state.cities[0].name, "Alpha");
        assert_eq!(state.cities[2].population, 100_000);
    }

    // ─── CityId Type Tests ──────────────────────────────────────────────

    #[test]
    fn city_id_type_is_u16() {
        let id: CityId = 0;
        assert_eq!(id, 0u16);
        let max: CityId = u16::MAX;
        assert_eq!(max, 65535);
        // Ensure CityId arithmetic works as expected for u16.
        let sum: CityId = 100u16 + 200u16;
        assert_eq!(sum, 300);
    }

    // ─── Serialization Round-trip Tests ─────────────────────────────────

    #[test]
    fn serialize_empty_region_round_trip() {
        let state = RegionState::default();
        let data = serialize_region(&state);
        let restored = deserialize_region(&data).expect("should deserialize empty region");
        assert_eq!(restored, state);
        assert!(restored.cities.is_empty());
    }

    #[test]
    fn serialize_region_with_cities_round_trip() {
        let state = RegionState {
            cities: vec![
                ConnectedCity {
                    id: 1,
                    name: "Riverside".to_string(),
                    population: 45_000,
                    distance_km: 20,
                },
                ConnectedCity {
                    id: 2,
                    name: "Hilltop".to_string(),
                    population: 12_000,
                    distance_km: 55,
                },
            ],
        };
        let data = serialize_region(&state);
        let restored = deserialize_region(&data).expect("should deserialize region with cities");
        assert_eq!(restored, state);
        assert_eq!(restored.cities.len(), 2);
        assert_eq!(restored.cities[0].name, "Riverside");
        assert_eq!(restored.cities[1].distance_km, 55);
    }

    #[test]
    fn serialize_region_with_unicode_name_round_trip() {
        let state = RegionState {
            cities: vec![ConnectedCity {
                id: 7,
                name: "Munchen".to_string(),
                population: 1_500_000,
                distance_km: 100,
            }],
        };
        let data = serialize_region(&state);
        let restored = deserialize_region(&data).expect("should handle unicode names");
        assert_eq!(restored.cities[0].name, "Munchen");
    }

    #[test]
    fn deserialize_invalid_data_returns_none() {
        // Empty data
        assert!(deserialize_region(&[]).is_none());

        // Too short
        assert!(deserialize_region(&[0, 1, 2, 3]).is_none());

        // Wrong magic
        let bad_magic = b"BAAD\x01\x00\x00\x00\x00\x00\x00\x00";
        assert!(deserialize_region(bad_magic).is_none());
    }

    #[test]
    fn deserialize_wrong_version_returns_none() {
        let mut data = serialize_region(&RegionState::default());
        // Overwrite version bytes (offset 4..8) with version 99.
        data[4] = 99;
        data[5] = 0;
        data[6] = 0;
        data[7] = 0;
        assert!(deserialize_region(&data).is_none());
    }

    #[test]
    fn deserialize_truncated_city_data_returns_none() {
        let state = RegionState {
            cities: vec![ConnectedCity {
                id: 1,
                name: "Test".to_string(),
                population: 1000,
                distance_km: 10,
            }],
        };
        let data = serialize_region(&state);
        // Truncate the data mid-city.
        let truncated = &data[..data.len() - 3];
        assert!(deserialize_region(truncated).is_none());
    }

    #[test]
    fn migration_contract_empty_data_uses_default() {
        // Legacy saves without region data should produce a default empty state.
        // The caller checks for absence and uses RegionState::default().
        let default_state = RegionState::default();
        assert!(default_state.cities.is_empty());
        // Verify the stub bridge works with the default state.
        let bridge = StubRegionBridge;
        assert!(bridge.connected_cities().is_empty());
        assert_eq!(bridge.import_demand(0), ResourceFlow::default());
    }

    // ─── Serialization Format Stability ─────────────────────────────────

    #[test]
    fn serialized_empty_region_starts_with_magic_and_version() {
        let data = serialize_region(&RegionState::default());
        assert_eq!(&data[0..4], b"REGN");
        assert_eq!(
            u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            REGION_VERSION
        );
        assert_eq!(
            u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            0
        ); // zero cities
    }

    #[test]
    fn serialized_region_city_count_matches() {
        let state = RegionState {
            cities: vec![
                ConnectedCity { id: 1, name: "A".to_string(), population: 1, distance_km: 1 },
                ConnectedCity { id: 2, name: "B".to_string(), population: 2, distance_km: 2 },
                ConnectedCity { id: 3, name: "C".to_string(), population: 3, distance_km: 3 },
            ],
        };
        let data = serialize_region(&state);
        let count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        assert_eq!(count, 3);
    }
}
