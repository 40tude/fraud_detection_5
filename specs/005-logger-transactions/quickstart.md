# Quickstart: Logger Batch Persistence

**Feature**: 005-logger-transactions | **Date**: 2026-02-23

## Prerequisites

- Rust edition 2024 toolchain
- All 61 existing tests pass: `cargo test`
- Branch `005-logger-transactions` checked out

## New Crate Setup

```bash
# Create logger crate directory
mkdir -p crates/logger/src

# Add to workspace members in root Cargo.toml:
# members = [..., "crates/logger"]

# Logger Cargo.toml dependencies:
# domain, uuid, rand, thiserror, log, tokio (all workspace)
```

## Build and Test

```bash
cargo build --release
cargo test
```

## Run End-to-End

```powershell
# Infinite mode (CTRL+C to stop)
$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG

# Debug output (per-transaction)
$env:RUST_LOG='debug'; cargo run; Remove-Item env:RUST_LOG
```

## Expected Pipeline Output (info level)

```
producer.batch.written: iteration=1
consumer.batch.processed: iteration=1
logger.batch.persisted: iteration=1
producer.batch.written: iteration=2
...
# CTRL+C
main.shutdown: ctrl_c received, closing buffers
producer.run.stopped: buffer closed after N iteration(s)
consumer.run.stopped: buffer closed after M iteration(s)
logger.run.stopped: buffer closed after K iteration(s)
```

## Key Integration Points

1. `domain/src/lib.rs` -- new types and traits
2. `crates/logger/src/lib.rs` -- Logger domain logic
3. `crates/fraud_detection/src/adapters/concurrent_buffer2.rs` -- Buffer2+Buffer2Read adapter
4. `crates/fraud_detection/src/adapters/in_memory_storage.rs` -- Storage adapter
5. `crates/fraud_detection/src/main.rs` -- Logger wired into `tokio::join!`
