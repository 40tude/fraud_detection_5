// Rust guideline compliant 2026-02-23

//! Fraud-detection pipeline entry point.
//!
//! Wires all pipeline components (Producer, Consumer) to their in-memory and
//! demo adapters and runs a proof-of-concept end-to-end pipeline.
//!
//! # Usage
//!
//! ```text
//! # Show iteration-level log lines
//! $env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG
//!
//! # Also show per-transaction debug output
//! $env:RUST_LOG='debug'; cargo run; Remove-Item env:RUST_LOG
//! ```

mod adapters;

use adapters::demo_modelizer::DemoModelizer;
use adapters::in_memory_buffer::InMemoryBuffer;
use adapters::in_memory_buffer2::InMemoryBuffer2;
use adapters::log_alarm::LogAlarm;
use anyhow::Context as _;
use consumer::{Consumer, ConsumerConfig};
use producer::{Producer, ProducerConfig};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Initialize the log facade before any async work.
    env_logger::init();

    // -- Producer: write 10 batches of up to 100 transactions into Buffer1 --
    let producer_config = ProducerConfig::builder(100)
        // 10 batches is enough to demonstrate the pipeline without running forever.
        .iterations(10)
        // 50 ms between batches so logs are readable in real time.
        .speed1(Duration::from_millis(50))
        .build()
        .context("failed to build producer config")?;

    let buffer1 = InMemoryBuffer::new();
    let producer = Producer::new(producer_config);

    producer
        .run(&buffer1)
        .await
        .context("producer run failed")?;

    log::info!("producer.run.complete");

    // -- Consumer: drain Buffer1 -> Modelizer -> Buffer2 --
    let consumer_config = ConsumerConfig::builder(50)
        // No delay in demo: drain the pre-filled Buffer1 as fast as possible.
        .speed2(Duration::ZERO)
        .build()
        .context("failed to build consumer config")?;

    // Generous capacity: 10 batches * 100 max tx = up to 1000 transactions.
    let buffer2 = InMemoryBuffer2::new(2_000);
    // Demo modelizer: all transactions classified as legitimate (no fraud alarms).
    let modelizer = DemoModelizer::new(false);
    let alarm = LogAlarm::new();
    let consumer = Consumer::new(consumer_config);

    consumer
        .run(&buffer1, &modelizer, &alarm, &buffer2)
        .await
        .context("consumer run failed")?;

    log::info!("consumer.run.complete");

    Ok(())
}
