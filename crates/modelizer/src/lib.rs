// Rust guideline compliant 2026-02-23

//! Generic Modelizer component for the fraud-detection pipeline.
//!
//! [`Modelizer`] implements the `domain::Modelizer` port by delegating
//! per-transaction classification to an injected `domain::Model` adapter.
//! It owns no concrete model logic -- all fraud detection is in the adapter.

use domain::{InferredTransaction, Model, ModelVersion, ModelizerError, Transaction};

// ---------------------------------------------------------------------------
// Modelizer
// ---------------------------------------------------------------------------

/// Pipeline component that implements the `domain::Modelizer` port.
///
/// Generic over any `Model` adapter; carries no model-specific logic.
/// Version switching and fraud classification are fully delegated to the adapter.
#[derive(Debug)]
pub struct Modelizer<M: Model> {
    model: M,
}

impl<M: Model> Modelizer<M> {
    /// Create a new Modelizer wrapping `model`.
    #[must_use]
    pub fn new(model: M) -> Self {
        Self { model }
    }
}

impl<M: Model> domain::Modelizer for Modelizer<M> {
    /// Classify all transactions in `batch` and return one `InferredTransaction` per input.
    ///
    /// Reads `model.name()` and `model.active_version()` once before iterating
    /// so version stays stable within a single call (FR-009).
    ///
    /// # Errors
    ///
    /// Returns `ModelizerError::InferenceFailed` if any `classify` call fails.
    async fn infer(
        &self,
        batch: Vec<Transaction>,
    ) -> Result<Vec<InferredTransaction>, ModelizerError> {
        log::debug!("modelizer.infer: batch_size={}", batch.len());
        // Read metadata once -- version is stable for the duration of this call.
        let model_name = self.model.name().to_owned();
        let model_version = self.model.active_version().to_owned();

        let mut results = Vec::with_capacity(batch.len());
        for tx in batch {
            let predicted_fraud = self.model.classify(&tx).await?;
            results.push(InferredTransaction {
                transaction: tx,
                predicted_fraud,
                model_name: model_name.clone(),
                model_version: model_version.clone(),
            });
        }
        Ok(results)
    }

    /// Switch the active model version; delegates entirely to the `Model` adapter.
    ///
    /// # Errors
    ///
    /// Returns `ModelizerError::SwitchFailed` if the adapter rejects the switch.
    async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError> {
        log::info!("modelizer.switch_version: version={version:?}");
        self.model.switch_version(version).await
    }
}

#[cfg(test)]
mod tests {
    use domain::{
        InferredTransaction, Model, ModelizerError, ModelVersion, Transaction,
    };
    use std::cell::Cell;

    // ------------------------------------------------------------------
    // MockModel helper
    // ------------------------------------------------------------------

    struct MockModel {
        predicted_fraud: bool,
        switch_call: Cell<Option<ModelVersion>>,
    }

    impl MockModel {
        fn new(predicted_fraud: bool) -> Self {
            Self { predicted_fraud, switch_call: Cell::new(None) }
        }
    }

    impl Model for MockModel {
        async fn classify(&self, _tx: &Transaction) -> Result<bool, ModelizerError> {
            Ok(self.predicted_fraud)
        }

        fn name(&self) -> &str {
            "MOCK"
        }

        fn active_version(&self) -> &str {
            "v0"
        }

        async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError> {
            self.switch_call.set(Some(version));
            Ok(())
        }
    }

    fn make_tx() -> Transaction {
        Transaction {
            id: uuid::Uuid::new_v4(),
            amount: 1.00_f64,
            last_name: "Test".to_owned(),
        }
    }

    // ------------------------------------------------------------------
    // T007: empty batch
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn empty_batch_returns_empty() {
        let model = MockModel::new(false);
        let modelizer = super::Modelizer::new(model);
        let result = domain::Modelizer::infer(&modelizer, vec![]).await.unwrap();
        assert!(result.is_empty());
    }

    // ------------------------------------------------------------------
    // T008: count and order
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn batch_inference_returns_same_count_in_order() {
        let txs: Vec<Transaction> = (0..5).map(|_| make_tx()).collect();
        let ids: Vec<uuid::Uuid> = txs.iter().map(|t| t.id).collect();

        let model = MockModel::new(false);
        let modelizer = super::Modelizer::new(model);
        let result = domain::Modelizer::infer(&modelizer, txs).await.unwrap();

        assert_eq!(result.len(), 5);
        for (i, inferred) in result.iter().enumerate() {
            assert_eq!(inferred.transaction.id, ids[i], "order mismatch at index {i}");
        }
    }

    // ------------------------------------------------------------------
    // T009: enrichment fields
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn inferred_tx_carries_enrichment_fields() {
        let tx = make_tx();
        let model = MockModel::new(true);
        let modelizer = super::Modelizer::new(model);
        let result = domain::Modelizer::infer(&modelizer, vec![tx]).await.unwrap();

        assert_eq!(result.len(), 1);
        let inferred: &InferredTransaction = &result[0];
        assert!(inferred.predicted_fraud);
        assert_eq!(inferred.model_name, "MOCK");
        assert_eq!(inferred.model_version, "v0");
    }

    // ------------------------------------------------------------------
    // T021: switch_version delegates to model
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn modelizer_switch_delegates_to_model() {
        let model = MockModel::new(false);
        let modelizer = super::Modelizer::new(model);
        domain::Modelizer::switch_version(&modelizer, ModelVersion::NMinus1).await.unwrap();
        assert_eq!(
            modelizer.model.switch_call.get(),
            Some(ModelVersion::NMinus1),
            "switch_version must be forwarded to the model"
        );
    }
}
