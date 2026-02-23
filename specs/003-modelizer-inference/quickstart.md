# Quickstart: Modelizer Inference

**Feature**: 003-modelizer-inference | **Date**: 2026-02-23

## Build & Run

```powershell
# Build all crates
cargo build --release

# Run pipeline (Producer -> Buffer1 -> Consumer -> Modelizer<DemoModel> -> Buffer2)
$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG

# Debug-level (per-transaction)
$env:RUST_LOG='debug'; cargo run; Remove-Item env:RUST_LOG
```

## Test

```powershell
# All workspace tests
cargo test

# Modelizer crate only
cargo test -p modelizer

# Domain crate (Model trait tests)
cargo test -p domain
```

## Key Integration Point

In `crates/fraud_detection/src/main.rs`, the DemoModel adapter plugs into the generic Modelizer:

```rust
use adapters::demo_model::DemoModel;
use modelizer::Modelizer;

let model = DemoModel::new(None);           // OS-seeded, defaults to version N (4)
let modelizer = Modelizer::new(model);      // Implements domain::Modelizer
// Consumer calls modelizer.infer(batch) and modelizer.switch_version(v)
```

## Crate Dependency Graph (this feature)

```text
domain          <-- defines Model trait, Modelizer trait, types
  ^       ^
  |       |
modelizer |     <-- Modelizer<M: Model> struct (implements domain::Modelizer)
  ^       |
  |       |
fraud_detection <-- DemoModel adapter (implements domain::Model)
                    + pipeline wiring in main.rs
```
