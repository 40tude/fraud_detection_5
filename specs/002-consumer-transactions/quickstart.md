# Quickstart: Consumer Batch Processing

**Feature**: 002-consumer-transactions | **Date**: 2026-02-22

## Prerequisites

- Rust edition 2024 (nightly or stable 1.85+)
- Feature 001 (Producer) complete and passing
- `cargo test --workspace` green

## What This Feature Adds

1. **Domain crate** -- new types: `InferredTransaction`, `ModelVersion`, `ModelizerError`, `AlarmError`, and 4 port traits (`Buffer1Read`, `Buffer2`, `Modelizer`, `Alarm`)
2. **Consumer crate** -- new lib crate: `Consumer`, `ConsumerConfig`, `ConsumerError`
3. **Binary crate** -- mock adapters (mock Modelizer, mock Alarm, in-memory Buffer2) + Consumer wiring in `main.rs`

## Build and Test

```bash
cargo build --release
cargo test --workspace
```

## Run the Pipeline

```powershell
$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG
```

Expected output (after full wiring): Producer generates batches into Buffer1, Consumer reads from Buffer1, sends to Modelizer, triggers alarms for fraudulent transactions, writes inferred batches to Buffer2.

## Key Patterns to Observe

- **4 hexagonal ports**: Consumer depends on trait abstractions only -- no concrete adapter in the consumer crate
- **Best-effort alarms**: alarm failures logged but do not block Buffer2 write
- **Variable batch sizes**: N2 randomly chosen in `[1, N2_MAX]` each iteration
- **Model version switching**: Consumer issues switch command to Modelizer port; Modelizer owns state
- **Error separation**: `ConsumerError::Read` vs `ConsumerError::Write` distinguish which buffer failed
