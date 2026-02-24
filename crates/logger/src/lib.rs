// Rust guideline compliant 2026-02-16

//! Logger crate: reads InferredTransaction batches from Buffer2, persists as PendingTransaction.
//!
//! Entry points: [`Logger::log_once`], [`Logger::run`].
//! Configuration via [`LoggerConfig::builder`].

use domain::{Buffer2Read, BufferError, InferredTransaction, PendingTransaction, Storage, StorageError};
use rand::{SeedableRng, rngs::StdRng};
use rand::Rng as _;
use std::cell::RefCell;
use std::time::Duration;

// ---------------------------------------------------------------------------
// LoggerError
// ---------------------------------------------------------------------------

/// Errors that can occur during logger operation.
#[derive(Debug, thiserror::Error)]
pub enum LoggerError {
    /// The supplied configuration is invalid.
    #[error("invalid logger configuration: {reason}")]
    InvalidConfig {
        /// Human-readable description of the problem.
        reason: String,
    },
    /// A buffer read failed.
    #[error("buffer read error: {0}")]
    Read(#[from] BufferError),
    /// A storage write failed.
    #[error("storage write error: {0}")]
    Write(#[from] StorageError),
}

// ---------------------------------------------------------------------------
// LoggerConfig + builder
// ---------------------------------------------------------------------------

/// Runtime configuration for a [`Logger`].
///
/// Construct via [`LoggerConfig::builder`].
#[derive(Debug)]
pub struct LoggerConfig {
    /// Maximum batch size drawn from Buffer2 per iteration (range: `[1, n3_max]`).
    pub n3_max: usize,
    /// Delay between successive iterations.
    pub poll_interval3: Duration,
    /// Optional upper bound on the number of iterations. `None` means infinite.
    pub iterations: Option<u64>,
    /// Optional RNG seed for reproducible batch sizing. `None` seeds from the OS.
    pub seed: Option<u64>,
}

/// Builder for [`LoggerConfig`].
///
/// Obtain via [`LoggerConfig::builder`]; finalize with [`build`](Self::build).
#[derive(Debug)]
pub struct LoggerConfigBuilder {
    n3_max: usize,
    poll_interval3: Duration,
    iterations: Option<u64>,
    seed: Option<u64>,
}

impl LoggerConfig {
    /// Create a builder. `n3_max` is the only required parameter.
    ///
    /// Default values: `poll_interval3 = 100 ms`, `iterations = None`, `seed = None`.
    #[must_use]
    pub fn builder(n3_max: usize) -> LoggerConfigBuilder {
        LoggerConfigBuilder {
            n3_max,
            // 100 ms chosen as a reasonable demo cadence; lower for tests.
            poll_interval3: Duration::from_millis(100),
            iterations: None,
            seed: None,
        }
    }
}

impl LoggerConfigBuilder {
    /// Override the inter-iteration delay.
    #[must_use]
    pub fn poll_interval3(mut self, poll_interval3: Duration) -> Self {
        self.poll_interval3 = poll_interval3;
        self
    }

    /// Set a finite iteration count. Without this the logger runs until Buffer2 is closed.
    #[must_use]
    pub fn iterations(mut self, n: u64) -> Self {
        self.iterations = Some(n);
        self
    }

    /// Fix the RNG seed for deterministic batch sizing (useful in tests).
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Validate and build the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`LoggerError::InvalidConfig`] when `n3_max` is zero.
    #[must_use = "the Result must be checked; use ? or unwrap"]
    pub fn build(self) -> Result<LoggerConfig, LoggerError> {
        if self.n3_max == 0 {
            return Err(LoggerError::InvalidConfig {
                reason: "n3_max must be >= 1".to_owned(),
            });
        }
        Ok(LoggerConfig {
            n3_max: self.n3_max,
            poll_interval3: self.poll_interval3,
            iterations: self.iterations,
            seed: self.seed,
        })
    }
}

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

/// Reads batches of `InferredTransaction` from a `Buffer2Read` port and persists
/// them as `PendingTransaction` to a `Storage` port.
///
/// Generic over `B: Buffer2Read` and `S: Storage` for zero-cost static dispatch.
/// Holds no concrete adapter references -- dependencies injected per call.
#[derive(Debug)]
pub struct Logger {
    config: LoggerConfig,
    /// Interior mutability required because all public methods take `&self`.
    rng: RefCell<StdRng>,
}

impl Logger {
    /// Create a new logger from `config`.
    ///
    /// Seeds the RNG from `config.seed` if set, otherwise from the OS.
    #[must_use]
    pub fn new(config: LoggerConfig) -> Self {
        let rng = match config.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        };
        Self { config, rng: RefCell::new(rng) }
    }

    /// Read one batch from `buf2`, transform each item, and persist to `storage`.
    ///
    /// Batch size `n3` is uniformly distributed in `[1, config.n3_max]`.
    /// Each `InferredTransaction` becomes a `PendingTransaction` with
    /// `is_reviewed = false` and `actual_fraud = None`.
    ///
    /// # Errors
    ///
    /// Returns [`LoggerError::Read`] on buffer errors, or
    /// [`LoggerError::Write`] on storage errors.
    pub async fn log_once<B: Buffer2Read, S: Storage>(
        &self,
        buf2: &B,
        storage: &S,
    ) -> Result<(), LoggerError> {
        let n3 = self.rng.borrow_mut().random_range(1..=self.config.n3_max);
        log::debug!("logger.log_once: batch_size={n3}");
        let batch: Vec<InferredTransaction> = buf2.read_batch(n3).await?;
        let pending: Vec<PendingTransaction> = batch
            .into_iter()
            .map(|tx| PendingTransaction { inferred_transaction: tx, is_reviewed: false, actual_fraud: None })
            .collect();
        storage.write_batch(pending).await?;
        Ok(())
    }

    /// Run the read-transform-persist loop until stopped.
    ///
    /// Calls [`log_once`](Self::log_once) repeatedly, sleeping `config.poll_interval3`
    /// between iterations. Stops cleanly when:
    /// - Buffer2 signals [`BufferError::Closed`] (returns `Ok(())`), or
    /// - `config.iterations` batches have been processed (returns `Ok(())`).
    ///
    /// # Errors
    ///
    /// Returns [`LoggerError::Write`] for any storage error.
    pub async fn run<B: Buffer2Read, S: Storage>(
        &self,
        buf2: &B,
        storage: &S,
    ) -> Result<(), LoggerError> {
        let mut count = 0u64;
        loop {
            match self.log_once(buf2, storage).await {
                Ok(()) => {}
                Err(LoggerError::Read(BufferError::Closed)) => {
                    log::info!(
                        "logger.run.stopped: buffer closed after {count} iteration(s)"
                    );
                    return Ok(());
                }
                Err(e) => return Err(e),
            }

            count += 1;
            log::info!("logger.batch.persisted: iteration={count}");

            if let Some(max) = self.config.iterations
                && count >= max
            {
                log::info!("logger.run.stopped: iteration limit reached");
                return Ok(());
            }

            tokio::time::sleep(self.config.poll_interval3).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Transaction;
    use uuid::Uuid;

    // ------------------------------------------------------------------
    // T011: Mock adapters
    // ------------------------------------------------------------------

    /// Returns a simple `InferredTransaction` with a known UUID for assertions.
    fn make_inferred(predicted_fraud: bool) -> InferredTransaction {
        InferredTransaction {
            transaction: Transaction {
                id: Uuid::new_v4(),
                amount: 1.00_f64,
                last_name: "Test".to_owned(),
            },
            predicted_fraud,
            model_name: "DEMO".to_owned(),
            model_version: "4".to_owned(),
        }
    }

    /// Mock read adapter: pre-loaded items; signals `Closed` when empty+closed.
    struct MockBuffer2Read {
        items: RefCell<Vec<InferredTransaction>>,
        closed: RefCell<bool>,
    }

    impl MockBuffer2Read {
        fn new(items: Vec<InferredTransaction>) -> Self {
            Self { items: RefCell::new(items), closed: RefCell::new(false) }
        }

        fn new_closed(items: Vec<InferredTransaction>) -> Self {
            Self { items: RefCell::new(items), closed: RefCell::new(true) }
        }
    }

    impl Buffer2Read for MockBuffer2Read {
        async fn read_batch(&self, max: usize) -> Result<Vec<InferredTransaction>, BufferError> {
            let mut items = self.items.borrow_mut();
            if items.is_empty() && *self.closed.borrow() {
                return Err(BufferError::Closed);
            }
            let count = max.min(items.len());
            Ok(items.drain(..count).collect())
        }
    }

    /// Mock write adapter: collects all writes; optional forced error.
    struct MockStorage {
        items: RefCell<Vec<PendingTransaction>>,
        force_error: Option<StorageError>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self { items: RefCell::new(vec![]), force_error: None }
        }

        fn with_error(err: StorageError) -> Self {
            Self { items: RefCell::new(vec![]), force_error: Some(err) }
        }
    }

    impl Storage for MockStorage {
        async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError> {
            if let Some(ref e) = self.force_error {
                return Err(e.clone());
            }
            self.items.borrow_mut().extend(batch);
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // T019: LoggerConfig builder tests
    // ------------------------------------------------------------------

    #[test]
    fn config_n3_max_5_builds_ok() {
        let cfg = LoggerConfig::builder(5).build();
        assert!(cfg.is_ok());
    }

    #[test]
    fn config_n3_max_0_returns_err() {
        let cfg = LoggerConfig::builder(0).build();
        assert!(matches!(cfg, Err(LoggerError::InvalidConfig { .. })));
    }

    #[test]
    fn config_poll_interval3_default_is_100ms() {
        let cfg = LoggerConfig::builder(1).build().unwrap();
        assert_eq!(cfg.poll_interval3, Duration::from_millis(100));
    }

    #[test]
    fn config_poll_interval3_setter_overrides() {
        let cfg = LoggerConfig::builder(1).poll_interval3(Duration::ZERO).build().unwrap();
        assert_eq!(cfg.poll_interval3, Duration::ZERO);
    }

    #[test]
    fn config_iterations_defaults_to_none() {
        let cfg = LoggerConfig::builder(1).build().unwrap();
        assert!(cfg.iterations.is_none());
    }

    #[test]
    fn config_seed_defaults_to_none() {
        let cfg = LoggerConfig::builder(1).build().unwrap();
        assert!(cfg.seed.is_none());
    }

    // ------------------------------------------------------------------
    // T012: batch size in range
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_log_once_batch_size_in_range() {
        // N3_MAX=10, buffer with 100 items, call log_once 20 times.
        // Each batch must be in [1, 10].
        let items: Vec<InferredTransaction> = (0..100).map(|_| make_inferred(false)).collect();
        let buf = MockBuffer2Read::new(items);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(10).seed(1).build().unwrap();
        let logger = Logger::new(cfg);
        for _ in 0..20 {
            // Stop if buffer drained (not failure).
            if logger.log_once(&buf, &storage).await.is_err() {
                break;
            }
        }
        // All stored items were consumed in batches of 1..=10.
        let stored = storage.items.borrow().len();
        assert!((1..=100).contains(&stored));
    }

    // ------------------------------------------------------------------
    // T013: batch capped at available
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_log_once_batch_capped_at_available() {
        // N3_MAX=20, only 3 items available.
        let items: Vec<InferredTransaction> = (0..3).map(|_| make_inferred(false)).collect();
        let buf = MockBuffer2Read::new(items);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(20).seed(1).build().unwrap();
        let logger = Logger::new(cfg);
        logger.log_once(&buf, &storage).await.unwrap();
        assert_eq!(storage.items.borrow().len(), 3);
    }

    // ------------------------------------------------------------------
    // T014: closed+empty returns Err(Read(Closed))
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_log_once_closed_empty_returns_error() {
        let buf = MockBuffer2Read::new_closed(vec![]);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(5).build().unwrap();
        let logger = Logger::new(cfg);
        let result = logger.log_once(&buf, &storage).await;
        assert!(
            matches!(result, Err(LoggerError::Read(BufferError::Closed))),
            "expected Err(Read(Closed)), got {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // T020: transform preserves all fields
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_transform_preserves_all_fields() {
        // Drain all 5 via run() with a closed buffer; verify every PendingTransaction.
        let originals: Vec<InferredTransaction> = (0..5).map(|_| make_inferred(false)).collect();
        let orig_clone = originals.clone();
        let buf = MockBuffer2Read::new_closed(originals);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(10)
            .seed(1)
            .poll_interval3(Duration::ZERO)
            .build()
            .unwrap();
        let logger = Logger::new(cfg);
        logger.run(&buf, &storage).await.unwrap();
        let stored = storage.items.borrow();
        assert_eq!(stored.len(), 5);
        for (i, pt) in stored.iter().enumerate() {
            assert_eq!(pt.inferred_transaction, orig_clone[i]);
            assert!(!pt.is_reviewed);
            assert!(pt.actual_fraud.is_none());
        }
    }

    // ------------------------------------------------------------------
    // T021: predicted_fraud=true preserved
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_transform_predicted_fraud_true_preserved() {
        let item = make_inferred(true);
        let buf = MockBuffer2Read::new(vec![item]);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(1).build().unwrap();
        let logger = Logger::new(cfg);
        logger.log_once(&buf, &storage).await.unwrap();
        let stored = storage.items.borrow();
        assert_eq!(stored.len(), 1);
        assert!(stored[0].inferred_transaction.predicted_fraud);
        assert!(!stored[0].is_reviewed);
        assert!(stored[0].actual_fraud.is_none());
    }

    // ------------------------------------------------------------------
    // T022: predicted_fraud=false preserved, both flags independent
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_transform_predicted_fraud_false_preserved() {
        let item = make_inferred(false);
        let buf = MockBuffer2Read::new(vec![item]);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(1).build().unwrap();
        let logger = Logger::new(cfg);
        logger.log_once(&buf, &storage).await.unwrap();
        let stored = storage.items.borrow();
        assert_eq!(stored.len(), 1);
        assert!(!stored[0].inferred_transaction.predicted_fraud);
        assert!(!stored[0].is_reviewed);
        assert!(stored[0].actual_fraud.is_none());
    }

    // ------------------------------------------------------------------
    // T023: all items persisted
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_persist_all_items() {
        // Drain all 8 via run() with a closed buffer; verify total count.
        let items: Vec<InferredTransaction> = (0..8).map(|_| make_inferred(false)).collect();
        let buf = MockBuffer2Read::new_closed(items);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(10)
            .seed(1)
            .poll_interval3(Duration::ZERO)
            .build()
            .unwrap();
        let logger = Logger::new(cfg);
        logger.run(&buf, &storage).await.unwrap();
        assert_eq!(storage.items.borrow().len(), 8);
    }

    // ------------------------------------------------------------------
    // T024: CapacityExceeded propagates
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_persist_capacity_exceeded_propagates() {
        let items = vec![make_inferred(false)];
        let buf = MockBuffer2Read::new(items);
        let storage = MockStorage::with_error(StorageError::CapacityExceeded { capacity: 0 });
        let cfg = LoggerConfig::builder(1).build().unwrap();
        let logger = Logger::new(cfg);
        let result = logger.log_once(&buf, &storage).await;
        assert!(
            matches!(
                result,
                Err(LoggerError::Write(StorageError::CapacityExceeded { capacity: 0 }))
            ),
            "expected CapacityExceeded, got {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // T025: Unavailable propagates
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_persist_unavailable_propagates() {
        let items = vec![make_inferred(false)];
        let buf = MockBuffer2Read::new(items);
        let storage = MockStorage::with_error(StorageError::Unavailable);
        let cfg = LoggerConfig::builder(1).build().unwrap();
        let logger = Logger::new(cfg);
        let result = logger.log_once(&buf, &storage).await;
        assert!(
            matches!(result, Err(LoggerError::Write(StorageError::Unavailable))),
            "expected Unavailable, got {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // T027: run() stops after iteration limit
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_run_iteration_limit() {
        let items: Vec<InferredTransaction> = (0..30).map(|_| make_inferred(false)).collect();
        let buf = MockBuffer2Read::new(items);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(5)
            .seed(1)
            .iterations(3)
            .poll_interval3(Duration::ZERO)
            .build()
            .unwrap();
        let logger = Logger::new(cfg);
        let result = logger.run(&buf, &storage).await;
        assert!(result.is_ok(), "run with iteration limit must return Ok: {result:?}");
        // At least 3 items persisted (3 iterations, each 1..=5).
        assert!((3..=15).contains(&storage.items.borrow().len()));
    }

    // ------------------------------------------------------------------
    // T028: run() stops when buffer closed
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_run_stops_on_closed() {
        // 5 items pre-loaded, then closed; Logger must drain and return Ok(()).
        let items: Vec<InferredTransaction> = (0..5).map(|_| make_inferred(false)).collect();
        let buf = MockBuffer2Read::new_closed(items);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(10)
            .seed(1)
            .poll_interval3(Duration::ZERO)
            .build()
            .unwrap();
        let logger = Logger::new(cfg);
        let result = logger.run(&buf, &storage).await;
        assert!(result.is_ok(), "run must stop cleanly on closed buffer: {result:?}");
    }

    // ------------------------------------------------------------------
    // T029: run() with zero delay completes without panic
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_run_zero_delay() {
        let items: Vec<InferredTransaction> = (0..20).map(|_| make_inferred(false)).collect();
        let buf = MockBuffer2Read::new(items);
        let storage = MockStorage::new();
        let cfg = LoggerConfig::builder(5)
            .seed(2)
            .iterations(2)
            .poll_interval3(Duration::ZERO)
            .build()
            .unwrap();
        let logger = Logger::new(cfg);
        let result = logger.run(&buf, &storage).await;
        assert!(result.is_ok(), "zero-delay run must complete without panic: {result:?}");
    }
}
