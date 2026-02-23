// Rust guideline compliant 2026-02-23

//! Shared domain types for the fraud-detection pipeline.
//!
//! Defines `Transaction`, `BufferError`, and the hexagonal port traits:
//! `Buffer1`, `Buffer1Read`, `Buffer2`, `Model`, `Modelizer`, and `Alarm`.
//! All pipeline components depend on this crate; no other crate is imported here.

/// A single banking transaction produced by the pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    /// Unique identifier (UUID v4-compatible random bytes).
    pub id: uuid::Uuid,
    /// Transaction amount in euros, range `[0.01, 10_000.00]`.
    pub amount: f64,
    /// Account holder last name.
    pub last_name: String,
}

/// A transaction enriched with Modelizer inference results.
#[derive(Debug, Clone, PartialEq)]
pub struct InferredTransaction {
    /// Original transaction (composition).
    pub transaction: Transaction,
    /// `true` if Modelizer flagged as fraudulent.
    pub predicted_fraud: bool,
    /// Name of the model used (e.g. "DINN").
    pub model_name: String,
    /// Version string of the model used (e.g. "v1").
    pub model_version: String,
}

impl InferredTransaction {
    /// Return the transaction ID, delegating to the wrapped transaction.
    #[must_use]
    pub fn id(&self) -> uuid::Uuid {
        self.transaction.id
    }
}

/// Selectable model version for Modelizer switch commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelVersion {
    /// Latest model version (default).
    N,
    /// Previous model version.
    NMinus1,
}

/// Errors from the Modelizer hexagonal port.
#[derive(Debug, thiserror::Error)]
pub enum ModelizerError {
    /// Inference could not be completed.
    #[error("inference failed: {reason}")]
    InferenceFailed {
        /// Human-readable description.
        reason: String,
    },
    /// Version switch could not be applied.
    #[error("switch failed: {reason}")]
    SwitchFailed {
        /// Human-readable description.
        reason: String,
    },
}

/// Errors from the Alarm hexagonal port.
#[derive(Debug, thiserror::Error)]
pub enum AlarmError {
    /// Alarm could not be delivered.
    #[error("delivery failed: {reason}")]
    DeliveryFailed {
        /// Human-readable description.
        reason: String,
    },
}

/// Errors that a buffer implementation may return.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BufferError {
    /// Buffer has reached its maximum capacity.
    #[error("buffer full (capacity: {capacity})")]
    Full { capacity: usize },
    /// Buffer has been closed; no further writes are accepted.
    #[error("buffer closed")]
    Closed,
}

/// Hexagonal port: the write side of the first inter-component buffer.
///
/// Implementations live outside the domain and producer crates (e.g. in the
/// binary crate). `Producer` depends exclusively on this trait -- never on a
/// concrete adapter.
#[expect(
    async_fn_in_trait,
    reason = "no dyn dispatch needed; internal workspace only"
)]
pub trait Buffer1 {
    /// Write a batch of transactions into the buffer.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::Full` when capacity is exceeded, or
    /// `BufferError::Closed` when the buffer has been shut down.
    async fn write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError>;
}

/// Hexagonal port: the read side of the first inter-component buffer.
///
/// Consumer depends exclusively on this trait -- never on a concrete adapter.
/// Implementations signal exhaustion via `BufferError::Closed`.
#[expect(
    async_fn_in_trait,
    reason = "no dyn dispatch needed; internal workspace only"
)]
pub trait Buffer1Read {
    /// Read up to `max` transactions from the buffer.
    ///
    /// Returns between 1 and `max` transactions when data is available.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::Closed` when the buffer is closed and drained.
    async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError>;
}

/// Hexagonal port: the write side of the second inter-component buffer.
///
/// Consumer writes inferred transactions here after classification.
#[expect(
    async_fn_in_trait,
    reason = "no dyn dispatch needed; internal workspace only"
)]
pub trait Buffer2 {
    /// Write a batch of inferred transactions into the buffer.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::Full` when capacity is exceeded, or
    /// `BufferError::Closed` when the buffer has been shut down.
    async fn write_batch(&self, batch: Vec<InferredTransaction>) -> Result<(), BufferError>;
}

/// Hexagonal port: per-transaction classification model.
///
/// Implemented by concrete model adapters (e.g. `DemoModel`). The Modelizer
/// component depends exclusively on this trait -- never on a concrete adapter.
/// Adapters are responsible for mapping abstract `ModelVersion` to their own
/// concrete version identifiers.
#[expect(
    async_fn_in_trait,
    reason = "no dyn dispatch needed; internal workspace only"
)]
pub trait Model {
    /// Classify a single transaction as fraudulent or legitimate.
    ///
    /// # Errors
    ///
    /// Returns `ModelizerError::InferenceFailed` if classification fails.
    async fn classify(&self, tx: &Transaction) -> Result<bool, ModelizerError>;

    /// Name of this model (e.g. `"DEMO"`).
    fn name(&self) -> &str;

    /// Version string for the currently active version (e.g. `"4"`).
    fn active_version(&self) -> &str;

    /// Switch to a different model version; takes effect on the next `classify` call.
    ///
    /// # Errors
    ///
    /// Returns `ModelizerError::SwitchFailed` if the switch cannot be applied.
    async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError>;
}

/// Hexagonal port: inference and version-switching for transaction classification.
///
/// Consumer calls `infer` once per batch and `switch_version` to change models.
#[expect(
    async_fn_in_trait,
    reason = "no dyn dispatch needed; internal workspace only"
)]
pub trait Modelizer {
    /// Run inference on a batch of transactions.
    ///
    /// Returns one `InferredTransaction` per input (same order, same count).
    ///
    /// # Errors
    ///
    /// Returns `ModelizerError::InferenceFailed` on failure.
    async fn infer(
        &self,
        batch: Vec<Transaction>,
    ) -> Result<Vec<InferredTransaction>, ModelizerError>;

    /// Switch to a different model version; takes effect on the next `infer` call.
    ///
    /// # Errors
    ///
    /// Returns `ModelizerError::SwitchFailed` on failure.
    async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError>;
}

/// Hexagonal port: per-transaction fraud alert delivery.
///
/// Consumer calls `trigger` once per fraudulent transaction per batch (best-effort).
#[expect(
    async_fn_in_trait,
    reason = "no dyn dispatch needed; internal workspace only"
)]
pub trait Alarm {
    /// Trigger a fraud alert for the given transaction.
    ///
    /// # Errors
    ///
    /// Returns `AlarmError::DeliveryFailed` when the alert cannot be delivered.
    async fn trigger(&self, transaction: &InferredTransaction) -> Result<(), AlarmError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    // ------------------------------------------------------------------
    // Existing tests
    // ------------------------------------------------------------------

    #[test]
    // 42.00 is an exact integer-valued f64 literal; assert_eq! is intentional.
    #[expect(clippy::float_cmp, reason = "exact integer-valued literal")]
    fn transaction_fields() {
        let id = uuid::Uuid::new_v4();
        let tx = Transaction {
            id,
            amount: 42.00_f64,
            last_name: "Smith".to_owned(),
        };
        assert_eq!(tx.id, id);
        assert_eq!(tx.amount, 42.00_f64);
        assert_eq!(tx.last_name, "Smith");
    }

    #[test]
    fn buffer_error_variants() {
        let full = BufferError::Full { capacity: 10 };
        let closed = BufferError::Closed;
        assert_eq!(full, BufferError::Full { capacity: 10 });
        assert_eq!(closed, BufferError::Closed);
        assert_ne!(full, closed);
    }

    /// Verify that a minimal `Buffer1` implementation stores transactions correctly.
    #[tokio::test]
    async fn buffer1_impl() {
        struct TestBuffer {
            inner: RefCell<Vec<Transaction>>,
        }

        impl Buffer1 for TestBuffer {
            async fn write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError> {
                self.inner.borrow_mut().extend(batch);
                Ok(())
            }
        }

        let buf = TestBuffer {
            inner: RefCell::new(vec![]),
        };
        let tx = Transaction {
            id: uuid::Uuid::new_v4(),
            amount: 1.00_f64,
            last_name: "Test".to_owned(),
        };
        buf.write_batch(vec![tx.clone()]).await.unwrap();
        assert_eq!(buf.inner.borrow().len(), 1);
        assert_eq!(buf.inner.borrow()[0], tx);
    }

    // ------------------------------------------------------------------
    // T012: new domain type + trait tests
    // ------------------------------------------------------------------

    #[test]
    fn inferred_transaction_fields() {
        let id = uuid::Uuid::new_v4();
        let tx = Transaction { id, amount: 99.99_f64, last_name: "Dupont".to_owned() };
        let inferred = InferredTransaction {
            transaction: tx.clone(),
            predicted_fraud: true,
            model_name: "DINN".to_owned(),
            model_version: "v1".to_owned(),
        };
        assert_eq!(inferred.id(), tx.id);
        assert!(inferred.predicted_fraud);
        assert_eq!(inferred.model_name, "DINN");
        assert_eq!(inferred.model_version, "v1");
        assert_eq!(inferred.transaction, tx);
    }

    #[test]
    fn model_version_variants() {
        assert_ne!(ModelVersion::N, ModelVersion::NMinus1);
        assert_eq!(ModelVersion::N, ModelVersion::N);
        // Copy semantics.
        let v = ModelVersion::NMinus1;
        let w = v;
        assert_eq!(v, w);
    }

    #[test]
    fn modelizer_error_variants() {
        let e1 = ModelizerError::InferenceFailed { reason: "oops".to_owned() };
        let e2 = ModelizerError::SwitchFailed { reason: "cant".to_owned() };
        assert_eq!(e1.to_string(), "inference failed: oops");
        assert_eq!(e2.to_string(), "switch failed: cant");
    }

    #[test]
    fn alarm_error_variants() {
        let e = AlarmError::DeliveryFailed { reason: "timeout".to_owned() };
        assert_eq!(e.to_string(), "delivery failed: timeout");
    }

    // ------------------------------------------------------------------
    // T004: Model trait -- compile check
    // ------------------------------------------------------------------

    /// Verify that a minimal `Model` implementation compiles and satisfies all methods.
    #[tokio::test]
    async fn model_trait_compiles_with_minimal_impl() {
        struct MinimalModel;

        impl Model for MinimalModel {
            async fn classify(&self, _tx: &Transaction) -> Result<bool, ModelizerError> {
                Ok(false)
            }

            fn name(&self) -> &str {
                "minimal"
            }

            fn active_version(&self) -> &str {
                "0"
            }

            async fn switch_version(&self, _version: ModelVersion) -> Result<(), ModelizerError> {
                Ok(())
            }
        }

        let m = MinimalModel;
        let tx = Transaction {
            id: uuid::Uuid::new_v4(),
            amount: 1.00_f64,
            last_name: "T".to_owned(),
        };
        let fraud = m.classify(&tx).await.unwrap();
        assert!(!fraud);
        assert_eq!(m.name(), "minimal");
        assert_eq!(m.active_version(), "0");
        m.switch_version(ModelVersion::N).await.unwrap();
    }

    /// Verify that all four new AFIT port traits compile with a minimal implementation.
    #[tokio::test]
    async fn port_trait_struct_impl() {
        struct AllPorts;

        impl Buffer1Read for AllPorts {
            async fn read_batch(&self, _max: usize) -> Result<Vec<Transaction>, BufferError> {
                Ok(vec![])
            }
        }

        impl Buffer2 for AllPorts {
            async fn write_batch(
                &self,
                _batch: Vec<InferredTransaction>,
            ) -> Result<(), BufferError> {
                Ok(())
            }
        }

        impl Modelizer for AllPorts {
            async fn infer(
                &self,
                batch: Vec<Transaction>,
            ) -> Result<Vec<InferredTransaction>, ModelizerError> {
                Ok(batch
                    .into_iter()
                    .map(|tx| InferredTransaction {
                        predicted_fraud: false,
                        model_name: "test".to_owned(),
                        model_version: "v0".to_owned(),
                        transaction: tx,
                    })
                    .collect())
            }

            async fn switch_version(
                &self,
                _version: ModelVersion,
            ) -> Result<(), ModelizerError> {
                Ok(())
            }
        }

        impl Alarm for AllPorts {
            async fn trigger(
                &self,
                _transaction: &InferredTransaction,
            ) -> Result<(), AlarmError> {
                Ok(())
            }
        }

        let ports = AllPorts;
        let txs = ports.read_batch(1).await.unwrap();
        assert!(txs.is_empty());
        ports.write_batch(vec![]).await.unwrap();
        let inferred = ports.infer(vec![]).await.unwrap();
        assert!(inferred.is_empty());
        ports.switch_version(ModelVersion::N).await.unwrap();
        let tx_for_alarm = InferredTransaction {
            transaction: Transaction {
                id: uuid::Uuid::new_v4(),
                amount: 1.0_f64,
                last_name: "T".to_owned(),
            },
            predicted_fraud: true,
            model_name: "t".to_owned(),
            model_version: "v0".to_owned(),
        };
        ports.trigger(&tx_for_alarm).await.unwrap();
    }
}
