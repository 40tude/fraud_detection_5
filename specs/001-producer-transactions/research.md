# Research: Producer -- Transaction Generation

**Feature Branch**: `001-producer-transactions`
**Created**: 2026-02-22

## R1: Error Handling Strategy

**Decision**: Library crates use `thiserror` 2.x; binary crate uses `anyhow` 1.x
**Rationale**: Diagram GENERAL quadrant mandates this split. `thiserror` gives typed,
ergonomic error enums in libs; `anyhow` gives frictionless propagation in the binary.
**Alternatives considered**: `std::io::Error` wrapping (too verbose), single `anyhow`
everywhere (loses typed errors at crate boundaries).

## R2: Logging Strategy

**Decision**: Library crates depend on `log` 0.4 facade; binary crate initializes
`env_logger` 0.11
**Rationale**: Diagram GENERAL quadrant mandates this. `log` facade decouples emission
from backend; `env_logger` reads `RUST_LOG` at runtime.
**Alternatives considered**: `tracing` (more powerful but heavier, overkill for PoC).

## R3: Async Runtime

**Decision**: `tokio` 1.x with features `rt`, `macros`, `time`
**Rationale**: Constitution mandates async communication. `tokio` is the de-facto
standard. Single-threaded runtime (`current_thread`) is sufficient for PoC.
`time` feature needed for `tokio::time::sleep` (speed1 delay between iterations).
**Alternatives considered**: `async-std` (smaller ecosystem), manual
`futures::executor` (no timer support).

## R4: UUID Generation

**Decision**: `uuid` 1.x with no extra feature flags. Generate via
`uuid::Builder::from_random_bytes(bytes).into_uuid()` using our seedable RNG.
**Rationale**: Skipping `v4` feature avoids pulling `getrandom`. Builder API sets
version/variant bits automatically. Deterministic UUIDs come from seedable RNG bytes.
**Alternatives considered**: `uuid` with `v4` feature (non-deterministic, breaks
seedable tests).

## R5: Random Number Generation

**Decision**: `rand` 0.9.x with default features
**Rationale**: rand 0.9 API:
- `StdRng::from_os_rng()` for production
- `StdRng::seed_from_u64(seed)` for deterministic tests
- `rng.random_range(1..=n)` for batch sizes and amounts
- `rng.fill_bytes(&mut buf)` for UUID bytes
**Alternatives considered**: `fastrand` (no seedable StdRng equivalent).

## R6: Async Fn in Trait (AFIT)

**Decision**: Use native `async fn` in `Buffer1` trait with
`#[expect(async_fn_in_trait, reason = "...")]` on the trait definition.
**Rationale**: Verified on Rust 1.93.1 -- the `async_fn_in_trait` lint is still
warn-by-default for public traits. Lint message: "auto trait bounds cannot be
specified". We use static dispatch (`impl Buffer1`) so `dyn` incompatibility is
irrelevant. `#[expect]` will auto-warn if the lint is removed in a future Rust version.
**Alternatives considered**: `async-trait` crate (heap-allocates futures, unnecessary
overhead), manual desugaring to `-> impl Future + Send` (verbose, no benefit for
internal workspace).

## R7: Interior Mutability for In-Memory Adapter

**Decision**: `std::cell::RefCell<Vec<Transaction>>` for single-threaded adapter
**Rationale**: `Buffer1::write_batch` takes `&self` (not `&mut self`) per hexagonal
port design -- callers should not need exclusive access. `RefCell` gives interior
mutability with runtime borrow checks. Single-threaded tokio runtime means no `Send`
bound required on the future; `RefCell` is sufficient.
**Alternatives considered**: `Mutex` (unnecessary overhead for single-threaded runtime),
`&mut self` on trait (forces exclusive access at call sites, breaks composability).

## R8: Amount Generation

**Decision**: Generate integer cents in `[1, 1_000_000]`, then divide by `100.0`
**Rationale**: Avoids floating-point rounding issues. Range 1..=1_000_000 cents
maps to 0.01..=10_000.00 with exactly 2 decimal places guaranteed.
**Alternatives considered**: Direct `f64` generation with rounding (rounding errors),
`rust_decimal` crate (overkill for PoC).

## R9: Workspace Lint Configuration

**Decision**: Centralized lints in root `Cargo.toml` under `[workspace.lints.clippy]`
and `[workspace.lints.rust]`. Each crate inherits via `[lints] workspace = true`.
**Rationale**: ms-rust M-STATIC-VERIFICATION guideline requires strict clippy
configuration. Centralized config avoids duplication across crates.
**Alternatives considered**: Per-crate `clippy.toml` files (fragmented, hard to maintain).

## R10: Last Name Generation

**Decision**: Hardcoded array of ~20 common last names; select randomly per transaction
**Rationale**: Simplest approach for PoC. No external data file needed. Sufficient
variety for visual inspection of pipeline output.
**Alternatives considered**: `fake` crate (heavy dependency for a static list),
external CSV file (file I/O complexity for no benefit).

## Dependency Summary

| Crate | Version | Feature Flags | Used In |
|-------|---------|---------------|---------|
| `thiserror` | `"2"` | none | `domain`, `producer` |
| `log` | `"0.4"` | none | `domain`, `producer` |
| `env_logger` | `"0.11"` | none | `fraud_detection` (binary) |
| `anyhow` | `"1"` | none | `fraud_detection` (binary) |
| `uuid` | `"1"` | none | `domain` |
| `rand` | `"0.9"` | none (defaults) | `producer` |
| `tokio` | `"1"` | `rt`, `macros`, `time` | all crates |
