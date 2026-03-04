//! WASM panic/trap recovery boundary.
//!
//! Provides error types, a panic hook for better error reporting,
//! `with_recovery` for catching panics, and an `IErrorBoundary` trait
//! with a default implementation that tracks errors and decides whether
//! to restart.

use std::panic;

// ---------------------------------------------------------------------------
// WasmError
// ---------------------------------------------------------------------------

/// Error types that can be reported to JavaScript.
#[derive(Debug, Clone)]
pub enum WasmError {
    Panic(String),
    OutOfMemory,
    InvalidState(String),
    Unknown,
}

impl WasmError {
    /// Numeric error code for the JS layer.
    pub fn code(&self) -> u32 {
        match self {
            WasmError::Panic(_) => 1,
            WasmError::OutOfMemory => 2,
            WasmError::InvalidState(_) => 3,
            WasmError::Unknown => 99,
        }
    }

    /// Human-readable error message.
    pub fn message(&self) -> String {
        match self {
            WasmError::Panic(msg) => format!("WASM panic: {}", msg),
            WasmError::OutOfMemory => "Out of WASM memory".to_string(),
            WasmError::InvalidState(msg) => format!("Invalid state: {}", msg),
            WasmError::Unknown => "Unknown WASM error".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Panic hook
// ---------------------------------------------------------------------------

/// Install a panic hook for better error reporting.
///
/// Replaces the default panic hook with one that extracts a string message
/// from the panic payload. In WASM builds this would log to the browser
/// console; in tests it simply captures the message.
pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        // In WASM, this would log to console via web_sys::console::error_1
        #[cfg(not(test))]
        {
            // web_sys::console::error_1(&message.into());
        }
        let _ = message; // suppress unused warning in test
    }));
}

// ---------------------------------------------------------------------------
// with_recovery
// ---------------------------------------------------------------------------

/// Run a closure with panic recovery.
///
/// Returns `Ok(result)` if the closure completes normally, or
/// `Err(WasmError::Panic(msg))` if it panics.
pub fn with_recovery<F, T>(f: F) -> Result<T, WasmError>
where
    F: FnOnce() -> T + panic::UnwindSafe,
{
    match panic::catch_unwind(f) {
        Ok(result) => Ok(result),
        Err(payload) => {
            let message = if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic payload".to_string()
            };
            Err(WasmError::Panic(message))
        }
    }
}

// ---------------------------------------------------------------------------
// IErrorBoundary trait
// ---------------------------------------------------------------------------

/// Trait for error boundary behaviour.
pub trait IErrorBoundary {
    /// Record an error that occurred.
    fn on_error(&mut self, error: &WasmError);

    /// Whether the engine should attempt a restart after the last error.
    fn should_restart(&self) -> bool;

    /// Total number of errors recorded.
    fn get_error_count(&self) -> u32;

    /// Reset the error state (e.g. after a successful restart).
    fn reset(&mut self);
}

// ---------------------------------------------------------------------------
// DefaultErrorBoundary
// ---------------------------------------------------------------------------

/// Default error boundary with a restart-after-N-errors policy.
///
/// Tracks the number of errors seen and permits restarts as long as
/// the count stays below `max_errors`.
pub struct DefaultErrorBoundary {
    error_count: u32,
    max_errors: u32,
    last_error: Option<WasmError>,
}

impl DefaultErrorBoundary {
    /// Create a new boundary that allows up to `max_errors` restarts.
    pub fn new(max_errors: u32) -> Self {
        Self {
            error_count: 0,
            max_errors,
            last_error: None,
        }
    }

    /// The most recent error, if any.
    pub fn last_error(&self) -> Option<&WasmError> {
        self.last_error.as_ref()
    }
}

impl IErrorBoundary for DefaultErrorBoundary {
    fn on_error(&mut self, error: &WasmError) {
        self.error_count += 1;
        self.last_error = Some(error.clone());
    }

    fn should_restart(&self) -> bool {
        self.error_count < self.max_errors
    }

    fn get_error_count(&self) -> u32 {
        self.error_count
    }

    fn reset(&mut self) {
        self.error_count = 0;
        self.last_error = None;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: with_recovery returns Ok on success ─────────────────────

    #[test]
    fn with_recovery_ok_on_success() {
        let result = with_recovery(|| 42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    // ── Test 2: with_recovery catches panic ─────────────────────────────

    #[test]
    fn with_recovery_catches_panic() {
        let result = with_recovery(|| {
            panic!("test panic");
        });
        assert!(result.is_err());
        match result.unwrap_err() {
            WasmError::Panic(msg) => assert!(msg.contains("test panic")),
            other => panic!("Expected Panic variant, got {:?}", other),
        }
    }

    // ── Test 3: WasmError codes are correct ─────────────────────────────

    #[test]
    fn wasm_error_codes_correct() {
        assert_eq!(WasmError::Panic("x".into()).code(), 1);
        assert_eq!(WasmError::OutOfMemory.code(), 2);
        assert_eq!(WasmError::InvalidState("x".into()).code(), 3);
        assert_eq!(WasmError::Unknown.code(), 99);
    }

    // ── Test 4: WasmError messages are correct ──────────────────────────

    #[test]
    fn wasm_error_messages_correct() {
        assert_eq!(
            WasmError::Panic("boom".into()).message(),
            "WASM panic: boom"
        );
        assert_eq!(
            WasmError::OutOfMemory.message(),
            "Out of WASM memory"
        );
        assert_eq!(
            WasmError::InvalidState("bad".into()).message(),
            "Invalid state: bad"
        );
        assert_eq!(
            WasmError::Unknown.message(),
            "Unknown WASM error"
        );
    }

    // ── Test 5: DefaultErrorBoundary counts errors ──────────────────────

    #[test]
    fn default_boundary_counts_errors() {
        let mut boundary = DefaultErrorBoundary::new(3);
        assert_eq!(boundary.get_error_count(), 0);

        boundary.on_error(&WasmError::Panic("a".into()));
        assert_eq!(boundary.get_error_count(), 1);

        boundary.on_error(&WasmError::OutOfMemory);
        assert_eq!(boundary.get_error_count(), 2);
    }

    // ── Test 6: should_restart true under max, false at max ─────────────

    #[test]
    fn should_restart_under_max_and_at_max() {
        let mut boundary = DefaultErrorBoundary::new(2);

        // Before any errors: can restart.
        assert!(boundary.should_restart());

        // First error: count=1, max=2 -> can restart.
        boundary.on_error(&WasmError::Unknown);
        assert!(boundary.should_restart());

        // Second error: count=2, max=2 -> cannot restart.
        boundary.on_error(&WasmError::Unknown);
        assert!(!boundary.should_restart());

        // Third error: still cannot restart.
        boundary.on_error(&WasmError::Unknown);
        assert!(!boundary.should_restart());
    }

    // ── Test 7: reset clears state ──────────────────────────────────────

    #[test]
    fn reset_clears_state() {
        let mut boundary = DefaultErrorBoundary::new(3);

        boundary.on_error(&WasmError::Panic("oops".into()));
        boundary.on_error(&WasmError::OutOfMemory);
        assert_eq!(boundary.get_error_count(), 2);
        assert!(boundary.last_error().is_some());

        boundary.reset();
        assert_eq!(boundary.get_error_count(), 0);
        assert!(boundary.last_error().is_none());
        assert!(boundary.should_restart());
    }

    // ── Test 8: last_error tracks most recent error ─────────────────────

    #[test]
    fn last_error_tracks_most_recent() {
        let mut boundary = DefaultErrorBoundary::new(5);

        assert!(boundary.last_error().is_none());

        boundary.on_error(&WasmError::Panic("first".into()));
        match boundary.last_error() {
            Some(WasmError::Panic(msg)) => assert_eq!(msg, "first"),
            other => panic!("Expected Panic, got {:?}", other),
        }

        boundary.on_error(&WasmError::OutOfMemory);
        match boundary.last_error() {
            Some(WasmError::OutOfMemory) => {}
            other => panic!("Expected OutOfMemory, got {:?}", other),
        }
    }

    // ── Test 9: with_recovery catches String panic payload ──────────────

    #[test]
    fn with_recovery_catches_string_panic() {
        let result = with_recovery(|| {
            panic!("{}", "string panic");
        });
        assert!(result.is_err());
        match result.unwrap_err() {
            WasmError::Panic(msg) => assert!(msg.contains("string panic")),
            other => panic!("Expected Panic variant, got {:?}", other),
        }
    }

    // ── Test 10: new boundary with zero max_errors never restarts ───────

    #[test]
    fn zero_max_never_restarts() {
        let mut boundary = DefaultErrorBoundary::new(0);
        // Even with zero errors, should_restart is false because 0 < 0 is false.
        assert!(!boundary.should_restart());

        boundary.on_error(&WasmError::Unknown);
        assert!(!boundary.should_restart());
    }
}
