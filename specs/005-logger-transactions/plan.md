# Implementation Plan: Logger Batch Persistence

**Branch**: `005-logger-transactions` | **Date**: 2026-02-23 | **Spec**: `specs/005-logger-transactions/spec.md`
**Input**: Feature specification from `/specs/005-logger-transactions/spec.md`

## Summary

Logger reads variable-size batches of `InferredTransaction` from a `Buffer2Read` hexagonal port, converts each to a `PendingTransaction` (wrapping the original + `prediction_confirmed = false`), and persists them to a `Storage` hexagonal port. New `logger` library crate follows Producer/Consumer patterns. Concurrent pipeline wiring via `tokio::join!` in `main.rs`. New domain types: `PendingTransaction`, `StorageError`, `Buffer2Read` trait, `Storage` trait. New adapters: `ConcurrentBuffer2` (Buffer2 + Buffer2Read), `InMemoryStorage`.

## Technical Context

**Language/Version**: Rust edition 2024 (nightly/stable 1.85+)
**Primary Dependencies**: `domain` (workspace), `thiserror`, `log`, `rand`, `tokio`, `uuid`
**Storage**: In-memory `VecDeque<PendingTransaction>` adapter (proof-of-concept)
**Testing**: `cargo test` -- `#[cfg(test)]` inline modules with mock adapters
**Target Platform**: Windows 11 (single-thread Tokio, `current_thread` flavor)
**Project Type**: Library crate (`logger`) + binary crate (`fraud_detection`) wiring
**Performance Goals**: N/A (didactic proof-of-concept)
**Constraints**: Single OS thread (`RefCell`, no `Sync`); cooperative async only
**Scale/Scope**: ~200-300 lines new logger crate, ~100 lines new domain types/traits, ~150 lines new adapters, ~30 lines main.rs changes

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | Evidence |
|---|-----------|--------|----------|
| I | Hexagonal Architecture | PASS | Logger depends on `Buffer2Read` + `Storage` traits only; no concrete adapter knowledge |
| II | Modular Monolith | PASS | New `crates/logger/` lib crate; `thiserror` for errors, `log` facade for logging; binary crate wires adapters |
| III | TDD | PASS | All tasks follow red-green-refactor; tests before implementation |
| IV | Pedagogical Clarity | PASS | Follows exact same patterns as Producer/Consumer; no new abstractions |
| V | Async Variable Batching | PASS | N3 in `[1, N3_MAX]` random per iteration; poll_interval3 delay between iterations |
| VI | Concurrent Lifecycle | PASS | Logger joins `tokio::join!`; runs indefinitely by default; stops on Buffer2 closed+drained; `RefCell` adapters valid on `current_thread` |

No violations. Gate PASSES.

## Project Structure

### Documentation (this feature)

```text
specs/005-logger-transactions/
+-- plan.md              # This file
+-- research.md          # Phase 0 output
+-- data-model.md        # Phase 1 output
+-- quickstart.md        # Phase 1 output
+-- tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
crates/
+-- domain/src/lib.rs                          # + PendingTransaction, StorageError, Buffer2Read, Storage traits
+-- logger/                                    # NEW crate
|   +-- Cargo.toml
|   +-- src/lib.rs                             # Logger, LoggerConfig, LoggerConfigBuilder, LoggerError
+-- fraud_detection/
    +-- Cargo.toml                             # + logger dependency
    +-- src/
        +-- main.rs                            # + Logger wiring in tokio::join!
        +-- adapters/
            +-- mod.rs                         # + concurrent_buffer2, in_memory_storage modules
            +-- concurrent_buffer2.rs          # NEW: Buffer2 + Buffer2Read adapter (yield-on-empty)
            +-- in_memory_storage.rs           # NEW: Storage adapter (VecDeque<PendingTransaction>)
```

**Structure Decision**: Follows established workspace layout. Logger is a new lib crate in `crates/logger/` with the same structure as `crates/producer/` and `crates/consumer/`. New adapters live in `crates/fraud_detection/src/adapters/`.

## Complexity Tracking

No constitution violations -- table empty.
