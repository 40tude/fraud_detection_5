// Rust guideline compliant 2026-02-16

//! Shared domain types for the fraud-detection pipeline.
//!
//! Defines `Transaction`, `BufferError`, and the `Buffer1` hexagonal port trait.
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

/// Errors that a `Buffer1` implementation may return.
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

#[cfg(test)]
mod tests {
    use super::{Buffer1, BufferError, Transaction};
    use std::cell::RefCell;

    #[test]
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
}
