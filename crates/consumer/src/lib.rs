// Rust guideline compliant 2026-02-23

//! Consumer component -- reads transaction batches from Buffer1, runs inference,
//! triggers fraud alarms, and writes results to Buffer2.
//!
//! Entry points: [`Consumer::consume_once`], [`Consumer::run`],
//! [`Consumer::switch_model_version`]. Configuration via [`ConsumerConfig::builder`].

use domain::{Alarm, AlarmError, Buffer1Read, Buffer2, BufferError, Modelizer, ModelizerError, ModelVersion};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::cell::RefCell;
use std::time::Duration;

// ---------------------------------------------------------------------------
// ConsumerError
// ---------------------------------------------------------------------------

/// Errors that can occur during transaction consumption.
#[derive(Debug, thiserror::Error)]
pub enum ConsumerError {
    /// The supplied configuration is invalid.
    #[error("invalid consumer configuration: {reason}")]
    InvalidConfig {
        /// Human-readable description of the problem.
        reason: String,
    },
    /// A Buffer1 read failed.
    #[error("buffer1 read error: {0}")]
    Read(BufferError),
    /// Modelizer inference or version-switch failed.
    #[error("modelizer error: {0}")]
    Inference(ModelizerError),
    /// A Buffer2 write failed.
    #[error("buffer2 write error: {0}")]
    Write(BufferError),
}

// ---------------------------------------------------------------------------
// ConsumerConfig + builder
// ---------------------------------------------------------------------------

/// Runtime configuration for a [`Consumer`].
///
/// Construct via [`ConsumerConfig::builder`].
#[derive(Debug)]
pub struct ConsumerConfig {
    /// Maximum number of transactions per batch (range: `[1, n2_max]`).
    pub n2_max: usize,
    /// Delay between successive batch-processing iterations.
    pub speed2: Duration,
    /// Optional upper bound on the number of iterations. `None` means infinite.
    pub iterations: Option<u64>,
    /// Optional RNG seed for reproducible batch sizes. `None` seeds from the OS.
    pub seed: Option<u64>,
}

/// Builder for [`ConsumerConfig`].
///
/// Obtain via [`ConsumerConfig::builder`]; finalize with [`build`](Self::build).
#[derive(Debug)]
pub struct ConsumerConfigBuilder {
    n2_max: usize,
    speed2: Duration,
    iterations: Option<u64>,
    seed: Option<u64>,
}

impl ConsumerConfig {
    /// Create a builder. `n2_max` is the only required parameter.
    ///
    /// Default values: `speed2 = 100 ms`, `iterations = None`, `seed = None`.
    #[must_use]
    pub fn builder(n2_max: usize) -> ConsumerConfigBuilder {
        ConsumerConfigBuilder {
            n2_max,
            // 100 ms chosen as a reasonable demo cadence; lower for tests.
            speed2: Duration::from_millis(100),
            iterations: None,
            seed: None,
        }
    }
}

impl ConsumerConfigBuilder {
    /// Override the inter-iteration delay.
    #[must_use]
    pub fn speed2(mut self, speed2: Duration) -> Self {
        self.speed2 = speed2;
        self
    }

    /// Set a finite iteration count. Without this the consumer runs until the
    /// buffer signals `Closed`.
    #[must_use]
    pub fn iterations(mut self, n: u64) -> Self {
        self.iterations = Some(n);
        self
    }

    /// Fix the RNG seed for deterministic batch sizes (useful in tests).
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Validate and build the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConsumerError::InvalidConfig`] when `n2_max` is zero.
    #[must_use = "the Result must be checked; use ? or unwrap"]
    pub fn build(self) -> Result<ConsumerConfig, ConsumerError> {
        if self.n2_max == 0 {
            return Err(ConsumerError::InvalidConfig {
                reason: "n2_max must be >= 1".to_owned(),
            });
        }
        Ok(ConsumerConfig {
            n2_max: self.n2_max,
            speed2: self.speed2,
            iterations: self.iterations,
            seed: self.seed,
        })
    }
}

// ---------------------------------------------------------------------------
// Consumer
// ---------------------------------------------------------------------------

/// Reads batches from Buffer1, infers with Modelizer, triggers alarms for
/// fraudulent transactions, and writes all results to Buffer2.
///
/// Generic over all four hexagonal ports for zero-cost static dispatch.
/// Holds no concrete adapter references -- dependencies are injected per call.
#[derive(Debug)]
pub struct Consumer {
    config: ConsumerConfig,
    /// Interior mutability required because all public methods take `&self`.
    rng: RefCell<StdRng>,
}

impl Consumer {
    /// Create a new consumer from `config`.
    ///
    /// Seeds the RNG from `config.seed` if set, otherwise from the OS.
    #[must_use]
    pub fn new(config: ConsumerConfig) -> Self {
        let rng = match config.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        };
        Self { config, rng: RefCell::new(rng) }
    }

    /// Read one batch from Buffer1, infer via Modelizer, trigger best-effort
    /// alarms for fraudulent transactions, and write all results to Buffer2.
    ///
    /// Returns collected alarm failures in `Ok(vec)`; hard errors propagate as `Err`.
    ///
    /// # Errors
    ///
    /// Returns [`ConsumerError::Read`] on Buffer1 failure (including `Closed`),
    /// [`ConsumerError::Inference`] on Modelizer failure, or
    /// [`ConsumerError::Write`] on Buffer2 failure.
    pub async fn consume_once<B1, M, A, B2>(
        &self,
        buf1: &B1,
        modelizer: &M,
        alarm: &A,
        buf2: &B2,
    ) -> Result<Vec<AlarmError>, ConsumerError>
    where
        B1: Buffer1Read,
        M: Modelizer,
        A: Alarm,
        B2: Buffer2,
    {
        let n2 = self.rng.borrow_mut().random_range(1..=self.config.n2_max);
        let batch = buf1.read_batch(n2).await.map_err(ConsumerError::Read)?;

        log::debug!("consumer.batch.read: size={}", batch.len());

        let inferred = modelizer.infer(batch).await.map_err(ConsumerError::Inference)?;

        // Best-effort alarm delivery: attempt every fraudulent transaction,
        // collect failures without aborting the batch.
        let mut alarm_errors: Vec<AlarmError> = vec![];
        for tx in &inferred {
            if tx.predicted_fraud && let Err(e) = alarm.trigger(tx).await {
                alarm_errors.push(e);
            }
        }

        buf2.write_batch(inferred).await.map_err(ConsumerError::Write)?;

        Ok(alarm_errors)
    }

    /// Run the consumption loop until stopped.
    ///
    /// Calls [`consume_once`](Self::consume_once) repeatedly, sleeping `speed2`
    /// between iterations. Stops cleanly when:
    /// - Buffer1 signals [`BufferError::Closed`] (returns `Ok(())`), or
    /// - `config.iterations` batches have been processed (returns `Ok(())`).
    ///
    /// Alarm failures within a batch are logged as warnings but do not abort the loop.
    ///
    /// # Errors
    ///
    /// Returns [`ConsumerError`] for any hard error other than Buffer1 `Closed`.
    pub async fn run<B1, M, A, B2>(
        &self,
        buf1: &B1,
        modelizer: &M,
        alarm: &A,
        buf2: &B2,
    ) -> Result<(), ConsumerError>
    where
        B1: Buffer1Read,
        M: Modelizer,
        A: Alarm,
        B2: Buffer2,
    {
        let mut count = 0u64;
        loop {
            match self.consume_once(buf1, modelizer, alarm, buf2).await {
                Ok(alarm_errs) => {
                    for e in &alarm_errs {
                        log::warn!("consumer.alarm.failed: error={e}");
                    }
                }
                Err(ConsumerError::Read(BufferError::Closed)) => {
                    log::info!(
                        "consumer.run.stopped: buffer closed after {count} iteration(s)"
                    );
                    return Ok(());
                }
                Err(e) => return Err(e),
            }

            count += 1;
            log::info!("consumer.batch.processed: iteration={count}");

            if let Some(max) = self.config.iterations
                && count >= max
            {
                log::info!("consumer.run.stopped: iteration limit reached");
                return Ok(());
            }

            tokio::time::sleep(self.config.speed2).await;
        }
    }

    /// Delegate a model version switch to the Modelizer port.
    ///
    /// Consumer holds no version state; Modelizer owns it internally.
    ///
    /// # Errors
    ///
    /// Returns [`ConsumerError::Inference`] if the switch fails.
    pub async fn switch_model_version<M: Modelizer>(
        &self,
        modelizer: &M,
        version: ModelVersion,
    ) -> Result<(), ConsumerError> {
        modelizer.switch_version(version).await.map_err(ConsumerError::Inference)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{Consumer, ConsumerConfig, ConsumerError};
    use domain::{
        Alarm, AlarmError, Buffer1Read, Buffer2, BufferError, InferredTransaction,
        Modelizer, ModelizerError, ModelVersion, Transaction,
    };
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::time::Duration;

    // ------------------------------------------------------------------
    // Test helpers
    // ------------------------------------------------------------------

    fn make_tx() -> Transaction {
        Transaction {
            id: uuid::Uuid::new_v4(),
            amount: 1.00_f64,
            last_name: "Test".to_owned(),
        }
    }

    fn make_txs(n: usize) -> Vec<Transaction> {
        (0..n).map(|_| make_tx()).collect()
    }

    fn make_consumer(n2_max: usize, seed: u64) -> Consumer {
        Consumer::new(
            ConsumerConfig::builder(n2_max)
                .seed(seed)
                .speed2(Duration::ZERO)
                .build()
                .unwrap(),
        )
    }

    // ------------------------------------------------------------------
    // Mock adapters (T017)
    // ------------------------------------------------------------------

    struct MockBuffer1Read {
        transactions: RefCell<VecDeque<Transaction>>,
    }

    impl MockBuffer1Read {
        fn new(transactions: Vec<Transaction>) -> Self {
            Self { transactions: RefCell::new(VecDeque::from(transactions)) }
        }
    }

    impl Buffer1Read for MockBuffer1Read {
        async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError> {
            let mut queue = self.transactions.borrow_mut();
            if queue.is_empty() {
                return Err(BufferError::Closed);
            }
            let count = max.min(queue.len());
            Ok(queue.drain(..count).collect())
        }
    }

    struct MockModelizer {
        predicted_fraud: bool,
        infer_call_count: Cell<u32>,
        last_batch_size: Cell<usize>,
        last_switch: Cell<Option<ModelVersion>>,
        fail_infer: bool,
        fail_switch: bool,
    }

    impl MockModelizer {
        fn new(predicted_fraud: bool) -> Self {
            Self {
                predicted_fraud,
                infer_call_count: Cell::new(0),
                last_batch_size: Cell::new(0),
                last_switch: Cell::new(None),
                fail_infer: false,
                fail_switch: false,
            }
        }

        fn failing_infer() -> Self {
            Self { fail_infer: true, ..Self::new(false) }
        }

        fn failing_switch() -> Self {
            Self { fail_switch: true, ..Self::new(false) }
        }
    }

    impl Modelizer for MockModelizer {
        async fn infer(
            &self,
            batch: Vec<Transaction>,
        ) -> Result<Vec<InferredTransaction>, ModelizerError> {
            if self.fail_infer {
                return Err(ModelizerError::InferenceFailed {
                    reason: "mock failure".to_owned(),
                });
            }
            self.infer_call_count.set(self.infer_call_count.get() + 1);
            self.last_batch_size.set(batch.len());
            Ok(batch
                .into_iter()
                .map(|tx| InferredTransaction {
                    predicted_fraud: self.predicted_fraud,
                    model_name: "MOCK".to_owned(),
                    model_version: "v_test".to_owned(),
                    transaction: tx,
                })
                .collect())
        }

        async fn switch_version(
            &self,
            version: ModelVersion,
        ) -> Result<(), ModelizerError> {
            if self.fail_switch {
                return Err(ModelizerError::SwitchFailed {
                    reason: "mock failure".to_owned(),
                });
            }
            self.last_switch.set(Some(version));
            Ok(())
        }
    }

    struct MockAlarm {
        call_count: Cell<u32>,
        always_fail: bool,
    }

    impl MockAlarm {
        fn new() -> Self {
            Self { call_count: Cell::new(0), always_fail: false }
        }

        fn always_failing() -> Self {
            Self { call_count: Cell::new(0), always_fail: true }
        }
    }

    impl Alarm for MockAlarm {
        async fn trigger(
            &self,
            transaction: &InferredTransaction,
        ) -> Result<(), AlarmError> {
            self.call_count.set(self.call_count.get() + 1);
            if self.always_fail {
                return Err(AlarmError::DeliveryFailed {
                    reason: format!("mock fail for tx {}", transaction.id()),
                });
            }
            Ok(())
        }
    }

    struct MockBuffer2 {
        captured: RefCell<Vec<InferredTransaction>>,
        fail: Option<BufferError>,
    }

    impl MockBuffer2 {
        fn new() -> Self {
            Self { captured: RefCell::new(vec![]), fail: None }
        }

        fn with_fail(error: BufferError) -> Self {
            Self { captured: RefCell::new(vec![]), fail: Some(error) }
        }
    }

    impl Buffer2 for MockBuffer2 {
        async fn write_batch(
            &self,
            batch: Vec<InferredTransaction>,
        ) -> Result<(), BufferError> {
            if let Some(e) = &self.fail {
                return Err(e.clone());
            }
            self.captured.borrow_mut().extend(batch);
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // T015: ConsumerConfig validation
    // ------------------------------------------------------------------

    #[test]
    fn config_rejects_zero_n2_max() {
        let result = ConsumerConfig::builder(0).build();
        assert!(matches!(result, Err(ConsumerError::InvalidConfig { .. })));
    }

    #[test]
    fn builder_defaults_speed2() {
        let config = ConsumerConfig::builder(10).build().unwrap();
        assert_eq!(config.speed2, Duration::from_millis(100));
    }

    #[test]
    fn builder_with_seed() {
        let config = ConsumerConfig::builder(10).seed(42).build().unwrap();
        assert_eq!(config.seed, Some(42));
    }

    #[test]
    fn builder_with_iterations() {
        let config = ConsumerConfig::builder(10).iterations(5).build().unwrap();
        assert_eq!(config.iterations, Some(5));
    }

    // ------------------------------------------------------------------
    // T018: US1 -- read behavior
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn batch_size_within_n2_max_range() {
        let n2_max = 10;
        let consumer = make_consumer(n2_max, 1);
        let buf1 = MockBuffer1Read::new(make_txs(1000));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        let sz = modelizer.last_batch_size.get();
        assert!(sz >= 1 && sz <= n2_max, "batch size {sz} out of [1, {n2_max}]");
    }

    #[tokio::test]
    async fn batch_size_capped_by_available_data() {
        // n2_max >> available transactions; buffer caps the returned batch.
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(3));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(modelizer.last_batch_size.get(), 3);
    }

    #[tokio::test]
    async fn seeded_batch_size_is_deterministic() {
        let c1 = make_consumer(10, 99);
        let c2 = make_consumer(10, 99);
        let buf1_a = MockBuffer1Read::new(make_txs(100));
        let buf1_b = MockBuffer1Read::new(make_txs(100));
        let m1 = MockModelizer::new(false);
        let m2 = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        c1.consume_once(&buf1_a, &m1, &alarm, &buf2).await.unwrap();
        c2.consume_once(&buf1_b, &m2, &alarm, &buf2).await.unwrap();

        assert_eq!(
            m1.last_batch_size.get(),
            m2.last_batch_size.get(),
            "identical seeds must request identical batch sizes"
        );
    }

    // ------------------------------------------------------------------
    // T019: US1 -- run loop
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn run_processes_n_iterations() {
        let consumer = Consumer::new(
            ConsumerConfig::builder(10)
                .seed(7)
                .iterations(3)
                .speed2(Duration::ZERO)
                .build()
                .unwrap(),
        );
        let buf1 = MockBuffer1Read::new(make_txs(1000));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.run(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(modelizer.infer_call_count.get(), 3, "expected 3 infer calls");
    }

    #[tokio::test]
    async fn run_stops_gracefully_on_closed() {
        let consumer = make_consumer(10, 1);
        // Empty buffer signals Closed immediately.
        let buf1 = MockBuffer1Read::new(vec![]);
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        let result = consumer.run(&buf1, &modelizer, &alarm, &buf2).await;
        assert!(result.is_ok(), "Closed must terminate cleanly: {result:?}");
    }

    // ------------------------------------------------------------------
    // T022: US2 -- Modelizer interaction
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn consume_once_sends_full_batch_to_modelizer() {
        // 10 transactions preloaded, n2_max = 100 -> all 10 reach Modelizer.
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(10));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(modelizer.last_batch_size.get(), 10);
    }

    #[tokio::test]
    async fn inference_error_propagates_as_consumer_error_inference() {
        let consumer = make_consumer(10, 1);
        let buf1 = MockBuffer1Read::new(make_txs(10));
        let modelizer = MockModelizer::failing_infer();
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        let result = consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await;
        assert!(
            matches!(result, Err(ConsumerError::Inference(_))),
            "inference failure must map to ConsumerError::Inference: {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // T023: US2 -- InferredTransaction enrichment fields
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn inferred_tx_enrichment_fields_survive_pipeline() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(2));
        let modelizer = MockModelizer::new(true);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        let captured = buf2.captured.borrow();
        assert_eq!(captured.len(), 2);
        for tx in captured.iter() {
            assert!(tx.predicted_fraud);
            assert_eq!(tx.model_name, "MOCK");
            assert_eq!(tx.model_version, "v_test");
        }
    }

    // ------------------------------------------------------------------
    // T025: US4 -- Buffer2 write
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn all_inferred_tx_written_to_buf2_regardless_of_fraud() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(5));
        let modelizer = MockModelizer::new(true); // all fraudulent
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(buf2.captured.borrow().len(), 5, "all 5 must reach Buffer2");
    }

    #[tokio::test]
    async fn buf2_full_propagates_as_consumer_error_write() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(5));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::with_fail(BufferError::Full { capacity: 0 });

        let result = consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await;
        assert!(
            matches!(result, Err(ConsumerError::Write(BufferError::Full { .. }))),
            "Full must map to ConsumerError::Write: {result:?}"
        );
    }

    #[tokio::test]
    async fn buf2_closed_propagates_as_consumer_error_write() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(5));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::with_fail(BufferError::Closed);

        let result = consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await;
        assert!(
            matches!(result, Err(ConsumerError::Write(BufferError::Closed))),
            "Closed must map to ConsumerError::Write: {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // T027: US3 -- alarm count
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn exactly_n_alarms_for_n_fraudulent_tx() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(5));
        let modelizer = MockModelizer::new(true); // all 5 fraudulent
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(alarm.call_count.get(), 5, "5 alarms for 5 fraudulent tx");
    }

    #[tokio::test]
    async fn no_alarms_for_zero_fraudulent_tx() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(5));
        let modelizer = MockModelizer::new(false); // none fraudulent
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(alarm.call_count.get(), 0, "0 alarms when none fraudulent");
    }

    #[tokio::test]
    async fn zero_alarms_when_all_legitimate() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(3));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(alarm.call_count.get(), 0);
    }

    // ------------------------------------------------------------------
    // T028: US3 -- best-effort alarm delivery
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn all_alarms_attempted_on_partial_failure() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(4));
        let modelizer = MockModelizer::new(true); // all 4 fraudulent
        let alarm = MockAlarm::always_failing();
        let buf2 = MockBuffer2::new();

        let result = consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await;
        assert!(result.is_ok(), "alarm failures must not abort consume_once: {result:?}");

        assert_eq!(alarm.call_count.get(), 4, "all 4 alarms must be attempted");
    }

    #[tokio::test]
    async fn alarm_failures_returned_in_ok_vec() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(3));
        let modelizer = MockModelizer::new(true); // all 3 fraudulent
        let alarm = MockAlarm::always_failing();
        let buf2 = MockBuffer2::new();

        let alarm_errors = consumer
            .consume_once(&buf1, &modelizer, &alarm, &buf2)
            .await
            .unwrap();

        assert_eq!(alarm_errors.len(), 3, "3 failures for 3 fraudulent tx");
    }

    #[tokio::test]
    async fn buf2_write_not_blocked_by_alarm_failure() {
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(2));
        let modelizer = MockModelizer::new(true); // both fraudulent
        let alarm = MockAlarm::always_failing();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert_eq!(buf2.captured.borrow().len(), 2, "Buffer2 write must proceed");
    }

    // ------------------------------------------------------------------
    // T030: US5 -- model version switch
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn switch_to_n_minus1_calls_modelizer_switch_version() {
        let consumer = make_consumer(10, 1);
        let modelizer = MockModelizer::new(false);

        consumer
            .switch_model_version(&modelizer, ModelVersion::NMinus1)
            .await
            .unwrap();

        assert_eq!(modelizer.last_switch.get(), Some(ModelVersion::NMinus1));
    }

    #[tokio::test]
    async fn switch_to_n_calls_modelizer_switch_version() {
        let consumer = make_consumer(10, 1);
        let modelizer = MockModelizer::new(false);

        consumer
            .switch_model_version(&modelizer, ModelVersion::N)
            .await
            .unwrap();

        assert_eq!(modelizer.last_switch.get(), Some(ModelVersion::N));
    }

    #[tokio::test]
    async fn switch_error_maps_to_consumer_error_inference() {
        let consumer = make_consumer(10, 1);
        let modelizer = MockModelizer::failing_switch();

        let result = consumer
            .switch_model_version(&modelizer, ModelVersion::N)
            .await;

        assert!(
            matches!(result, Err(ConsumerError::Inference(_))),
            "switch failure must map to ConsumerError::Inference: {result:?}"
        );
    }

    #[tokio::test]
    async fn default_model_version_is_n() {
        // Consumer must not call switch_version implicitly before infer.
        let consumer = make_consumer(100, 1);
        let buf1 = MockBuffer1Read::new(make_txs(5));
        let modelizer = MockModelizer::new(false);
        let alarm = MockAlarm::new();
        let buf2 = MockBuffer2::new();

        consumer.consume_once(&buf1, &modelizer, &alarm, &buf2).await.unwrap();

        assert!(
            modelizer.last_switch.get().is_none(),
            "Consumer must not call switch_version implicitly"
        );
        assert_eq!(modelizer.infer_call_count.get(), 1, "infer must be called once");
    }
}
