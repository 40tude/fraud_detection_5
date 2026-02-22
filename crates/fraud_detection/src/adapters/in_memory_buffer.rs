// Rust guideline compliant 2026-02-16

//! In-memory adapter for the `Buffer1` port.
//!
//! Intended for proof-of-concept runs and unit tests only. The buffer never
//! signals `Full` or `Closed`; it grows without bound.

use std::cell::{Ref, RefCell};

use domain::{Buffer1, BufferError, Transaction};

/// `Buffer1` adapter backed by an in-memory `Vec`.
///
/// Transactions written via [`Buffer1::write_batch`] are appended to an
/// internal list and can be inspected through [`transactions`](Self::transactions).
#[derive(Debug)]
pub struct InMemoryBuffer {
    inner: RefCell<Vec<Transaction>>,
}

impl InMemoryBuffer {
    /// Create an empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self { inner: RefCell::new(vec![]) }
    }

    /// Borrow the full list of stored transactions.
    #[must_use]
    pub fn transactions(&self) -> Ref<'_, Vec<Transaction>> {
        self.inner.borrow()
    }
}

impl Default for InMemoryBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer1 for InMemoryBuffer {
    /// Append `batch` to the internal store. Never fails for proof-of-concept usage.
    async fn write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError> {
        self.inner.borrow_mut().extend(batch);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::InMemoryBuffer;
    use domain::{Buffer1 as _, Transaction};

    #[tokio::test]
    async fn in_memory_buffer_stores_batch() {
        let buffer = InMemoryBuffer::new();
        let ids: Vec<uuid::Uuid> = (0..5).map(|_| uuid::Uuid::new_v4()).collect();
        let batch: Vec<Transaction> = ids
            .iter()
            .map(|&id| Transaction {
                id,
                amount: 1.00_f64,
                last_name: "Test".to_owned(),
            })
            .collect();

        buffer.write_batch(batch).await.unwrap();

        let stored = buffer.transactions();
        assert_eq!(stored.len(), 5, "all 5 transactions must be stored");
        for (i, tx) in stored.iter().enumerate() {
            assert_eq!(tx.id, ids[i], "UUID at position {i} must match");
        }
    }
}
