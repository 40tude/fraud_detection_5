// Rust guideline compliant 2026-02-23

//! In-memory adapter for the `Buffer1` (write) and `Buffer1Read` (read) ports.
//!
//! Intended for proof-of-concept runs and unit tests only. The buffer never
//! signals `Full`; it grows without bound on writes. Reads drain from the
//! front; an empty buffer signals `Closed`.

use std::cell::RefCell;

use domain::{Buffer1, Buffer1Read, BufferError, Transaction};

/// `Buffer1` and `Buffer1Read` adapter backed by an in-memory `Vec`.
///
/// Transactions written via [`Buffer1::write_batch`] are appended to an
/// internal list. Reads via [`Buffer1Read::read_batch`] drain from the front,
/// returning [`BufferError::Closed`] when the list is exhausted.
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

impl Buffer1Read for InMemoryBuffer {
    /// Drain up to `max` transactions from the front of the internal store.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::Closed` when the store is empty.
    async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError> {
        let mut inner = self.inner.borrow_mut();
        if inner.is_empty() {
            return Err(BufferError::Closed);
        }
        let count = max.min(inner.len());
        Ok(inner.drain(..count).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::InMemoryBuffer;
    use domain::{Buffer1 as _, Buffer1Read as _, BufferError, Transaction};

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

        // Read all 5 back via Buffer1Read to verify storage.
        let stored = buffer.read_batch(5).await.unwrap();
        assert_eq!(stored.len(), 5, "all 5 transactions must be stored");
        for (i, tx) in stored.iter().enumerate() {
            assert_eq!(tx.id, ids[i], "UUID at position {i} must match");
        }
    }

    #[tokio::test]
    async fn read_batch_drains_from_front() {
        let buffer = InMemoryBuffer::new();
        let ids: Vec<uuid::Uuid> = (0..4).map(|_| uuid::Uuid::new_v4()).collect();
        let batch: Vec<Transaction> = ids
            .iter()
            .map(|&id| Transaction { id, amount: 1.0_f64, last_name: "T".to_owned() })
            .collect();
        buffer.write_batch(batch).await.unwrap();

        let first = buffer.read_batch(2).await.unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].id, ids[0]);
        assert_eq!(first[1].id, ids[1]);

        let second = buffer.read_batch(10).await.unwrap();
        assert_eq!(second.len(), 2);

        let closed = buffer.read_batch(1).await;
        assert_eq!(closed, Err(BufferError::Closed));
    }
}
