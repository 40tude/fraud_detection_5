// Rust guideline compliant 2026-02-23

//! In-memory adapter for the `Buffer2` port.
//!
//! Intended for proof-of-concept runs and unit tests only. Returns
//! `BufferError::Full` when the configured capacity is exceeded.

use std::cell::RefCell;
use std::collections::VecDeque;

use domain::{Buffer2, BufferError, InferredTransaction};

/// `Buffer2` adapter backed by an in-memory `VecDeque`.
///
/// Inferred transactions written via [`Buffer2::write_batch`] are appended to
/// an internal deque. Returns [`BufferError::Full`] when
/// `current_count + batch_size` exceeds `capacity`.
#[derive(Debug)]
pub struct InMemoryBuffer2 {
    inner: RefCell<VecDeque<InferredTransaction>>,
    /// Maximum number of inferred transactions the buffer can hold.
    capacity: usize,
}

impl InMemoryBuffer2 {
    /// Create an empty buffer with the given `capacity`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self { inner: RefCell::new(VecDeque::new()), capacity }
    }
}

impl Buffer2 for InMemoryBuffer2 {
    /// Append `batch` to the internal store.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::Full` when `current_count + batch.len()` exceeds
    /// the configured capacity.
    async fn write_batch(&self, batch: Vec<InferredTransaction>) -> Result<(), BufferError> {
        let mut inner = self.inner.borrow_mut();
        if inner.len() + batch.len() > self.capacity {
            return Err(BufferError::Full { capacity: self.capacity });
        }
        inner.extend(batch);
        Ok(())
    }
}
