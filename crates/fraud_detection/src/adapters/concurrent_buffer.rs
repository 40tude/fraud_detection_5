// Rust guideline compliant 2026-02-23

//! Concurrent-capable adapter for the `Buffer1` and `Buffer1Read` ports.
//!
//! Unlike `InMemoryBuffer`, an empty buffer cooperatively yields rather than
//! signaling `Closed`. Explicit `close()` signals end-of-data to readers.
//! Designed for `tokio::join!` on a `current_thread` runtime.

use std::cell::RefCell;

use domain::{Buffer1, Buffer1Read, BufferError, Transaction};

// ---------------------------------------------------------------------------
// Inner state
// ---------------------------------------------------------------------------

/// Heap storage for buffered transactions and the close flag.
#[derive(Debug)]
struct ConcurrentBufferInner {
    data: Vec<Transaction>,
    closed: bool,
}

// ---------------------------------------------------------------------------
// ConcurrentBuffer
// ---------------------------------------------------------------------------

/// `Buffer1` and `Buffer1Read` adapter that yields on empty instead of signaling Closed.
///
/// Shares a single `RefCell` across both trait impls. Safe on `current_thread`
/// runtimes because `RefCell` borrows are always dropped before any `.await`
/// point inside `read_batch`, preventing re-entrant borrow panics.
#[derive(Debug)]
pub struct ConcurrentBuffer {
    inner: RefCell<ConcurrentBufferInner>,
}

impl ConcurrentBuffer {
    /// Create an empty, open buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(ConcurrentBufferInner { data: vec![], closed: false }),
        }
    }

    /// Signal end-of-data. Idempotent: safe to call multiple times.
    pub fn close(&self) {
        self.inner.borrow_mut().closed = true;
    }
}

impl Default for ConcurrentBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer1 for ConcurrentBuffer {
    /// Append `batch` to the buffer if open.
    ///
    /// # Errors
    ///
    /// Returns [`BufferError::Closed`] if the buffer has been closed.
    async fn write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(BufferError::Closed);
        }
        inner.data.extend(batch);
        Ok(())
    }
}

impl Buffer1Read for ConcurrentBuffer {
    /// Drain up to `max` transactions from the front; yield and retry if empty and open.
    ///
    /// Loops via `tokio::task::yield_now` while the buffer is open but empty,
    /// allowing other futures in a `tokio::join!` to make progress. The
    /// `RefCell` borrow is always released before the yield point.
    ///
    /// # Errors
    ///
    /// Returns [`BufferError::Closed`] when the buffer is empty and closed.
    async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError> {
        loop {
            // Scope the borrow so it is dropped before yield_now().await,
            // preventing a panic on re-entrant polling within tokio::join!.
            let result = {
                let mut inner = self.inner.borrow_mut();
                if !inner.data.is_empty() {
                    let count = max.min(inner.data.len());
                    Some(Ok(inner.data.drain(..count).collect()))
                } else if inner.closed {
                    Some(Err(BufferError::Closed))
                } else {
                    None
                }
            }; // borrow dropped here

            match result {
                Some(r) => return r,
                None => tokio::task::yield_now().await,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::ConcurrentBuffer;
    use domain::{Buffer1 as _, Buffer1Read as _, BufferError, Transaction};
    use uuid::Uuid;

    fn make_tx() -> Transaction {
        Transaction { id: Uuid::new_v4(), amount: 1.00_f64, last_name: "Test".to_owned() }
    }

    fn make_txs(n: usize) -> Vec<Transaction> {
        (0..n).map(|_| make_tx()).collect()
    }

    // CB-T01: write/read roundtrip preserves all transactions.
    #[tokio::test]
    async fn write_read_roundtrip() {
        let buffer = ConcurrentBuffer::new();
        let txs = make_txs(3);
        let ids: Vec<_> = txs.iter().map(|t| t.id).collect();

        buffer.write_batch(txs).await.unwrap();
        buffer.close();

        let read = buffer.read_batch(10).await.unwrap();
        assert_eq!(read.len(), 3);
        for (i, tx) in read.iter().enumerate() {
            assert_eq!(tx.id, ids[i]);
        }
    }

    // CB-T02: empty buffer after close returns Err(Closed).
    #[tokio::test]
    async fn empty_closed_returns_err_closed() {
        let buffer = ConcurrentBuffer::new();
        buffer.close();

        let result = buffer.read_batch(1).await;
        assert_eq!(result, Err(BufferError::Closed));
    }

    // CB-T03: writing to a closed buffer returns Err(Closed).
    #[tokio::test]
    async fn write_to_closed_returns_err_closed() {
        let buffer = ConcurrentBuffer::new();
        buffer.close();

        let result = buffer.write_batch(make_txs(1)).await;
        assert_eq!(result, Err(BufferError::Closed));
    }

    // CB-T04: successive reads drain from the front (FIFO order).
    #[tokio::test]
    async fn drain_from_front() {
        let buffer = ConcurrentBuffer::new();
        let txs = make_txs(4);
        let ids: Vec<_> = txs.iter().map(|t| t.id).collect();

        buffer.write_batch(txs).await.unwrap();
        buffer.close();

        let first = buffer.read_batch(2).await.unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].id, ids[0]);
        assert_eq!(first[1].id, ids[1]);

        let second = buffer.read_batch(10).await.unwrap();
        assert_eq!(second.len(), 2);
        assert_eq!(second[0].id, ids[2]);
        assert_eq!(second[1].id, ids[3]);
    }

    // CB-T05: close() is idempotent; double close must not panic.
    #[tokio::test]
    async fn idempotent_close() {
        let buffer = ConcurrentBuffer::new();
        buffer.close();
        buffer.close(); // must not panic

        let result = buffer.read_batch(1).await;
        assert_eq!(result, Err(BufferError::Closed));
    }

    // CB-T06: read_batch yields on empty+open; a concurrent write unblocks it.
    //
    // tokio::join! polls reader first (empty -> yield_now), then polls writer
    // (writes one tx, completes). Yield waker fires, join! re-polls reader
    // which now finds data and returns Ok.
    #[tokio::test]
    async fn yield_unblocks_read() {
        let buffer = ConcurrentBuffer::new();

        let (read_result, _) = tokio::join!(
            buffer.read_batch(1),
            async { buffer.write_batch(vec![make_tx()]).await.unwrap(); }
        );

        assert_eq!(read_result.unwrap().len(), 1);
    }
}
