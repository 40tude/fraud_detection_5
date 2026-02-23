// Rust guideline compliant 2026-02-23

//! DEMO model adapter for the `Model` port.
//!
//! Named `"DEMO"`, with two versions: 4 (N, latest) and 3 (N-1, previous).
//! Classifies transactions probabilistically: 4% fraud rate for version 4,
//! 3% for version 3. Supports seeded randomness for reproducible tests.

use std::cell::RefCell;

use domain::{Model, ModelizerError, ModelVersion, Transaction};
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Concrete adapter for the `domain::Model` port.
///
/// Maps `ModelVersion::N` to version `"4"` and `ModelVersion::NMinus1` to `"3"`.
/// Starts at `ModelVersion::N` per FR-007. Fraud detection is probabilistic:
/// `"4"` detects ~4% fraud, `"3"` detects ~3% (FR-005, FR-006).
#[derive(Debug)]
pub struct DemoModel {
    /// Currently active version; interior mutability required (trait takes `&self`).
    current_version: RefCell<ModelVersion>,
    /// RNG for probabilistic fraud classification; seeded for reproducibility (FR-011).
    rng: RefCell<StdRng>,
}

impl DemoModel {
    /// Create a new DEMO model.
    ///
    /// `seed = Some(s)` produces deterministic results; `None` seeds from the OS.
    /// Starts with `ModelVersion::N` (version 4) per FR-007.
    #[must_use]
    pub fn new(seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_os_rng(),
        };
        Self {
            // FR-007: default to version N at startup.
            current_version: RefCell::new(ModelVersion::N),
            rng: RefCell::new(rng),
        }
    }

    /// Fraud probability for the currently active version.
    fn fraud_rate(&self) -> f64 {
        match *self.current_version.borrow() {
            ModelVersion::N => 0.04,       // FR-006: version 4 detects ~4%
            ModelVersion::NMinus1 => 0.03, // FR-005: version 3 detects ~3%
        }
    }
}

impl Model for DemoModel {
    /// Classify a transaction probabilistically using the active fraud rate (FR-010).
    ///
    /// # Errors
    ///
    /// Currently infallible; returns `Ok(bool)`.
    async fn classify(&self, _tx: &Transaction) -> Result<bool, ModelizerError> {
        let rate = self.fraud_rate();
        let roll: f64 = self.rng.borrow_mut().random();
        let is_fraud = roll < rate;
        log::debug!("demo_model.classify: fraud={is_fraud} rate={rate}");
        Ok(is_fraud)
    }

    /// Returns `"DEMO"` (FR-003).
    fn name(&self) -> &str {
        "DEMO"
    }

    /// Returns `"4"` for `ModelVersion::N` and `"3"` for `ModelVersion::NMinus1` (FR-004, FR-015).
    fn active_version(&self) -> &str {
        match *self.current_version.borrow() {
            ModelVersion::N => "4",
            ModelVersion::NMinus1 => "3",
        }
    }

    /// Switch the active model version (FR-008, FR-009).
    ///
    /// Takes effect on the next `classify` call.
    ///
    /// # Errors
    ///
    /// Currently infallible; returns `Ok(())`.
    async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError> {
        log::info!("demo_model.switch_version: version={version:?}");
        *self.current_version.borrow_mut() = version;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // T014: name
    // ------------------------------------------------------------------

    #[test]
    fn demo_model_name_is_demo() {
        let m = DemoModel::new(None);
        assert_eq!(m.name(), "DEMO");
    }

    // ------------------------------------------------------------------
    // T015: default active version
    // ------------------------------------------------------------------

    #[test]
    fn demo_model_default_active_version_is_4() {
        let m = DemoModel::new(None);
        assert_eq!(m.active_version(), "4");
    }

    // ------------------------------------------------------------------
    // T019: switch to NMinus1
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn switch_to_nminus1_active_version_is_3() {
        let m = DemoModel::new(None);
        m.switch_version(ModelVersion::NMinus1).await.unwrap();
        assert_eq!(m.active_version(), "3");
    }

    // ------------------------------------------------------------------
    // T020: switch back to N
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn switch_to_n_active_version_is_4() {
        let m = DemoModel::new(None);
        m.switch_version(ModelVersion::NMinus1).await.unwrap();
        m.switch_version(ModelVersion::N).await.unwrap();
        assert_eq!(m.active_version(), "4");
    }

    // ------------------------------------------------------------------
    // T025: determinism with seed
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn classify_seeded_is_deterministic() {
        let tx = Transaction { id: uuid::Uuid::new_v4(), amount: 1.0_f64, last_name: "A".to_owned() };
        let m1 = DemoModel::new(Some(42));
        let m2 = DemoModel::new(Some(42));
        let results1: Vec<bool> = {
            let mut v = Vec::new();
            for _ in 0..100 {
                v.push(m1.classify(&tx).await.unwrap());
            }
            v
        };
        let results2: Vec<bool> = {
            let mut v = Vec::new();
            for _ in 0..100 {
                v.push(m2.classify(&tx).await.unwrap());
            }
            v
        };
        assert_eq!(results1, results2, "identical seeds must produce identical sequences");
    }

    // ------------------------------------------------------------------
    // T026: fraud rate ~4% for version N
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn fraud_rate_v4_is_approx_4pct() {
        let tx = Transaction { id: uuid::Uuid::new_v4(), amount: 1.0_f64, last_name: "B".to_owned() };
        let m = DemoModel::new(Some(0));
        let count = 10_000usize;
        let mut fraud = 0usize;
        for _ in 0..count {
            if m.classify(&tx).await.unwrap() {
                fraud += 1;
            }
        }
        let rate = (fraud as f64) / (count as f64) * 100.0_f64;
        assert!(
            (3.0_f64..=5.0_f64).contains(&rate),
            "v4 fraud rate {rate:.2}% not in [3%, 5%]"
        );
    }

    // ------------------------------------------------------------------
    // T027: fraud rate ~3% for version NMinus1
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn fraud_rate_v3_is_approx_3pct() {
        let tx = Transaction { id: uuid::Uuid::new_v4(), amount: 1.0_f64, last_name: "C".to_owned() };
        let m = DemoModel::new(Some(0));
        m.switch_version(ModelVersion::NMinus1).await.unwrap();
        let count = 10_000usize;
        let mut fraud = 0usize;
        for _ in 0..count {
            if m.classify(&tx).await.unwrap() {
                fraud += 1;
            }
        }
        let rate = (fraud as f64) / (count as f64) * 100.0_f64;
        assert!(
            (2.0_f64..=4.0_f64).contains(&rate),
            "v3 fraud rate {rate:.2}% not in [2%, 4%]"
        );
    }
}
