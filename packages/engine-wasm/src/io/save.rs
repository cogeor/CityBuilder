//! Binary save/load serialization for WorldState.
//!
//! Uses a deterministic, little-endian binary format with a versioned header.
//! No external serialization dependencies -- raw byte writing for full control.

use crate::core::entity::EntityStore;
use crate::core::tilemap::{TileFlags, TileKind, TileMap, TileValue};
use crate::core::world::{CityPolicies, WorldSeeds, WorldState};
use crate::core_types::*;
use std::fmt;

// ─── Constants ──────────────────────────────────────────────────────────────

/// Current save format version.
pub const SAVE_VERSION: u32 = 2;

/// Magic bytes identifying a TownBuilder save file.
const MAGIC: [u8; 4] = *b"TOWN";

/// Fixed header size in bytes (excluding city_name).
const HEADER_SIZE: usize = 4 + 4 + 2 + 2 + 4 + 8 + 8 + 8 + 2; // 42 bytes

// ─── SaveError ──────────────────────────────────────────────────────────────

/// Errors that can occur during deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The magic bytes do not match "TOWN".
    InvalidMagic,
    /// The save version is not supported by this build.
    UnsupportedVersion(u32),
    /// Data is structurally corrupt or inconsistent.
    CorruptData(String),
    /// Not enough bytes to read the expected data.
    InsufficientData,
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::InvalidMagic => write!(f, "invalid magic bytes (expected TOWN)"),
            SaveError::UnsupportedVersion(v) => {
                write!(
                    f,
                    "unsupported save version: {} (expected {})",
                    v, SAVE_VERSION
                )
            }
            SaveError::CorruptData(msg) => write!(f, "corrupt save data: {}", msg),
            SaveError::InsufficientData => write!(f, "insufficient data in save file"),
        }
    }
}

impl From<SaveError> for String {
    fn from(err: SaveError) -> Self {
        err.to_string()
    }
}

// ─── SaveHeader ─────────────────────────────────────────────────────────────

/// Binary save file header. Appears at the start of every save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub map_width: u16,
    pub map_height: u16,
    pub entity_count: u32,
    pub tick: u64,
    pub treasury: i64,
    pub seed: u64,
    pub city_name_len: u16,
}

// ─── Serialize ──────────────────────────────────────────────────────────────

/// Serialize a WorldState into a binary byte buffer.
///
/// Layout (all little-endian):
/// - Header (42 bytes)
/// - City name (variable, `city_name_len` bytes)
/// - CityPolicies (8 bytes, 8 sequential u8 fields)
/// - Tile data (5 bytes per tile: terrain, kind, zone, density, flags)
/// - Entity data (11 bytes per alive entity)
pub fn serialize_world(world: &WorldState) -> Vec<u8> {
    let map_w = world.tiles.width() as u16;
    let map_h = world.tiles.height() as u16;
    let entity_count = world.entities.count();
    let name_bytes = world.city_name.as_bytes();
    let name_len = name_bytes.len().min(u16::MAX as usize) as u16;

    let tile_count = map_w as usize * map_h as usize;
    let estimated_size =
        HEADER_SIZE + name_len as usize + 8 + tile_count * 5 + entity_count as usize * 11;
    let mut buf = Vec::with_capacity(estimated_size);

    // ── Header ──
    buf.extend_from_slice(&MAGIC);
    buf.extend_from_slice(&SAVE_VERSION.to_le_bytes());
    buf.extend_from_slice(&map_w.to_le_bytes());
    buf.extend_from_slice(&map_h.to_le_bytes());
    buf.extend_from_slice(&entity_count.to_le_bytes());
    buf.extend_from_slice(&world.tick.to_le_bytes());
    buf.extend_from_slice(&world.treasury.to_le_bytes());
    buf.extend_from_slice(&world.seeds.global_seed.to_le_bytes());
    buf.extend_from_slice(&name_len.to_le_bytes());

    // ── City name ──
    buf.extend_from_slice(&name_bytes[..name_len as usize]);

    // ── CityPolicies (8 u8 fields) ──
    buf.push(world.policies.residential_tax_pct);
    buf.push(world.policies.commercial_tax_pct);
    buf.push(world.policies.industrial_tax_pct);
    buf.push(world.policies.police_budget_pct);
    buf.push(world.policies.fire_budget_pct);
    buf.push(world.policies.health_budget_pct);
    buf.push(world.policies.education_budget_pct);
    buf.push(world.policies.transport_budget_pct);

    // ── Tile data (row-major: y then x) ──
    for y in 0..map_h as u32 {
        for x in 0..map_w as u32 {
            if let Some(tile) = world.tiles.get(x, y) {
                buf.push(tile.terrain  as u8);
                buf.push(tile.kind     as u8);
                buf.push(tile.zone     as u8);
                buf.push(tile.density  as u8);
                buf.push(tile.flags.bits());
            }
        }
    }

    // ── Entity data (only alive entities) ──
    for handle in world.entities.iter_alive() {
        let idx = handle.index as usize;
        buf.extend_from_slice(&world.entities.archetype_id[idx].to_le_bytes());
        buf.extend_from_slice(&world.entities.pos_x[idx].to_le_bytes());
        buf.extend_from_slice(&world.entities.pos_y[idx].to_le_bytes());
        buf.push(world.entities.rotation[idx]);
        buf.push(world.entities.level[idx]);
        buf.extend_from_slice(&world.entities.flags[idx].bits().to_le_bytes());
        buf.extend_from_slice(&world.entities.construction_progress[idx].to_le_bytes());
        buf.push(if world.entities.enabled[idx] { 1u8 } else { 0u8 });
    }

    buf
}

// ─── Deserialize helpers ────────────────────────────────────────────────────

/// A cursor for reading little-endian values from a byte slice.
struct ReadCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ReadCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        ReadCursor { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], SaveError> {
        if self.pos + n > self.data.len() {
            return Err(SaveError::InsufficientData);
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, SaveError> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> Result<u16, SaveError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_i16(&mut self) -> Result<i16, SaveError> {
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, SaveError> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, SaveError> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_i64(&mut self) -> Result<i64, SaveError> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

// ─── Enum conversion helpers ────────────────────────────────────────────────

fn terrain_from_u8(v: u8) -> Result<TerrainType, SaveError> {
    match v {
        0 => Ok(TerrainType::Grass),
        1 => Ok(TerrainType::Water),
        2 => Ok(TerrainType::Sand),
        3 => Ok(TerrainType::Forest),
        4 => Ok(TerrainType::Rock),
        _ => Err(SaveError::CorruptData(format!(
            "invalid terrain type: {}",
            v
        ))),
    }
}

fn kind_from_u8(b: u8) -> Result<TileKind, SaveError> {
    match b {
        0  => Ok(TileKind::Empty),
        1  => Ok(TileKind::Water),
        2  => Ok(TileKind::Nature),
        3  => Ok(TileKind::Rubble),
        4  => Ok(TileKind::Flood),
        5  => Ok(TileKind::Radiation),
        6  => Ok(TileKind::Fire),
        7  => Ok(TileKind::Road),
        8  => Ok(TileKind::PowerLine),
        9  => Ok(TileKind::Rail),
        10 => Ok(TileKind::Zone),
        11 => Ok(TileKind::Building),
        12 => Ok(TileKind::Port),
        13 => Ok(TileKind::Airport),
        14 => Ok(TileKind::Special),
        _ => Err(SaveError::CorruptData(format!("unknown TileKind byte: {}", b))),
    }
}

fn zone_from_u8(v: u8) -> Result<ZoneType, SaveError> {
    match v {
        0 => Ok(ZoneType::None),
        1 => Ok(ZoneType::Residential),
        2 => Ok(ZoneType::Commercial),
        3 => Ok(ZoneType::Industrial),
        4 => Ok(ZoneType::Civic),
        5 => Ok(ZoneType::Park),
        6 => Ok(ZoneType::Transport),
        _ => Err(SaveError::CorruptData(format!("invalid zone type: {}", v))),
    }
}

fn density_from_u8(v: u8) -> Result<ZoneDensity, SaveError> {
    match v {
        0 => Ok(ZoneDensity::Low),
        1 => Ok(ZoneDensity::Medium),
        2 => Ok(ZoneDensity::High),
        _ => Err(SaveError::CorruptData(format!("invalid zone density: {}", v))),
    }
}

// ─── Deserialize ────────────────────────────────────────────────────────────

/// Deserialize a binary byte buffer into a WorldState.
///
/// Returns `SaveError` for any validation failures.
pub fn deserialize_world(data: &[u8]) -> Result<WorldState, SaveError> {
    let mut cursor = ReadCursor::new(data);

    // ── Header ──
    let magic_bytes = cursor.read_bytes(4)?;
    if magic_bytes != MAGIC {
        return Err(SaveError::InvalidMagic);
    }

    let version = cursor.read_u32()?;
    if version != SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion(version));
    }

    let map_width = cursor.read_u16()?;
    let map_height = cursor.read_u16()?;
    let entity_count = cursor.read_u32()?;
    let tick = cursor.read_u64()?;
    let treasury = cursor.read_i64()?;
    let seed = cursor.read_u64()?;
    let city_name_len = cursor.read_u16()?;

    // ── City name ──
    let name_bytes = cursor.read_bytes(city_name_len as usize)?;
    let city_name = String::from_utf8(name_bytes.to_vec())
        .map_err(|e| SaveError::CorruptData(format!("invalid UTF-8 in city name: {}", e)))?;

    // ── CityPolicies ──
    let policies = CityPolicies {
        residential_tax_pct: cursor.read_u8()?,
        commercial_tax_pct: cursor.read_u8()?,
        industrial_tax_pct: cursor.read_u8()?,
        police_budget_pct: cursor.read_u8()?,
        fire_budget_pct: cursor.read_u8()?,
        health_budget_pct: cursor.read_u8()?,
        education_budget_pct: cursor.read_u8()?,
        transport_budget_pct: cursor.read_u8()?,
    };

    // ── Tile data ──
    let size = MapSize::new(map_width, map_height);
    let tile_count = size.area() as usize;

    // Validate we have enough data for tiles
    let tile_data_size = tile_count * 5;
    if cursor.remaining() < tile_data_size {
        return Err(SaveError::InsufficientData);
    }

    let mut tiles = TileMap::new(map_width as u32, map_height as u32);
    for y in 0..map_height as u32 {
        for x in 0..map_width as u32 {
            let terrain = terrain_from_u8(cursor.read_u8()?)?;
            let kind    = kind_from_u8(cursor.read_u8()?)?;
            let zone    = zone_from_u8(cursor.read_u8()?)?;
            let density = density_from_u8(cursor.read_u8()?)?;
            let flags   = TileFlags::from_bits_retain(cursor.read_u8()?);
            tiles.set(x, y, TileValue { terrain, kind, zone, density, flags });
        }
    }

    // ── Entity data ──
    let entity_data_size = entity_count as usize * 11;
    if cursor.remaining() < entity_data_size {
        return Err(SaveError::InsufficientData);
    }

    // Capacity must fit all entities; use at least tile_count (matching WorldState::new)
    let entity_capacity = tile_count.max(entity_count as usize);
    let mut entities = EntityStore::new(entity_capacity);

    for _ in 0..entity_count {
        let archetype = cursor.read_u16()?;
        let pos_x = cursor.read_i16()?;
        let pos_y = cursor.read_i16()?;
        let rotation = cursor.read_u8()?;
        let level = cursor.read_u8()?;
        let flags_raw = cursor.read_u16()?;
        let construction_progress = cursor.read_u16()?;
        let enabled_byte = cursor.read_u8()?;

        let handle = entities
            .alloc(archetype, pos_x, pos_y, rotation)
            .ok_or_else(|| {
                SaveError::CorruptData("entity store full during deserialization".to_string())
            })?;

        // Override defaults set by alloc with the actual saved values
        entities.set_level(handle, level);
        entities.set_flags(handle, StatusFlags::from_bits_retain(flags_raw));
        entities.set_construction_progress(handle, construction_progress);
        entities.set_enabled(handle, enabled_byte != 0);
    }

    Ok(WorldState {
        tiles,
        entities,
        policies,
        seeds: WorldSeeds::new(seed),
        tick,
        treasury,
        city_name,
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal world for testing.
    fn make_test_world(width: u16, height: u16, seed: u64) -> WorldState {
        WorldState::new(MapSize::new(width, height), seed)
    }

    // ── Test 1: Round-trip serialize/deserialize gives identical state ────

    #[test]
    fn round_trip_empty_world() {
        let world = make_test_world(4, 4, 42);
        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");

        assert_eq!(restored.tick, world.tick);
        assert_eq!(restored.treasury, world.treasury);
        assert_eq!(restored.city_name, world.city_name);
        assert_eq!(restored.seeds.global_seed, world.seeds.global_seed);
        assert_eq!(restored.tiles.width(), world.tiles.width());
        assert_eq!(restored.tiles.height(), world.tiles.height());
        assert_eq!(restored.entities.count(), 0);
    }

    // ── Test 2: Magic bytes validated ────────────────────────────────────

    #[test]
    fn invalid_magic_returns_error() {
        let world = make_test_world(2, 2, 1);
        let mut data = serialize_world(&world);
        // Corrupt magic bytes
        data[0] = b'X';
        data[1] = b'X';
        let err = deserialize_world(&data).unwrap_err();
        assert_eq!(err, SaveError::InvalidMagic);
    }

    // ── Test 3: Version check ────────────────────────────────────────────

    #[test]
    fn unsupported_version_returns_error() {
        let world = make_test_world(2, 2, 1);
        let mut data = serialize_world(&world);
        // Overwrite version to 99
        let version_bytes = 99u32.to_le_bytes();
        data[4..8].copy_from_slice(&version_bytes);
        let err = deserialize_world(&data).unwrap_err();
        assert_eq!(err, SaveError::UnsupportedVersion(99));
    }

    // ── Test 4: Empty world round-trip (with custom name) ────────────────

    #[test]
    fn empty_world_custom_name_round_trip() {
        let mut world = make_test_world(8, 8, 999);
        world.city_name = "Springfield".to_string();
        world.tick = 12345;
        world.treasury = -50_000;

        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");

        assert_eq!(restored.city_name, "Springfield");
        assert_eq!(restored.tick, 12345);
        assert_eq!(restored.treasury, -50_000);
    }

    // ── Test 5: World with entities round-trip ───────────────────────────

    #[test]
    fn world_with_entities_round_trip() {
        let mut world = make_test_world(16, 16, 7);

        // Place several entities with different properties
        let h1 = world.place_entity(10, 3, 4, 1).unwrap();
        let h2 = world.place_entity(20, 7, 8, 2).unwrap();
        let h3 = world.place_entity(30, 0, 0, 0).unwrap();

        // Customize entity fields
        world.entities.set_level(h1, 5);
        world
            .entities
            .set_flags(h1, StatusFlags::POWERED | StatusFlags::STAFFED);
        world.entities.set_construction_progress(h1, 0xFFFF);
        world.entities.set_enabled(h1, false);

        world.entities.set_level(h2, 3);
        world.entities.set_flags(h2, StatusFlags::ON_FIRE);
        world.entities.set_construction_progress(h2, 0x8000);

        world.entities.set_level(h3, 1);
        world.entities.set_flags(h3, StatusFlags::NONE);
        world.entities.set_construction_progress(h3, 0);

        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");

        assert_eq!(restored.entities.count(), 3);

        // Verify entities by iterating alive and checking properties
        let alive: Vec<EntityHandle> = restored.entities.iter_alive().collect();
        assert_eq!(alive.len(), 3);

        // Find entity with archetype 10
        let e1 = alive
            .iter()
            .find(|h| restored.entities.get_archetype(**h) == Some(10))
            .unwrap();
        assert_eq!(restored.entities.get_pos(*e1), Some(TileCoord::new(3, 4)));
        assert_eq!(restored.entities.get_level(*e1), Some(5));
        assert_eq!(
            restored.entities.get_flags(*e1),
            Some(StatusFlags::POWERED | StatusFlags::STAFFED)
        );
        assert_eq!(
            restored.entities.get_construction_progress(*e1),
            Some(0xFFFF)
        );
        assert_eq!(restored.entities.get_enabled(*e1), Some(false));

        // Find entity with archetype 20
        let e2 = alive
            .iter()
            .find(|h| restored.entities.get_archetype(**h) == Some(20))
            .unwrap();
        assert_eq!(restored.entities.get_pos(*e2), Some(TileCoord::new(7, 8)));
        assert_eq!(restored.entities.get_level(*e2), Some(3));
        assert_eq!(
            restored.entities.get_flags(*e2),
            Some(StatusFlags::ON_FIRE)
        );
        assert_eq!(
            restored.entities.get_construction_progress(*e2),
            Some(0x8000)
        );
    }

    // ── Test 6: Insufficient data error ──────────────────────────────────

    #[test]
    fn insufficient_data_returns_error() {
        // Too short to even read header
        let data = vec![b'T', b'O', b'W', b'N'];
        let err = deserialize_world(&data).unwrap_err();
        assert_eq!(err, SaveError::InsufficientData);
    }

    // ── Test 7: Invalid magic error ──────────────────────────────────────

    #[test]
    fn completely_wrong_magic() {
        let data = vec![0u8; 100];
        let err = deserialize_world(&data).unwrap_err();
        assert_eq!(err, SaveError::InvalidMagic);
    }

    // ── Test 8: Policy values preserved ──────────────────────────────────

    #[test]
    fn policy_values_preserved() {
        let mut world = make_test_world(4, 4, 1);
        world.policies.residential_tax_pct = 15;
        world.policies.commercial_tax_pct = 20;
        world.policies.industrial_tax_pct = 25;
        world.policies.police_budget_pct = 80;
        world.policies.fire_budget_pct = 90;
        world.policies.health_budget_pct = 110;
        world.policies.education_budget_pct = 120;
        world.policies.transport_budget_pct = 200;

        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");

        assert_eq!(restored.policies.residential_tax_pct, 15);
        assert_eq!(restored.policies.commercial_tax_pct, 20);
        assert_eq!(restored.policies.industrial_tax_pct, 25);
        assert_eq!(restored.policies.police_budget_pct, 80);
        assert_eq!(restored.policies.fire_budget_pct, 90);
        assert_eq!(restored.policies.health_budget_pct, 110);
        assert_eq!(restored.policies.education_budget_pct, 120);
        assert_eq!(restored.policies.transport_budget_pct, 200);
    }

    // ── Test 9: Entity flags preserved ───────────────────────────────────

    #[test]
    fn entity_flags_preserved() {
        let mut world = make_test_world(8, 8, 1);
        let h = world.place_entity(1, 2, 3, 0).unwrap();

        let flags = StatusFlags::POWERED
            | StatusFlags::HAS_WATER
            | StatusFlags::STAFFED
            | StatusFlags::DAMAGED;
        world.entities.set_flags(h, flags);

        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");

        let alive: Vec<EntityHandle> = restored.entities.iter_alive().collect();
        assert_eq!(alive.len(), 1);
        let restored_flags = restored.entities.get_flags(alive[0]).unwrap();
        assert_eq!(restored_flags, flags);
        assert!(restored_flags.contains(StatusFlags::POWERED));
        assert!(restored_flags.contains(StatusFlags::HAS_WATER));
        assert!(restored_flags.contains(StatusFlags::STAFFED));
        assert!(restored_flags.contains(StatusFlags::DAMAGED));
        assert!(!restored_flags.contains(StatusFlags::ON_FIRE));
        assert!(!restored_flags.contains(StatusFlags::UNDER_CONSTRUCTION));
    }

    // ── Test 10: Treasury and tick preserved ─────────────────────────────

    #[test]
    fn treasury_and_tick_preserved() {
        let mut world = make_test_world(4, 4, 1);
        world.tick = u64::MAX;
        world.treasury = i64::MIN;

        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");

        assert_eq!(restored.tick, u64::MAX);
        assert_eq!(restored.treasury, i64::MIN);
    }

    // ── Test 11: Tile data round-trip with mixed terrain ─────────────────

    #[test]
    fn tile_data_round_trip() {
        use crate::core::tilemap::{TileFlags, TileKind, TileValue};

        let mut world = make_test_world(4, 4, 1);
        world.tiles.set(
            0,
            0,
            TileValue {
                terrain: TerrainType::Sand,
                kind: TileKind::Zone,
                zone: ZoneType::Commercial,
                density: ZoneDensity::Low,
                flags: TileFlags::NONE,
            },
        );
        world.tiles.set(
            1,
            1,
            TileValue {
                terrain: TerrainType::Water,
                kind: TileKind::Empty,
                zone: ZoneType::None,
                density: ZoneDensity::Low,
                flags: TileFlags::NONE,
            },
        );
        world.tiles.set(
            3,
            3,
            TileValue {
                terrain: TerrainType::Rock,
                kind: TileKind::Zone,
                zone: ZoneType::Industrial,
                density: ZoneDensity::Low,
                flags: TileFlags::NONE,
            },
        );

        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");

        let t00 = restored.tiles.get(0, 0).unwrap();
        assert_eq!(t00.terrain, TerrainType::Sand);
        assert_eq!(t00.zone, ZoneType::Commercial);
        assert_eq!(t00.kind, TileKind::Zone);

        let t11 = restored.tiles.get(1, 1).unwrap();
        assert_eq!(t11.terrain, TerrainType::Water);
        assert_eq!(t11.zone, ZoneType::None);
        assert_eq!(t11.kind, TileKind::Empty);

        let t33 = restored.tiles.get(3, 3).unwrap();
        assert_eq!(t33.terrain, TerrainType::Rock);
        assert_eq!(t33.zone, ZoneType::Industrial);
        assert_eq!(t33.kind, TileKind::Zone);
    }

    // ── Test 12: Header fields match world state ─────────────────────────

    #[test]
    fn header_fields_correct() {
        let mut world = make_test_world(16, 8, 77);
        world.city_name = "TestCity".to_string();
        world.tick = 1000;
        world.treasury = 99_999;
        world.place_entity(1, 0, 0, 0);
        world.place_entity(2, 1, 1, 0);

        let data = serialize_world(&world);

        // Manually parse header
        let magic = &data[0..4];
        assert_eq!(magic, b"TOWN");

        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(version, SAVE_VERSION);

        let map_w = u16::from_le_bytes([data[8], data[9]]);
        let map_h = u16::from_le_bytes([data[10], data[11]]);
        assert_eq!(map_w, 16);
        assert_eq!(map_h, 8);

        let entity_count = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        assert_eq!(entity_count, 2);

        let tick = u64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);
        assert_eq!(tick, 1000);

        let treasury = i64::from_le_bytes([
            data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
        ]);
        assert_eq!(treasury, 99_999);

        let name_len = u16::from_le_bytes([data[40], data[41]]);
        assert_eq!(name_len, 8); // "TestCity"
    }

    // ── Test 13: Truncated tile data returns InsufficientData ────────────

    #[test]
    fn truncated_tile_data_returns_error() {
        let world = make_test_world(4, 4, 1);
        let data = serialize_world(&world);
        // Truncate in the middle of tile data
        let header_and_name = HEADER_SIZE + world.city_name.len() + 8; // +8 for policies
        let truncated = &data[..header_and_name + 5]; // only 5 bytes of tile data (need 80)
        let err = deserialize_world(truncated).unwrap_err();
        assert_eq!(err, SaveError::InsufficientData);
    }

    // ── Test 14: Truncated entity data returns InsufficientData ──────────

    #[test]
    fn truncated_entity_data_returns_error() {
        let mut world = make_test_world(2, 2, 1);
        world.place_entity(1, 0, 0, 0);
        let data = serialize_world(&world);
        // Truncate so entity data is incomplete (remove last 3 bytes)
        let truncated = &data[..data.len() - 3];
        let err = deserialize_world(truncated).unwrap_err();
        assert_eq!(err, SaveError::InsufficientData);
    }

    // ── Test 15: Seed value preserved ────────────────────────────────────

    #[test]
    fn seed_value_preserved() {
        let world = make_test_world(4, 4, 0xDEAD_BEEF_CAFE_BABE);
        let data = serialize_world(&world);
        let restored = deserialize_world(&data).expect("deserialization failed");
        assert_eq!(restored.seeds.global_seed, 0xDEAD_BEEF_CAFE_BABE);
    }

    // ── Test 16: Empty data returns InsufficientData ─────────────────────

    #[test]
    fn empty_data_returns_error() {
        let err = deserialize_world(&[]).unwrap_err();
        assert_eq!(err, SaveError::InsufficientData);
    }
}
