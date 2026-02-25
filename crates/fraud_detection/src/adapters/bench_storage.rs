// Rust guideline compliant 2026-02-16

//! Discard adapter for the `Storage` port -- benchmark use only.
//!
//! # Measurement scope
//!
//! `BenchStorage` counts received transactions and immediately drops each
//! batch without persisting anything.  All throughput figures produced by
//! `fraud_detection_bench` therefore measure **pipeline infrastructure only**
//! (Producer, Consumer, Modelizer, Logger, both buffers) and explicitly
//! **exclude storage write cost**.
//!
//! This is intentional: the goal of the benchmark is to establish a baseline
//! for the pipeline itself.  Adding real I/O (even `InMemoryStorage`) would
//! conflate storage allocation cost with pipeline throughput at large batch
//! sizes (100k × 1 000 iterations ≈ 100 M transactions ≈ 15 GB RAM per
//! round), making results unstable and hard to reproduce.
//!
//! If you need to benchmark a specific storage backend, wire it directly in
//! a dedicated binary and measure it in isolation.

use std::cell::RefCell;

use domain::{PendingTransaction, Storage, StorageError};

/// `Storage` adapter that counts batches and discards them immediately.
///
/// No heap allocation beyond the counter itself.  Intended exclusively for
/// `fraud_detection_bench`; not suitable for production use.
///
/// # Measurement scope
///
/// Throughput numbers obtained with this adapter reflect **pipeline overhead
/// only** -- storage write cost is excluded by design.
#[derive(Debug)]
pub struct BenchStorage {
    count: RefCell<usize>,
}

impl BenchStorage {
    /// Create a new discard storage with a zero transaction count.
    #[must_use]
    pub fn new() -> Self {
        Self { count: RefCell::new(0) }
    }

    /// Return the cumulative number of transactions received so far.
    #[must_use]
    pub fn count(&self) -> usize {
        *self.count.borrow()
    }
}

impl Default for BenchStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for BenchStorage {
    /// Increment the counter by `batch.len()` and drop the batch.
    ///
    /// # Errors
    ///
    /// Infallible; always returns `Ok(())`.
    async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError> {
        *self.count.borrow_mut() += batch.len();
        // Batch dropped here -- no persistence, no allocation.
        Ok(())
    }
}
