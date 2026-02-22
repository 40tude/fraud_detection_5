# Quickstart: Producer -- Transaction Generation

**Feature Branch**: `001-producer-transactions`

## Prerequisites

- Rust 1.93+ (edition 2024)
- Windows 11 (project default)

## Build and Run

```bash
cargo build --release
cargo run
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
    .speed1(Duration::from_millis(50))        // 50ms between batches
    .iterations(Some(10))                     // 10 batches then stop
    .seed(Some(42))                           // deterministic for testing
    .build()?;
```

## Architecture Notes

- `domain` crate owns shared types (`Transaction`, `Buffer1` trait, `BufferError`)
- `producer` crate contains domain logic, depends only on `domain` traits
- `fraud_detection` binary crate owns adapters (`InMemoryBuffer`) and wires everything
- No component depends on a concrete buffer implementation (hexagonal architecture)
