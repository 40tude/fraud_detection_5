// Rust guideline compliant 2026-02-16

//! Producer component -- generates random transaction batches and writes them
//! to a `Buffer1` hexagonal port.
//!
//! Entry points: [`Producer::generate_batch`], [`Producer::produce_once`],
//! [`Producer::run`]. Configuration via [`ProducerConfig::builder`].

use domain::{Buffer1, BufferError, Transaction};
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use std::cell::RefCell;
use std::time::Duration;

// ---------------------------------------------------------------------------
// ProducerError
// ---------------------------------------------------------------------------

/// Errors that can occur during transaction production.
#[derive(Debug, thiserror::Error)]
pub enum ProducerError {
    /// The supplied configuration is invalid.
    #[error("invalid producer configuration: {reason}")]
    InvalidConfig {
        /// Human-readable description of the problem.
        reason: String,
    },
    /// A buffer write failed.
    #[error("buffer error: {source}")]
    Buffer {
        /// The underlying buffer error.
        #[from]
        source: BufferError,
    },
}

// ---------------------------------------------------------------------------
// ProducerConfig + builder
// ---------------------------------------------------------------------------

/// Runtime configuration for a [`Producer`].
///
/// Construct via [`ProducerConfig::builder`].
#[derive(Debug)]
pub struct ProducerConfig {
    /// Maximum number of transactions per batch (range: `[1, n1_max]`).
    pub n1_max: usize,
    /// Delay between successive batch writes.
    pub poll_interval1: Duration,
    /// Optional upper bound on the number of iterations. `None` means infinite.
    pub iterations: Option<u64>,
    /// Optional RNG seed for reproducible batches. `None` seeds from the OS.
    pub seed: Option<u64>,
}

/// Builder for [`ProducerConfig`].
///
/// Obtain via [`ProducerConfig::builder`]; finalize with [`build`](Self::build).
#[derive(Debug)]
pub struct ProducerConfigBuilder {
    n1_max: usize,
    poll_interval1: Duration,
    iterations: Option<u64>,
    seed: Option<u64>,
}

impl ProducerConfig {
    /// Create a builder. `n1_max` is the only required parameter.
    ///
    /// Default values: `poll_interval1 = 100 ms`, `iterations = None`, `seed = None`.
    #[must_use]
    pub fn builder(n1_max: usize) -> ProducerConfigBuilder {
        ProducerConfigBuilder {
            n1_max,
            // 100 ms chosen as a reasonable demo cadence; lower for tests.
            poll_interval1: Duration::from_millis(100),
            iterations: None,
            seed: None,
        }
    }
}

impl ProducerConfigBuilder {
    /// Override the inter-batch delay.
    #[must_use]
    pub fn poll_interval1(mut self, poll_interval1: Duration) -> Self {
        self.poll_interval1 = poll_interval1;
        self
    }

    /// Set a finite iteration count. Without this the producer runs until the
    /// buffer signals `Closed`.
    #[must_use]
    pub fn iterations(mut self, n: u64) -> Self {
        self.iterations = Some(n);
        self
    }

    /// Fix the RNG seed for deterministic output (useful in tests).
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Validate and build the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ProducerError::InvalidConfig`] when `n1_max` is zero.
    #[must_use = "the Result must be checked; use ? or unwrap"]
    pub fn build(self) -> Result<ProducerConfig, ProducerError> {
        if self.n1_max == 0 {
            return Err(ProducerError::InvalidConfig {
                reason: "n1_max must be >= 1".to_owned(),
            });
        }
        Ok(ProducerConfig {
            n1_max: self.n1_max,
            poll_interval1: self.poll_interval1,
            iterations: self.iterations,
            seed: self.seed,
        })
    }
}

// ---------------------------------------------------------------------------
// Producer
// ---------------------------------------------------------------------------

/// Last-name pool used for synthetic transaction generation.
///
/// 10 entries -- index always derived from `random_range(0..10)`, never panics.
const LAST_NAMES: &[&str] = &[
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Wilson",
    "Taylor",
];

/// Generates random transaction batches and forwards them to a [`Buffer1`] port.
///
/// Generic over `B: Buffer1` for zero-cost static dispatch. Holds no concrete
/// buffer reference -- dependency is injected per call (hexagonal architecture).
#[derive(Debug)]
pub struct Producer {
    config: ProducerConfig,
    /// Interior mutability required because all public methods take `&self`.
    rng: RefCell<StdRng>,
}

impl Producer {
    /// Create a new producer from `config`.
    ///
    /// Seeds the RNG from `config.seed` if set, otherwise from the OS.
    #[must_use]
    pub fn new(config: ProducerConfig) -> Self {
        let rng = match config.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        };
        Self {
            config,
            rng: RefCell::new(rng),
        }
    }

    /// Generate one batch of random transactions.
    ///
    /// Batch size is uniformly distributed in `[1, config.n1_max]`.
    /// Each transaction has a random UUID, an amount in `[0.01, 10_000.00]`
    /// (integer cents / 100), and a random last name from the built-in pool.
    #[must_use]
    pub fn generate_batch(&self) -> Vec<Transaction> {
        let mut rng = self.rng.borrow_mut();
        let size = rng.random_range(1..=self.config.n1_max);
        let mut batch = Vec::with_capacity(size);
        for _ in 0..size {
            // Build UUID from raw random bytes (no v4 fast-path needed).
            let mut bytes = [0u8; 16];
            rng.fill_bytes(&mut bytes);
            let id = uuid::Builder::from_random_bytes(bytes).into_uuid();

            // Integer cents avoids float-rounding during generation.
            // All values in [1, 1_000_000] are exactly representable as f64.
            let amount = f64::from(rng.random_range(1u32..=1_000_000u32)) / 100.0;

            // Index is always in bounds: derived from len().
            let last_name_idx = rng.random_range(0..LAST_NAMES.len());
            let last_name = LAST_NAMES[last_name_idx].to_owned();

            batch.push(Transaction {
                id,
                amount,
                last_name,
            });
        }
        batch
    }

    /// Generate one batch and write it to `buffer`.
    ///
    /// # Errors
    ///
    /// Propagates any [`BufferError`] wrapped in [`ProducerError::Buffer`].
    pub async fn produce_once<B: Buffer1>(&self, buffer: &B) -> Result<(), ProducerError> {
        let batch = self.generate_batch();
        log::debug!("producer.batch.generated: size={}", batch.len());
        buffer.write_batch(batch).await?;
        Ok(())
    }

    /// Run the production loop until stopped.
    ///
    /// Calls [`produce_once`](Self::produce_once) repeatedly, sleeping
    /// `config.poll_interval1` between iterations. Stops cleanly when:
    /// - the buffer signals [`BufferError::Closed`] (returns `Ok(())`), or
    /// - `config.iterations` batches have been written (returns `Ok(())`).
    ///
    /// # Errors
    ///
    /// Returns [`ProducerError::Buffer`] for any buffer error other than `Closed`.
    pub async fn run<B: Buffer1>(&self, buffer: &B) -> Result<(), ProducerError> {
        let mut count = 0u64;
        loop {
            match self.produce_once(buffer).await {
                Ok(()) => {}
                Err(ProducerError::Buffer {
                    source: BufferError::Closed,
                }) => {
                    log::info!("producer.run.stopped: buffer closed after {count} iteration(s)");
                    return Ok(());
                }
                Err(e) => return Err(e),
            }

            count += 1;
            log::info!("producer.batch.written: iteration={count}");

            if let Some(max) = self.config.iterations
                && count >= max
            {
                log::info!("producer.run.stopped: iteration limit reached");
                return Ok(());
            }

            tokio::time::sleep(self.config.poll_interval1).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{Producer, ProducerConfig, ProducerError};
    use domain::{Buffer1, BufferError, Transaction};
    use std::cell::RefCell;
    use std::time::Duration;

    // ------------------------------------------------------------------
    // Test helpers
    // ------------------------------------------------------------------

    /// In-memory buffer that tracks individual batches for assertion.
    struct TestBuffer {
        batches: RefCell<Vec<Vec<Transaction>>>,
    }

    impl TestBuffer {
        fn new() -> Self {
            Self {
                batches: RefCell::new(vec![]),
            }
        }

        fn batch_count(&self) -> usize {
            self.batches.borrow().len()
        }

        fn total_tx_count(&self) -> usize {
            self.batches.borrow().iter().map(Vec::len).sum()
        }
    }

    impl Buffer1 for TestBuffer {
        async fn write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError> {
            self.batches.borrow_mut().push(batch);
            Ok(())
        }
    }

    /// Buffer that immediately signals `Closed`.
    struct ClosedBuffer;

    impl Buffer1 for ClosedBuffer {
        async fn write_batch(&self, _batch: Vec<Transaction>) -> Result<(), BufferError> {
            Err(BufferError::Closed)
        }
    }

    /// Buffer that immediately signals `Full`.
    struct FullBuffer;

    impl Buffer1 for FullBuffer {
        async fn write_batch(&self, _batch: Vec<Transaction>) -> Result<(), BufferError> {
            Err(BufferError::Full { capacity: 0 })
        }
    }

    // ------------------------------------------------------------------
    // US1: configuration + batch generation
    // ------------------------------------------------------------------

    #[test]
    fn config_rejects_zero() {
        let result = ProducerConfig::builder(0).build();
        assert!(matches!(result, Err(ProducerError::InvalidConfig { .. })));
    }

    #[test]
    fn batch_size_bounds() {
        // Seed-fixed producer; generate 100 batches, verify all sizes in [1, 10]
        // and that every value in [1, 10] appears at least once.
        let config = ProducerConfig::builder(10).seed(1).build().unwrap();
        let producer = Producer::new(config);
        let mut seen = [false; 11]; // index 1..=10
        for _ in 0..100 {
            let batch = producer.generate_batch();
            let sz = batch.len();
            assert!((1..=10).contains(&sz), "batch size {sz} out of [1, 10]");
            seen[sz] = true;
        }
        #[expect(
            clippy::needless_range_loop,
            reason = "index 0 is intentionally skipped"
        )]
        for i in 1..=10 {
            assert!(seen[i], "size {i} never appeared in 100 batches");
        }
    }

    #[test]
    fn tx_fields_valid() {
        let config = ProducerConfig::builder(10).seed(2).build().unwrap();
        let producer = Producer::new(config);
        let batch = producer.generate_batch();
        assert!(!batch.is_empty());
        for tx in &batch {
            // UUID can always be round-tripped through its string form.
            let parsed = tx.id.to_string().parse::<uuid::Uuid>().unwrap();
            assert_eq!(parsed, tx.id, "id must be a valid UUID");
            assert!(
                tx.amount >= 0.01_f64 && tx.amount <= 10_000.00_f64,
                "amount {} out of range",
                tx.amount
            );
            assert!(!tx.last_name.is_empty(), "last_name must be non-empty");
        }
    }

    #[test]
    fn seeded_rng_deterministic() {
        let c1 = ProducerConfig::builder(10).seed(99).build().unwrap();
        let c2 = ProducerConfig::builder(10).seed(99).build().unwrap();
        let batch1 = Producer::new(c1).generate_batch();
        let batch2 = Producer::new(c2).generate_batch();
        assert_eq!(
            batch1, batch2,
            "identical seeds must produce identical batches"
        );
    }

    // ------------------------------------------------------------------
    // US2: produce_once + buffer write
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn produce_and_write() {
        let config = ProducerConfig::builder(10).seed(42).build().unwrap();
        let producer = Producer::new(config);
        let buffer = TestBuffer::new();

        producer.produce_once(&buffer).await.unwrap();

        assert_eq!(buffer.batch_count(), 1);
        let sz = buffer.total_tx_count();
        assert!((1..=10).contains(&sz), "batch size {sz} out of [1, 10]");
    }

    // ------------------------------------------------------------------
    // US3: run loop
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn run_n_iterations() {
        let config = ProducerConfig::builder(10)
            .seed(7)
            .iterations(5)
            .poll_interval1(Duration::ZERO)
            .build()
            .unwrap();
        let producer = Producer::new(config);
        let buffer = TestBuffer::new();

        producer.run(&buffer).await.unwrap();

        assert_eq!(buffer.batch_count(), 5, "expected exactly 5 batches");
        let total = buffer.total_tx_count();
        // 5 batches, each 1..=10 transactions
        assert!(
            (5..=50).contains(&total),
            "total tx count {total} out of expected range"
        );
    }

    #[tokio::test]
    async fn run_stops_on_closed() {
        let config = ProducerConfig::builder(10)
            .poll_interval1(Duration::ZERO)
            .build()
            .unwrap();
        let producer = Producer::new(config);
        let result = producer.run(&ClosedBuffer).await;
        assert!(result.is_ok(), "Closed must terminate cleanly: {result:?}");
    }

    #[tokio::test]
    async fn run_propagates_full() {
        let config = ProducerConfig::builder(10)
            .poll_interval1(Duration::ZERO)
            .build()
            .unwrap();
        let producer = Producer::new(config);
        let result = producer.run(&FullBuffer).await;
        assert!(
            matches!(
                result,
                Err(ProducerError::Buffer {
                    source: BufferError::Full { .. }
                })
            ),
            "Full error must be propagated: {result:?}"
        );
    }
}
