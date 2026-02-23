// Rust guideline compliant 2026-02-23

//! Fraud-detection pipeline entry point.
//!
//! Wires all pipeline components (Producer, Consumer, Modelizer) to their
//! concurrent-buffer and DEMO adapters and runs a proof-of-concept concurrent
//! end-to-end pipeline.
//!
//! # Usage
//!
//! ```text
//! # Infinite mode -- press CTRL+C to stop
//! $env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG
//!
//! # Also show per-transaction debug output
//! $env:RUST_LOG='debug'; cargo run; Remove-Item env:RUST_LOG
//! ```

mod adapters;

use adapters::concurrent_buffer::ConcurrentBuffer;
use adapters::demo_model::DemoModel;
use adapters::in_memory_buffer2::InMemoryBuffer2;
use adapters::log_alarm::LogAlarm;
use anyhow::Context as _;
use consumer::{Consumer, ConsumerConfig};
use modelizer::Modelizer;
use producer::{Producer, ProducerConfig};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Initialize the log facade before any async work.
    env_logger::init();

    // -- Producer: infinite mode by default; press CTRL+C to stop --
    // Set .iterations(10) here for a finite demo run.
    let producer_config = ProducerConfig::builder(100)
        // 50 ms between batches keeps logs readable in real time.
        .speed1(Duration::from_millis(500))
        // .iterations(10)
        .build()
        .context("failed to build producer config")?;

    // ConcurrentBuffer: shared by Producer (write) and Consumer (read).
    let buffer1 = ConcurrentBuffer::new();
    let producer = Producer::new(producer_config);

    // -- Consumer: drain Buffer1 -> Modelizer<DemoModel> -> Buffer2 --
    let consumer_config = ConsumerConfig::builder(50)
        // 25 ms ensures Consumer yields regularly so Producer gets CPU time.
        .speed2(Duration::from_millis(25))
        .build()
        .context("failed to build consumer config")?;

    // Generous capacity: enough for continuous concurrent operation.
    let buffer2 = InMemoryBuffer2::new(2_000);
    // DEMO model: OS-seeded RNG, starts at version N (version 4, ~4% fraud rate).
    let model = DemoModel::new(None);
    let modelizer = Modelizer::new(model);
    let alarm = LogAlarm::new();
    let consumer = Consumer::new(consumer_config);

    // Wrap the concurrent pair in one async block (FR-001, FR-002, Decision 3).
    // Finite-mode: producer.run completes -> close() -> consumer drains -> join resolves.
    let pipeline = async {
        // tokio::join! polls both futures concurrently and returns the tuple directly.
        let (p, c) = tokio::join!(
            async {
                let r = producer.run(&buffer1).await;
                // Close buffer so Consumer exits cleanly after draining (FR-005).
                buffer1.close();
                r
            },
            consumer.run(&buffer1, &modelizer, &alarm, &buffer2)
        );
        p.context("producer failed")
            .and(c.context("consumer failed"))
    };

    // Race the pipeline against CTRL+C (FR-004, Decision 4).
    // CTRL+C: close buffer, cancel both tasks, exit cleanly (SC-002).
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("main.shutdown: ctrl_c received, closing buffer");
            buffer1.close();
        }
        result = pipeline => {
            result?;
        }
    }

    Ok(())
}
