// Rust guideline compliant 2026-02-16

//! In-memory adapter for the `Storage` port.
//!
//! Intended for proof-of-concept runs and unit tests only.
//! Returns `StorageError::CapacityExceeded` when the configured capacity is exceeded.
//! `StorageError::Unavailable` is part of the Storage trait contract but is never
//! returned by this adapter; it is reserved for future concrete backends.

use std::cell::RefCell;

use domain::{PendingTransaction, Storage, StorageError};

/// `Storage` adapter backed by an in-memory `Vec<PendingTransaction>`.
///
/// Pending transactions written via [`Storage::write_batch`] are appended to
/// an internal vector. Returns [`StorageError::CapacityExceeded`] when
/// `current_count + batch_size` exceeds `capacity`.
// #[allow] not #[expect]: dead_code fires in fraud_detection_sqlite binary but
// NOT in fraud_detection binary, so #[expect] would generate an unfulfilled-
// expectation warning in one of the two binaries.
#[allow(dead_code, reason = "used by fraud_detection binary; dead in fraud_detection_sqlite")]
#[derive(Debug)]
pub struct InMemoryStorage {
    inner: RefCell<Vec<PendingTransaction>>,
    /// Maximum number of pending transactions the storage can hold.
    capacity: usize,
}

impl InMemoryStorage {
    /// Create an empty storage with the given `capacity`.
    // See struct-level allow(dead_code) comment above.
    #[allow(dead_code, reason = "used by fraud_detection binary; dead in fraud_detection_sqlite")]
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self { inner: RefCell::new(vec![]), capacity }
    }

    /// Return the number of stored items.
    ///
    /// Used in tests to assert persistence counts.
    #[cfg(test)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }
}

impl Storage for InMemoryStorage {
    /// Append `batch` to the internal store.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::CapacityExceeded` when `current_count + batch.len()`
    /// exceeds the configured capacity.
    async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError> {
        let inner = self.inner.borrow();
        if inner.len() + batch.len() > self.capacity {
            return Err(StorageError::CapacityExceeded { capacity: self.capacity });
        }
        drop(inner);
        self.inner.borrow_mut().extend(batch);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::InMemoryStorage;
    use domain::{
        InferredTransaction, PendingTransaction, Storage as _, StorageError, Transaction,
    };
    use uuid::Uuid;

    fn make_pending() -> PendingTransaction {
        PendingTransaction {
            inferred_transaction: InferredTransaction {
                transaction: Transaction {
                    id: Uuid::new_v4(),
                    amount: 1.00_f64,
                    last_name: "Test".to_owned(),
                },
                predicted_fraud: false,
                model_name: "DEMO".to_owned(),
                model_version: "4".to_owned(),
            },
            is_reviewed: false,
            actual_fraud: None,
        }
    }

    fn make_batch(n: usize) -> Vec<PendingTransaction> {
        (0..n).map(|_| make_pending()).collect()
    }

    // IMS-T01: write_batch stores all items.
    #[tokio::test]
    async fn write_batch_stores_all_items() {
        let storage = InMemoryStorage::new(100);
        storage.write_batch(make_batch(5)).await.unwrap();
        assert_eq!(storage.len(), 5);
    }

    // IMS-T02: CapacityExceeded returned with correct capacity when full.
    #[tokio::test]
    async fn capacity_exceeded_correct_value() {
        let storage = InMemoryStorage::new(3);
        let result = storage.write_batch(make_batch(4)).await;
        assert!(
            matches!(result, Err(StorageError::CapacityExceeded { capacity: 3 })),
            "expected CapacityExceeded(3), got {result:?}"
        );
    }

    // IMS-T03: multiple batches accumulate.
    #[tokio::test]
    async fn multiple_batches_accumulate() {
        let storage = InMemoryStorage::new(100);
        storage.write_batch(make_batch(3)).await.unwrap();
        storage.write_batch(make_batch(4)).await.unwrap();
        assert_eq!(storage.len(), 7);
    }
}
