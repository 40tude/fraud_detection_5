// Rust guideline compliant 2026-02-16

//! Fraud-detection pipeline entry point.
//!
//! Wires the `Producer` domain component to the `InMemoryBuffer` adapter and
//! runs the production loop.
//!
//! # Usage
//!
//! ```text
//! # Show iteration-level log lines (one per batch)
//! RUST_LOG=info cargo run
//!
//! # Also show per-transaction debug output
//! RUST_LOG=debug cargo run
//! ```

mod adapters;

use adapters::in_memory_buffer::InMemoryBuffer;
use anyhow::Context as _;
use producer::{Producer, ProducerConfig};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Initialize the log facade before any async work.
    env_logger::init();

    let config = ProducerConfig::builder(100)
        // 10 batches is enough to demonstrate the pipeline without running forever.
        .iterations(10)
        // 50 ms between batches so logs are readable in real time.
        .speed1(Duration::from_millis(50))
        .build()
        .context("failed to build producer config")?;

    let buffer = InMemoryBuffer::new();
    let producer = Producer::new(config);

    producer
        .run(&buffer)
        .await
        .context("producer run failed")?;

    log::info!(
        "producer.run.complete: total_transactions={}",
        buffer.transactions().len()
    );

    Ok(())
}
