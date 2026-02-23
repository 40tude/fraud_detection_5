// Rust guideline compliant 2026-02-23

//! Demo adapter for the `Modelizer` port.
//!
//! Classifies all transactions as fraudulent or legitimate based on a config
//! flag. Intended for proof-of-concept runs only.

use std::cell::RefCell;

use domain::{InferredTransaction, Modelizer, ModelizerError, ModelVersion, Transaction};

/// `Modelizer` adapter for demonstration purposes.
///
/// Marks every transaction as fraudulent when `always_fraud = true`, or
/// legitimate when `false`. Populates `model_name = "DINN"` and
/// `model_version = "v1"` (N) / `"v0"` (`NMinus1`). Starts with `ModelVersion::N`.
#[derive(Debug)]
pub struct DemoModelizer {
    /// When `true`, every transaction is classified as fraudulent.
    always_fraud: bool,
    /// Current model version; updated via `switch_version`. Uses interior
    /// mutability because trait methods take `&self`.
    current_version: RefCell<ModelVersion>,
}

impl DemoModelizer {
    /// Create a new demo modelizer.
    ///
    /// `always_fraud = true` classifies every transaction as fraudulent;
    /// `false` classifies all as legitimate. Starts with `ModelVersion::N`
    /// per FR-009.
    #[must_use]
    pub fn new(always_fraud: bool) -> Self {
        Self {
            always_fraud,
            // FR-009: Modelizer defaults to version N at startup.
            current_version: RefCell::new(ModelVersion::N),
        }
    }
}

impl Modelizer for DemoModelizer {
    async fn infer(
        &self,
        batch: Vec<Transaction>,
    ) -> Result<Vec<InferredTransaction>, ModelizerError> {
        log::debug!("demo_modelizer.infer: batch_size={}", batch.len());
        let version = *self.current_version.borrow();
        // v1 = latest model (N), v0 = previous model (NMinus1).
        let model_version_str = match version {
            ModelVersion::N => "v1",
            ModelVersion::NMinus1 => "v0",
        };
        Ok(batch
            .into_iter()
            .map(|tx| InferredTransaction {
                predicted_fraud: self.always_fraud,
                model_name: "DINN".to_owned(),
                model_version: model_version_str.to_owned(),
                transaction: tx,
            })
            .collect())
    }

    async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError> {
        log::info!("demo_modelizer.switch_version: version={version:?}");
        *self.current_version.borrow_mut() = version;
        Ok(())
    }
}
