// Rust guideline compliant 2026-02-27

//! Fraud-detection pipeline entry point -- `SQLite` storage demo.
//!
//! Identical to the main `fraud_detection` binary except that storage is
//! backed by a `SQLite` file (`fraud_detection.db` in the current working
//! directory) instead of an in-memory vector. This demonstrates that the
//! hexagonal `Storage` port is truly swappable: only this entry point and
//! the adapter change; all domain and pipeline crates are untouched.
//!
//! # Usage
//!
//! ```text
//! # Infinite mode -- press CTRL+C to stop
//! $env:RUST_LOG='info'; cargo run --bin fraud_detection_sqlite; Remove-Item env:RUST_LOG
//!
//! # Also show per-transaction debug output
//! $env:RUST_LOG='debug'; cargo run --bin fraud_detection_sqlite; Remove-Item env:RUST_LOG
//! ```
//!
//! The file `fraud_detection.db` is created on first run. Inspect rows with
//! any `SQLite` browser (e.g., DB Browser for `SQLite`).

mod adapters;

// Load sqlite_storage directly so it only enters this binary's module tree,
// avoiding dead_code warnings in the `fraud_detection` binary (which uses
// InMemoryStorage instead).
#[path = "adapters/sqlite_storage.rs"]
mod sqlite_storage;

use adapters::concurrent_buffer::ConcurrentBuffer;
use adapters::concurrent_buffer2::ConcurrentBuffer2;
use adapters::demo_model::DemoModel;
use adapters::log_alarm::LogAlarm;
use sqlite_storage::SqliteStorage;
use anyhow::Context as _;
use consumer::{Consumer, ConsumerConfig};
use logger::{Logger, LoggerConfig};
use modelizer::Modelizer;
use producer::{Producer, ProducerConfig};
use std::time::Duration;
use tracing::Instrument as _;

/// Database file created in the current working directory on first run.
///
/// Using the current working directory is acceptable for a demo adapter.
/// A production adapter would read this from configuration or environment.
const DB_URL: &str = "sqlite:fraud_detection.db";

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber before any async work.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // -- Producer: infinite mode by default; press CTRL+C to stop --
    // Set .iterations(10) here for a finite demo run.
    let producer_config = ProducerConfig::builder(100)
        // 500 ms between batches keeps logs readable in real time.
        .poll_interval1(Duration::from_millis(500))
        // .iterations(10)
        .build()
        .context("failed to build producer config")?;

    // ConcurrentBuffer: shared by Producer (write) and Consumer (read).
    let buffer1 = ConcurrentBuffer::new();
    let producer = Producer::new(producer_config);

    // -- Consumer: drain Buffer1 -> Modelizer<DemoModel> -> Buffer2 --
    let consumer_config = ConsumerConfig::builder(50)
        // 25 ms ensures Consumer yields regularly so Producer gets CPU time.
        .poll_interval2(Duration::from_millis(25))
        .build()
        .context("failed to build consumer config")?;

    // ConcurrentBuffer2: shared by Consumer (write) and Logger (read).
    let buffer2 = ConcurrentBuffer2::new();
    // DEMO model: OS-seeded RNG, starts at version N (version 4, ~4% fraud rate).
    let model = DemoModel::new(None);
    let modelizer = Modelizer::new(model);
    let alarm = LogAlarm::new();
    let consumer = Consumer::new(consumer_config);

    // -- Logger: drain Buffer2 -> SqliteStorage --
    let logger_config = LoggerConfig::builder(10)
        // 25 ms matches Consumer cadence.
        .poll_interval3(Duration::from_millis(25))
        .build()
        .context("failed to build logger config")?;

    // SqliteStorage: opens or creates fraud_detection.db in the working directory.
    // INSERT OR REPLACE: duplicate UUIDs are silently overwritten (demo adapter).
    let storage = SqliteStorage::new(DB_URL)
        .await
        .context("failed to open SQLite storage")?;
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
            }
            .instrument(tracing::info_span!("producer")),
            consumer_then_close.instrument(tracing::info_span!("consumer")),
            logger
                .run(&buffer2, &storage)
                .instrument(tracing::info_span!("logger"))
        );
        p.context("producer failed")
            .and(c.context("consumer failed"))
            .and(l.context("logger failed"))
    };

    // Race the pipeline against CTRL+C.
    // CTRL+C: close buffer1 only; buffer2 cascade follows from consumer_then_close.
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("main.shutdown: ctrl_c received, closing buffers");
            buffer1.close();
        }
        result = pipeline => {
            result?;
        }
    }

    Ok(())
}
