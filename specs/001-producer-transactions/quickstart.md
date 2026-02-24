# Quickstart: Producer -- Transaction Generation

**Feature Branch**: `001-producer-transactions`

## Prerequisites

- Rust 1.93+ (edition 2024)
- Windows 11 (project default)

## Build and Run

```bash
cargo build
$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG

cargo build --release
cargo run
```

Expected output:

```text
$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `C:/Users/phili/rust_builds/Documents/Programmation/rust/13_fraud_detection_5\debug\fraud_detection.exe`
[2026-02-22T11:52:05Z INFO  producer] producer.batch.written: iteration=1
[2026-02-22T11:52:05Z INFO  producer] producer.batch.written: iteration=2
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=3
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=4
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=5
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=6
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=7
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=8
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=9
[2026-02-22T11:52:06Z INFO  producer] producer.batch.written: iteration=10
[2026-02-22T11:52:06Z INFO  fraud_detection] producer.run.complete: total_transactions=423
```

## Run Tests

```bash
cargo test --workspace

```

## Set Log Level

```bash
RUST_LOG=info cargo run
RUST_LOG=debug cargo test
```

## Project Layout

```text
Cargo.toml                          # workspace manifest + lint config
crates/
  domain/
    Cargo.toml                      # uuid, thiserror, log, tokio
    src/lib.rs                      # Transaction, BufferError, Buffer1 trait
  producer/
    Cargo.toml                      # domain, rand, thiserror, log, tokio
    src/lib.rs                      # Producer, ProducerConfig, ProducerError
  fraud_detection/
    Cargo.toml                      # domain, producer, anyhow, env_logger, log, tokio
    src/
      main.rs                       # async main, wiring
      adapters/
        mod.rs
        in_memory_buffer.rs         # InMemoryBuffer impl Buffer1
```

## Configuration

`ProducerConfig` via builder pattern:

```rust
let config = ProducerConfig::builder(100)    // n1_max = 100
    .poll_interval1(Duration::from_millis(50))        // 50ms between batches
    .iterations(Some(10))                     // 10 batches then stop
    .seed(Some(42))                           // deterministic for testing
    .build()?;
```

## Architecture Notes

- `domain` crate owns shared types (`Transaction`, `Buffer1` trait, `BufferError`)
- `producer` crate contains domain logic, depends only on `domain` traits
- `fraud_detection` binary crate owns adapters (`InMemoryBuffer`) and wires everything
- No component depends on a concrete buffer implementation (hexagonal architecture)
