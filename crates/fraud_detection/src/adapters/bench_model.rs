// Rust guideline compliant 2026-02-16

//! Deterministic zero-overhead adapter for the `Model` port.
//!
//! Illustrates DIP in the hexagonal architecture: Consumer depends only on
//! the `Model` port; swapping `DemoModel` for `BenchModel` requires zero
//! changes to Consumer, Modelizer, or domain code.
//!
//! Always returns `Ok(false)` (no fraud), eliminating RNG overhead and
//! preventing any `LogAlarm` calls during benchmarks.

use domain::{Model, ModelizerError, ModelVersion, Transaction};

/// `Model` adapter that always classifies transactions as non-fraudulent.
///
/// No RNG, no I/O, no state mutation -- minimal overhead for throughput
/// measurement.
#[derive(Debug)]
pub struct BenchModel;

impl BenchModel {
    /// Create a new bench model adapter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for BenchModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Model for BenchModel {
    /// Always returns `Ok(false)` (no fraud).
    ///
    /// # Errors
    ///
    /// Infallible; always returns `Ok(false)`.
    async fn classify(&self, _tx: &Transaction) -> Result<bool, ModelizerError> {
        Ok(false)
    }

    /// Returns `"BENCH"`.
    fn name(&self) -> &'static str {
        "BENCH"
    }

    /// Returns `"1"`.
    fn active_version(&self) -> &'static str {
        "1"
    }

    /// No-op version switch.
    ///
    /// # Errors
    ///
    /// Infallible; always returns `Ok(())`.
    async fn switch_version(&self, _version: ModelVersion) -> Result<(), ModelizerError> {
        Ok(())
    }
}
