//! System execution template — Template Method pattern.
//!
//! Provides a standardized four-phase execution flow for simulation systems:
//! gather -> compute -> apply -> emit. Each system implements the phases
//! independently; the template orchestrates them in order.

use crate::core::events::EventBus;
use crate::core_types::Tick;

/// Result of a system tick execution.
#[derive(Debug, Default)]
pub struct SystemTickResult {
    /// Number of events emitted during this tick.
    pub events_emitted: u32,
    /// Number of entities processed during computation.
    pub entities_processed: u32,
    /// Whether the system was skipped (gather returned false).
    pub skipped: bool,
}

/// Template for standardized system execution flow.
/// Each system follows: gather -> compute -> apply -> emit.
pub trait SystemTemplate {
    /// System identifier for logging and metrics.
    fn name(&self) -> &str;

    /// Phase 1: Gather data needed for computation.
    /// Returns true if the system should proceed.
    fn gather(&self) -> bool {
        true
    }

    /// Phase 2: Compute results. Returns number of entities processed.
    fn compute(&mut self) -> u32;

    /// Phase 3: Apply computed results to world state.
    fn apply(&mut self) -> u32;

    /// Phase 4: Emit events based on results.
    fn emit(&self, events: &mut EventBus, tick: Tick) -> u32;

    /// Execute the full template flow.
    fn execute(&mut self, events: &mut EventBus, tick: Tick) -> SystemTickResult {
        if !self.gather() {
            return SystemTickResult {
                skipped: true,
                ..Default::default()
            };
        }
        let entities_processed = self.compute();
        self.apply();
        let events_emitted = self.emit(events, tick);
        SystemTickResult {
            events_emitted,
            entities_processed,
            skipped: false,
        }
    }
}

/// A no-op system for testing.
pub struct NoOpSystem {
    /// System name.
    pub name: String,
    /// Whether gather returns true (system proceeds) or false (system skips).
    pub should_gather: bool,
    /// Number of entities to report as processed.
    pub entity_count: u32,
    /// Tracks phase execution order for testing.
    phases_executed: Vec<&'static str>,
}

impl NoOpSystem {
    /// Create a new NoOpSystem with default settings (enabled, 0 entities).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            should_gather: true,
            entity_count: 0,
            phases_executed: Vec::new(),
        }
    }

    /// Create a NoOpSystem that processes a specific number of entities.
    pub fn with_entities(name: &str, count: u32) -> Self {
        Self {
            name: name.to_string(),
            should_gather: true,
            entity_count: count,
            phases_executed: Vec::new(),
        }
    }

    /// Create a disabled NoOpSystem (gather returns false).
    pub fn disabled(name: &str) -> Self {
        Self {
            name: name.to_string(),
            should_gather: false,
            entity_count: 0,
            phases_executed: Vec::new(),
        }
    }

    /// Get the phases that were executed, for testing phase ordering.
    pub fn phases_executed(&self) -> &[&'static str] {
        &self.phases_executed
    }
}

impl SystemTemplate for NoOpSystem {
    fn name(&self) -> &str {
        &self.name
    }

    fn gather(&self) -> bool {
        self.should_gather
    }

    fn compute(&mut self) -> u32 {
        self.phases_executed.push("compute");
        self.entity_count
    }

    fn apply(&mut self) -> u32 {
        self.phases_executed.push("apply");
        self.entity_count
    }

    fn emit(&self, _events: &mut EventBus, _tick: Tick) -> u32 {
        // Note: cannot push to phases_executed here since emit takes &self.
        // Phase ordering is verified via compute and apply.
        0
    }
}

/// Metrics collected from running a single system.
#[derive(Debug, Default)]
pub struct SystemMetrics {
    /// Name of the system that was run.
    pub system_name: String,
    /// Result of the system tick execution.
    pub result: SystemTickResult,
}

/// Run multiple systems and collect metrics.
pub fn run_systems(
    systems: &mut [&mut dyn SystemTemplate],
    events: &mut EventBus,
    tick: Tick,
) -> Vec<SystemMetrics> {
    let mut metrics = Vec::with_capacity(systems.len());
    for system in systems.iter_mut() {
        let name = system.name().to_string();
        let result = system.execute(events, tick);
        metrics.push(SystemMetrics {
            system_name: name,
            result,
        });
    }
    metrics
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::EventBus;

    // ─── Test 1: NoOpSystem execute returns correct result ──────────────

    #[test]
    fn noop_system_execute_returns_correct_result() {
        let mut system = NoOpSystem::with_entities("test", 42);
        let mut events = EventBus::new();

        let result = system.execute(&mut events, 0);

        assert_eq!(result.entities_processed, 42);
        assert_eq!(result.events_emitted, 0);
        assert!(!result.skipped);
    }

    // ─── Test 2: Skipped system has skipped=true ────────────────────────

    #[test]
    fn skipped_system_has_skipped_true() {
        let mut system = NoOpSystem::disabled("disabled-sys");
        let mut events = EventBus::new();

        let result = system.execute(&mut events, 0);

        assert!(result.skipped);
        assert_eq!(result.entities_processed, 0);
        assert_eq!(result.events_emitted, 0);
    }

    // ─── Test 3: Template executes phases in order ──────────────────────

    #[test]
    fn template_executes_phases_in_order() {
        let mut system = NoOpSystem::with_entities("ordered", 10);
        let mut events = EventBus::new();

        system.execute(&mut events, 0);

        let phases = system.phases_executed();
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0], "compute");
        assert_eq!(phases[1], "apply");
    }

    // ─── Test 4: run_systems processes all systems ──────────────────────

    #[test]
    fn run_systems_processes_all_systems() {
        let mut sys_a = NoOpSystem::with_entities("alpha", 5);
        let mut sys_b = NoOpSystem::with_entities("beta", 10);
        let mut sys_c = NoOpSystem::with_entities("gamma", 15);
        let mut events = EventBus::new();

        let metrics = run_systems(
            &mut [&mut sys_a, &mut sys_b, &mut sys_c],
            &mut events,
            1,
        );

        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[0].result.entities_processed, 5);
        assert_eq!(metrics[1].result.entities_processed, 10);
        assert_eq!(metrics[2].result.entities_processed, 15);
    }

    // ─── Test 5: run_systems skips disabled systems ─────────────────────

    #[test]
    fn run_systems_skips_disabled_systems() {
        let mut enabled = NoOpSystem::with_entities("enabled", 7);
        let mut disabled = NoOpSystem::disabled("disabled");
        let mut events = EventBus::new();

        let metrics = run_systems(
            &mut [&mut enabled, &mut disabled],
            &mut events,
            1,
        );

        assert_eq!(metrics.len(), 2);
        assert!(!metrics[0].result.skipped);
        assert_eq!(metrics[0].result.entities_processed, 7);
        assert!(metrics[1].result.skipped);
        assert_eq!(metrics[1].result.entities_processed, 0);
    }

    // ─── Test 6: Metrics contain correct system names ───────────────────

    #[test]
    fn metrics_contain_correct_system_names() {
        let mut sys_a = NoOpSystem::new("finance");
        let mut sys_b = NoOpSystem::new("population");
        let mut events = EventBus::new();

        let metrics = run_systems(
            &mut [&mut sys_a, &mut sys_b],
            &mut events,
            0,
        );

        assert_eq!(metrics[0].system_name, "finance");
        assert_eq!(metrics[1].system_name, "population");
    }

    // ─── Test 7: Default SystemTickResult is zeroed ─────────────────────

    #[test]
    fn default_system_tick_result_is_zeroed() {
        let result = SystemTickResult::default();

        assert_eq!(result.events_emitted, 0);
        assert_eq!(result.entities_processed, 0);
        assert!(!result.skipped);
    }

    // ─── Test 8: Disabled system phases not executed ────────────────────

    #[test]
    fn disabled_system_phases_not_executed() {
        let mut system = NoOpSystem::disabled("skip-me");
        let mut events = EventBus::new();

        system.execute(&mut events, 0);

        // When skipped, compute and apply should not run.
        assert!(system.phases_executed().is_empty());
    }

    // ─── Test 9: NoOpSystem new has correct defaults ────────────────────

    #[test]
    fn noop_system_new_has_correct_defaults() {
        let system = NoOpSystem::new("test-sys");

        assert_eq!(system.name, "test-sys");
        assert!(system.should_gather);
        assert_eq!(system.entity_count, 0);
    }

    // ─── Test 10: System name trait method works ────────────────────────

    #[test]
    fn system_name_trait_method_works() {
        let system = NoOpSystem::new("my-system");
        let name: &str = system.name();
        assert_eq!(name, "my-system");
    }

    // ─── Test 11: run_systems with empty slice ──────────────────────────

    #[test]
    fn run_systems_with_empty_slice() {
        let mut events = EventBus::new();
        let metrics = run_systems(&mut [], &mut events, 0);
        assert!(metrics.is_empty());
    }

    // ─── Test 12: Default SystemMetrics is zeroed ───────────────────────

    #[test]
    fn default_system_metrics_is_zeroed() {
        let metrics = SystemMetrics::default();
        assert_eq!(metrics.system_name, "");
        assert_eq!(metrics.result.events_emitted, 0);
        assert_eq!(metrics.result.entities_processed, 0);
        assert!(!metrics.result.skipped);
    }
}
