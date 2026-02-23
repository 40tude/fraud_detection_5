// Rust guideline compliant 2026-02-23

//! Fraud-detection pipeline entry point.
//!
//! Wires all pipeline components (Producer, Consumer, Modelizer, Logger) to their
//! concurrent-buffer, storage, and DEMO adapters and runs a proof-of-concept
//! concurrent end-to-end pipeline.
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
use adapters::concurrent_buffer2::ConcurrentBuffer2;
use adapters::demo_model::DemoModel;
use adapters::in_memory_storage::InMemoryStorage;
use adapters::log_alarm::LogAlarm;
use anyhow::Context as _;
use consumer::{Consumer, ConsumerConfig};
use logger::{Logger, LoggerConfig};
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
        // 500 ms between batches keeps logs readable in real time.
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

    // ConcurrentBuffer2: shared by Consumer (write) and Logger (read).
    let buffer2 = ConcurrentBuffer2::new();
    // DEMO model: OS-seeded RNG, starts at version N (version 4, ~4% fraud rate).
    let model = DemoModel::new(None);
    let modelizer = Modelizer::new(model);
    let alarm = LogAlarm::new();
    let consumer = Consumer::new(consumer_config);

    // -- Logger: drain Buffer2 -> InMemoryStorage --
    let logger_config = LoggerConfig::builder(10)
        // 25 ms matches Consumer cadence.
        .speed3(Duration::from_millis(25))
        .build()
        .context("failed to build logger config")?;

    // usize::MAX capacity: effectively unbounded for proof-of-concept.
    let storage = InMemoryStorage::new(usize::MAX);
    let logger = Logger::new(logger_config);

    // Shutdown cascade: Consumer.run completes -> buffer2.close() -> Logger drains+stops.
    // On CTRL+C, only buffer1.close() is needed; buffer2 cascade follows automatically.
    let consumer_then_close = async {
        let r = consumer.run(&buffer1, &modelizer, &alarm, &buffer2).await;
        // Close buffer2 so Logger exits cleanly after draining (cascade shutdown).
        buffer2.close();
        r
    };

    let pipeline = async {
        // tokio::join! polls all three futures concurrently and returns the tuple directly.
        let (p, c, l) = tokio::join!(
            async {
                let r = producer.run(&buffer1).await;
                // Close buffer1 so Consumer exits cleanly after draining.
                buffer1.close();
                r
            },
            consumer_then_close,
            logger.run(&buffer2, &storage)
        );
        p.context("producer failed")
            .and(c.context("consumer failed"))
            .and(l.context("logger failed"))
    };

    // Race the pipeline against CTRL+C.
    // CTRL+C: close buffer1 only; buffer2 cascade follows from consumer_then_close.
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("main.shutdown: ctrl_c received, closing buffers");
            buffer1.close();
        }
        result = pipeline => {
            result?;
        }
    }

    Ok(())
}
