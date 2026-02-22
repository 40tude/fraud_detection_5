# Research: Consumer Batch Processing

**Feature**: 002-consumer-transactions | **Date**: 2026-02-22

## Decision 1: Buffer1Read Trait Semantics

**Decision**: New `Buffer1Read` trait with `async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError>`.

**Rationale**: Spec (FR-001) requires a separate read trait. AFIT pattern already established by `Buffer1`. Async semantics: awaits until data available (adapter decides), returns `Err(BufferError::Closed)` when closed and drained.

**Alternatives considered**:
- Extending existing `Buffer1` trait with read method -- rejected: violates separation of read/write ports.
- Returning `Option<Vec<Transaction>>` for closed signal -- rejected: `BufferError::Closed` already exists and is consistent.

## Decision 2: BufferError Reuse

**Decision**: Reuse single `BufferError` for Buffer1Read, Buffer1 (write), and Buffer2 (write).

**Rationale**: `Full` applies to writes, `Closed` to both. Reads never return `Full`. Keeping one type avoids proliferating error enums for a PoC. Pedagogical clarity: one vocabulary for all buffer operations.

**Alternatives considered**:
- Split `BufferReadError` / `BufferWriteError` -- rejected: adds complexity without benefit; unused variants in reads are harmless.

## Decision 3: Consumer Generic Parameters

**Decision**: 4 type parameters per call: `run<B1, M, A, B2>(&self, buf1: &B1, modelizer: &M, alarm: &A, buf2: &B2)`.

**Rationale**: Mirrors Producer pattern (1 param per port). Maximizes testability with concrete test doubles. Explicit port injection is pedagogically valuable.

**Alternatives considered**:
- Bundled `ConsumerContext<B1, M, A, B2>` struct -- rejected: over-engineering for PoC scope.
- `dyn Trait` objects -- rejected: violates "no dyn dispatch" in existing codebase; adds runtime cost.

## Decision 4: Alarm Failure Reporting

**Decision**: `consume_once` returns `Result<Vec<AlarmError>, ConsumerError>`. Ok variant carries collected alarm failures (empty if none). Buffer2 write always happens regardless.

**Rationale**: Spec requires best-effort alarms (FR-006, FR-013). Real errors (Buffer, Modelizer) are `Err`. Alarm failures are informational in `Ok` path. Departs from Producer's fail-fast only for this specific scenario as spec mandates.

**Alternatives considered**:
- `ConsumerError::AlarmsPartiallyFailed` variant -- rejected: conflates "batch processed successfully" with "hard error".
- Ignoring alarm failures entirely -- rejected: FR-013 requires reporting.

## Decision 5: InferredTransaction Composition

**Decision**: `InferredTransaction` wraps `Transaction` via composition field, plus `predicted_fraud: bool`, `model_name: String`, `model_version: String`.

**Rationale**: Avoids field duplication. Teaches composition over inheritance. `model_version` is a string in the data type (populated by Modelizer); `ModelVersion` enum is used only for switch commands.

**Alternatives considered**:
- Flat struct with duplicated fields -- rejected: violates DRY, harder to maintain consistency.
- Using `ModelVersion` enum in the struct -- rejected: spec says "string" and Modelizer owns the mapping.

## Decision 6: ConsumerError Variants

**Decision**: Four variants: `InvalidConfig`, `Read(BufferError)`, `Inference(ModelizerError)`, `Write(BufferError)`.

**Rationale**: Distinguishes Buffer1 read errors from Buffer2 write errors (both use `BufferError`). No `#[from]` since two variants share source type; manual mapping via `.map_err()`.

**Alternatives considered**:
- Single `Buffer { source: BufferError }` for both -- rejected: caller cannot tell which buffer failed.

## Decision 7: Dependencies

**Decision**: Consumer crate uses `domain`, `rand`, `thiserror`, `log`, `tokio` (all workspace deps). No `anyhow` (lib crate). No `env_logger` (binary concern).

**Rationale**: Diagram General quadrant: lib crates use `thiserror`, binary uses `anyhow`. Components use `log` macros; binary initializes `env_logger`. Mirrors Producer's dependency set exactly.

## Decision 8: Model Version Switching

**Decision**: Consumer exposes pass-through `switch_model_version` calling `Modelizer::switch_version`. Consumer does not store version state. Modelizer owns version internally.

**Rationale**: Spec FR-008/FR-010: "Modelizer owns its version state; Consumer issues switch command." Consumer is a conduit, not a state holder. Binary demo can call switch between iterations.
