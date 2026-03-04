//! WASM API surface for TownBuilder.
//!
//! Provides a `GameHandle` struct that wraps the simulation engine and
//! exposes methods for creating, ticking, commanding, and saving/loading
//! the game. Uses conditional compilation so that `wasm_bindgen` attributes
//! are only applied when targeting `wasm32`, allowing the module to compile
//! and be tested on native targets as well.

pub mod error_boundary;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::core::archetypes::ArchetypeRegistry;
use crate::core::buildings::register_base_city_builder_archetypes;
use crate::core::commands::Command;
use crate::core::network::RoadGraph;
use crate::core::world::WorldState;
use crate::core_types::*;
use crate::io::save;
use crate::sim::tick::SimulationEngine;

// ---------------------------------------------------------------------------
// GameHandle
// ---------------------------------------------------------------------------

/// Opaque handle to a running game simulation.
///
/// Wraps an `Option<SimulationEngine>` — `None` only if the engine has been
/// moved out (e.g. during `save`/`load` round-trips).
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GameHandle {
    engine: Option<SimulationEngine>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GameHandle {
    // -- Construction -------------------------------------------------------

    /// Create a new game with the given seed and map dimensions.
    ///
    /// Initialises a `WorldState`, empty `ArchetypeRegistry`, empty
    /// `RoadGraph`, and wires them into a `SimulationEngine`.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(seed: u64, map_width: u16, map_height: u16) -> GameHandle {
        let world = WorldState::new(MapSize::new(map_width, map_height), seed);
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        let road_graph = RoadGraph::new();
        let engine = SimulationEngine::new(world, registry, road_graph);

        GameHandle {
            engine: Some(engine),
        }
    }

    // -- Tick ---------------------------------------------------------------

    /// Advance the simulation by one tick.
    ///
    /// Returns a JSON string describing the `TickOutput` (tick number,
    /// population, treasury, and event count).
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub fn tick(&mut self) -> String {
        let engine = match self.engine.as_mut() {
            Some(e) => e,
            None => return r#"{"error":"no engine"}"#.to_string(),
        };

        let output = engine.tick();

        // Build a JSON object manually — avoids requiring TickOutput to
        // derive Serialize and keeps the dependency surface small.
        format!(
            r#"{{"tick":{},"population":{},"treasury":{},"event_count":{}}}"#,
            output.tick,
            output.population,
            output.treasury,
            output.events.len(),
        )
    }

    // -- Commands -----------------------------------------------------------

    /// Apply a command described by a JSON string.
    ///
    /// The JSON must deserialize to a `Command` enum variant. Returns a JSON
    /// string with the result: `{"ok": ...}` on success or
    /// `{"error": "..."}` on failure.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub fn apply_command_json(&mut self, json: &str) -> String {
        let engine = match self.engine.as_mut() {
            Some(e) => e,
            None => return r#"{"error":"no engine"}"#.to_string(),
        };

        let cmd: Command = match serde_json::from_str(json) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"parse: {}"}}"#, e),
        };

        match engine.apply_command(&cmd) {
            Ok(effect) => format!(r#"{{"ok":"{}"}}"#, format!("{:?}", effect)),
            Err(err) => format!(r#"{{"error":"{}"}}"#, format!("{:?}", err)),
        }
    }

    // -- Getters ------------------------------------------------------------

    /// Return the current simulation tick.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub fn get_tick(&self) -> u64 {
        match &self.engine {
            Some(e) => e.world.tick,
            None => 0,
        }
    }

    /// Return the current treasury balance (in cents).
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub fn get_treasury(&self) -> i64 {
        match &self.engine {
            Some(e) => e.world.treasury,
            None => 0,
        }
    }

    /// Return the current city population.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub fn get_population(&self) -> u32 {
        match &self.engine {
            Some(e) => e.population,
            None => 0,
        }
    }

    // -- Save / Load --------------------------------------------------------

    /// Serialize the current world state to a byte vector.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub fn save(&self) -> Vec<u8> {
        match &self.engine {
            Some(e) => save::serialize_world(&e.world),
            None => Vec::new(),
        }
    }

    /// Deserialize a world state from bytes and create a new `GameHandle`.
    ///
    /// Returns `Err(String)` if deserialization fails.
    pub fn load(data: &[u8]) -> Result<GameHandle, String> {
        let world = save::deserialize_world(data)?;
        let mut registry = ArchetypeRegistry::new();
        register_base_city_builder_archetypes(&mut registry);
        let road_graph = RoadGraph::new();
        let engine = SimulationEngine::new(world, registry, road_graph);

        Ok(GameHandle {
            engine: Some(engine),
        })
    }
}

// ---------------------------------------------------------------------------
// Standalone WASM helper: load from bytes (since wasm_bindgen does not
// support Result<T, String> on methods easily, provide a free function).
// ---------------------------------------------------------------------------

/// Load a saved game from a byte slice. Returns a `GameHandle` or throws
/// a JS error on the WASM target.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn load_game(data: &[u8]) -> Result<GameHandle, JsValue> {
    GameHandle::load(data).map_err(|e| JsValue::from_str(&e))
}

// ---------------------------------------------------------------------------
// Tests (native only)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::buildings::ARCH_UTIL_POWER_PLANT;

    // ── Test 1: GameHandle::new creates valid handle ────────────────────

    #[test]
    fn new_creates_valid_handle() {
        let handle = GameHandle::new(42, 32, 32);
        assert!(handle.engine.is_some());
        assert_eq!(handle.get_tick(), 0);
        assert_eq!(handle.get_treasury(), 500_000);
        assert_eq!(handle.get_population(), 0);
    }

    // ── Test 2: tick() returns valid JSON output ────────────────────────

    #[test]
    fn tick_returns_valid_json() {
        let mut handle = GameHandle::new(42, 32, 32);
        let json = handle.tick();
        assert!(json.contains("\"tick\":1"));
        assert!(json.contains("\"population\":"));
        assert!(json.contains("\"treasury\":"));
        assert!(json.contains("\"event_count\":"));
    }

    // ── Test 3: get_tick / get_treasury / get_population ────────────────

    #[test]
    fn getters_work_after_ticks() {
        let mut handle = GameHandle::new(42, 32, 32);
        assert_eq!(handle.get_tick(), 0);

        handle.tick();
        assert_eq!(handle.get_tick(), 1);

        handle.tick();
        assert_eq!(handle.get_tick(), 2);

        // Treasury should still be reasonable (started at 500_000, no buildings).
        assert!(handle.get_treasury() >= 0);

        // Population starts at 0, may change slightly due to rng.
        let _pop = handle.get_population();
    }

    // ── Test 4: apply_command_json with valid PlaceEntity ───────────────

    #[test]
    fn apply_command_json_valid() {
        let mut handle = GameHandle::new(42, 32, 32);
        let cmd_json = format!(
            r#"{{"PlaceEntity":{{"archetype_id":{},"x":5,"y":5,"rotation":0}}}}"#,
            ARCH_UTIL_POWER_PLANT
        );
        let result = handle.apply_command_json(&cmd_json);
        assert!(result.contains("ok"), "Expected ok, got: {}", result);
        assert!(result.contains("EntityPlaced"));
    }

    // ── Test 5: apply_command_json with invalid JSON ────────────────────

    #[test]
    fn apply_command_json_invalid() {
        let mut handle = GameHandle::new(42, 32, 32);
        let result = handle.apply_command_json("not json at all");
        assert!(result.contains("error"), "Expected error, got: {}", result);
        assert!(result.contains("parse"));
    }

    // ── Test 6: apply_command_json out-of-bounds ────────────────────────

    #[test]
    fn apply_command_json_out_of_bounds() {
        let mut handle = GameHandle::new(42, 32, 32);
        let cmd_json = format!(
            r#"{{"PlaceEntity":{{"archetype_id":{},"x":100,"y":100,"rotation":0}}}}"#,
            ARCH_UTIL_POWER_PLANT
        );
        let result = handle.apply_command_json(&cmd_json);
        assert!(result.contains("error"), "Expected error, got: {}", result);
        assert!(result.contains("OutOfBounds"));
    }

    // ── Test 7: save returns non-empty bytes ────────────────────────────

    #[test]
    fn save_returns_non_empty_bytes() {
        let handle = GameHandle::new(42, 32, 32);
        let data = handle.save();
        assert!(!data.is_empty(), "save() should return non-empty bytes");
        assert!(data.len() > 100, "save data should have meaningful size");
    }

    // ── Test 8: round-trip save/load ────────────────────────────────────

    #[test]
    fn round_trip_save_load() {
        let mut handle = GameHandle::new(42, 32, 32);

        // Run a few ticks to change state.
        for _ in 0..10 {
            handle.tick();
        }
        let tick_before = handle.get_tick();
        let treasury_before = handle.get_treasury();

        // Save.
        let data = handle.save();
        assert!(!data.is_empty());

        // Load.
        let loaded = GameHandle::load(&data).unwrap();
        assert_eq!(loaded.get_tick(), tick_before);
        assert_eq!(loaded.get_treasury(), treasury_before);
    }

    // ── Test 9: load with garbage data fails ────────────────────────────

    #[test]
    fn load_garbage_fails() {
        let result = GameHandle::load(b"not a save file");
        assert!(result.is_err());
    }

    // ── Test 10: multiple ticks advance state correctly ─────────────────

    #[test]
    fn multiple_ticks_advance() {
        let mut handle = GameHandle::new(123, 64, 64);
        for i in 1..=50 {
            let json = handle.tick();
            assert!(json.contains(&format!("\"tick\":{}", i)));
        }
        assert_eq!(handle.get_tick(), 50);
    }

    // ── Test 11: no engine gracefully handles calls ─────────────────────

    #[test]
    fn no_engine_returns_defaults() {
        let mut handle = GameHandle {
            engine: None,
        };
        assert_eq!(handle.get_tick(), 0);
        assert_eq!(handle.get_treasury(), 0);
        assert_eq!(handle.get_population(), 0);
        let tick_json = handle.tick();
        assert!(tick_json.contains("error"));
        let cmd_json = handle.apply_command_json("{}");
        assert!(cmd_json.contains("error"));
        let save_data = handle.save();
        assert!(save_data.is_empty());
    }
}
