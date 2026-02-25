// Rust guideline compliant 2026-02-16

//! Pipeline throughput benchmark entry point.
//!
//! Measures end-to-end pipeline throughput (transactions processed per second)
//! across a range of batch sizes.  Each batch size is run `ROUNDS` times;
//! min/avg/max throughput is printed to stdout.
//!
//! # Measurement scope
//!
//! **Storage write cost is excluded from all measurements.**
//! The pipeline runs with [`BenchStorage`] (a discard adapter) and
//! [`BenchModel`] (always returns `Ok(false)`, no RNG).  What is measured:
//!
//! - Producer: UUID generation, amount sampling, batch assembly
//! - Consumer: buffer read, Modelizer call, buffer write
//! - Logger: buffer read, `PendingTransaction` construction, storage call
//! - Both `ConcurrentBuffer` instances: interior-mutability yield loops
//!
//! What is **not** measured: any real I/O, storage allocation, alarm delivery.
//!
//! No `env_logger::init()`: log macros compile to no-ops, eliminating log I/O
//! overhead from measurements.
//!
//! # Usage
//!
//! ```text
//! # Quick sanity check (debug build)
//! cargo build --bin fraud_detection_bench
//!
//! # Accurate throughput numbers (release build)
//! cargo run --bin fraud_detection_bench --release
//! ```

mod adapters;

// Load bench-only adapters into this binary's module tree only.
// Same #[path] technique as main_sqlite.rs / sqlite_storage:
// avoids dead_code warnings in the other binaries.
#[path = "adapters/bench_model.rs"]
mod bench_model;
#[path = "adapters/bench_storage.rs"]
mod bench_storage;

use std::time::Instant;

use adapters::concurrent_buffer::ConcurrentBuffer;
use adapters::concurrent_buffer2::ConcurrentBuffer2;
use adapters::log_alarm::LogAlarm;
use bench_model::BenchModel;
use bench_storage::BenchStorage;
use consumer::{Consumer, ConsumerConfig};
use logger::{Logger, LoggerConfig};
use modelizer::Modelizer;
use producer::{Producer, ProducerConfig};

// ---------------------------------------------------------------------------
// Benchmark parameters
// ---------------------------------------------------------------------------

/// Number of producer iterations per pipeline run.
///
/// Drives the shutdown cascade: Producer completes after `ITERATIONS` batches,
/// closes buffer1, which eventually stops Consumer and Logger.
const ITERATIONS: u64 = 1_000;

/// Number of pipeline runs averaged per batch size.
const ROUNDS: u32 = 5;

/// Batch sizes exercised. Applied uniformly to n1_max, n2_max, and n3_max.
const BATCH_SIZES: &[usize] = &[1_000, 2_000, 5_000, 10_000, 20_000, 50_000, 100_000];

// ---------------------------------------------------------------------------
// Single pipeline run
// ---------------------------------------------------------------------------

/// Run the full pipeline once with the given `batch_size`; return `(total_tx, elapsed)`.
///
/// # Errors
///
/// Returns an error if any config builder or pipeline stage fails.
async fn run_bench(batch_size: usize) -> anyhow::Result<(usize, std::time::Duration)> {
    let producer_config = ProducerConfig::builder(batch_size)
        // Duration::ZERO: no artificial delay -- maximum throughput.
        .poll_interval1(std::time::Duration::ZERO)
        .iterations(ITERATIONS)
        .seed(42)
        .build()?;

    let consumer_config = ConsumerConfig::builder(batch_size)
        .poll_interval2(std::time::Duration::ZERO)
        // No .iterations(): drain until buffer closes.
        .seed(42)
        .build()?;

    let logger_config = LoggerConfig::builder(batch_size)
        .poll_interval3(std::time::Duration::ZERO)
        // No .iterations(): drain until buffer closes.
        .seed(42)
        .build()?;

    let buffer1 = ConcurrentBuffer::new();
    let buffer2 = ConcurrentBuffer2::new();
    let model = BenchModel::new();
    let modelizer = Modelizer::new(model);
    let alarm = LogAlarm::new();
    // BenchStorage: counts transactions, discards immediately -- no allocation.
    let storage = BenchStorage::new();

    let producer = Producer::new(producer_config);
    let consumer = Consumer::new(consumer_config);
    let logger = Logger::new(logger_config);

    let start = Instant::now();

    // Shutdown cascade identical to main.rs:
    //   Producer completes -> buffer1.close() -> Consumer drains+stops
    //   -> buffer2.close() -> Logger drains+stops.
    let consumer_then_close = async {
        let r = consumer.run(&buffer1, &modelizer, &alarm, &buffer2).await;
        buffer2.close();
        r
    };

    let (p, c, l) = tokio::join!(
        async {
            let r = producer.run(&buffer1).await;
            buffer1.close();
            r
        },
        consumer_then_close,
        logger.run(&buffer2, &storage)
    );
    p?;
    c?;
    l?;

    let elapsed = start.elapsed();
    Ok((storage.count(), elapsed))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    println!("bench: ITERATIONS={ITERATIONS}  ROUNDS={ROUNDS}  (storage cost excluded)");
    println!(
        "{:>10} | {:>10} | {:>10} | {:>10} | {:>10}",
        "batch_size", "total_tx", "min tx/s", "avg tx/s", "max tx/s"
    );
    println!("{:-<11}+{:-<12}+{:-<12}+{:-<12}+{:-<11}", "", "", "", "", "");

    for &batch_size in BATCH_SIZES {
        let mut total_tx_first = 0usize;
        let mut min_tps = f64::MAX;
        let mut max_tps = 0.0_f64;
        let mut sum_tps = 0.0_f64;

        for round in 0..ROUNDS {
            let (total_tx, elapsed) = run_bench(batch_size).await?;
            let tps = total_tx as f64 / elapsed.as_secs_f64();
            if round == 0 {
                total_tx_first = total_tx;
            }
            if tps < min_tps {
                min_tps = tps;
            }
            if tps > max_tps {
                max_tps = tps;
            }
            sum_tps += tps;
        }

        let avg_tps = sum_tps / f64::from(ROUNDS);

        println!(
            "{:>10} | {:>10} | {:>10} | {:>10} | {:>10}",
            fmt_number(batch_size),
            fmt_number(total_tx_first),
            fmt_number(min_tps as usize),
            fmt_number(avg_tps as usize),
            fmt_number(max_tps as usize),
        );
    }

    Ok(())
}

/// Format a `usize` with space-separated thousands groups (e.g. `1 234 567`).
fn fmt_number(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}
