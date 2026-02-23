# Research: Modelizer Inference

**Feature**: 003-modelizer-inference | **Date**: 2026-02-23

## R1: Model Trait Design

**Decision**: New `Model` trait in domain crate with four methods -- `classify` (async, per-transaction), `name` (sync getter), `active_version` (sync getter), `switch_version` (async).

**Rationale**: FR-013 mandates per-transaction operation. FR-014 mandates self-describing (name + version). Async on `classify` and `switch_version` because future Model adapters may call external services (MLFlow). Sync getters for `name`/`active_version` because they return cached metadata with no I/O.

**Alternatives considered**:
- Sync-only trait: rejected because future adapters need async I/O for classification.
- Batch-level trait (like current `Modelizer`): rejected by FR-013 (per-transaction).
- Separate error type (`ModelError`): rejected because `ModelizerError` variants (`InferenceFailed`, `SwitchFailed`) already map naturally; adding a wrapper adds conversion boilerplate with no semantic gain.

## R2: RNG Ownership

**Decision**: The `DemoModel` adapter owns the `StdRng`. The `Modelizer` struct does not touch randomness.

**Rationale**: Fraud detection probability is model-specific behavior (FR-005, FR-006). The Modelizer struct is a generic bridge that delegates to any `Model` impl. Placing the RNG in DemoModel keeps it with the behavior that needs it, and avoids leaking randomness concerns into the generic component.

**Alternatives considered**:
- RNG in Modelizer struct, passed to Model via method param: rejected because it couples the generic component to a DEMO-specific concern.
- Global/thread-local RNG: rejected by FR-011 (seeded reproducibility) and Principle IV (explicit over magical).

## R3: Error Reuse

**Decision**: `Model` trait methods return `Result<_, ModelizerError>`, reusing the existing domain error type.

**Rationale**: `ModelizerError::InferenceFailed` maps to `classify` failures; `ModelizerError::SwitchFailed` maps to `switch_version` failures. No new variant needed. Avoids a conversion layer between `ModelError -> ModelizerError`.

**Alternatives considered**:
- Dedicated `ModelError` enum: rejected because it would require `impl From<ModelError> for ModelizerError` with identical variants -- pure boilerplate.

## R4: Modelizer Struct Design (No Config)

**Decision**: `Modelizer<M: Model>` struct with `Modelizer::new(model: M)` constructor. No config, no builder.

**Rationale**: The Modelizer struct is a thin bridge: iterate batch, delegate `classify`, read metadata, build `InferredTransaction`. It has no speed, no batch-size cap, no seed, no iterations. A config/builder would be empty ceremony. Consumer already controls cadence and batch size.

**Alternatives considered**:
- `ModelizerConfig` with builder (for symmetry with Producer/Consumer): rejected because there are no configurable parameters. Adding empty config violates Principle IV (clarity over ceremony).

## R5: Version String Representation

**Decision**: DEMO model returns `"4"` for version N and `"3"` for version N-1. Model name is `"DEMO"`.

**Rationale**: FR-003 ("DEMO"), FR-004 (versions 3 and 4), spec assumption ("version numbers presented as strings"). Concrete version mapping lives in the adapter (FR-015), not in the generic Modelizer.

**Alternatives considered**:
- Prefixed strings ("v3", "v4"): rejected because spec examples use bare numbers and say "version 3" / "version 4".

## R6: Infer Optimization

**Decision**: Read `model.name()` and `model.active_version()` once before the batch loop; clone for each `InferredTransaction`.

**Rationale**: FR-009 says version switch takes effect on the *next* `infer` call, so name and version are stable within a single infer invocation. Reading once avoids repeated calls. Cloning a short string per transaction is negligible.

**Alternatives considered**:
- Read per-transaction: functionally equivalent but wasteful.
- Use `Rc<str>` or `Arc<str>`: over-engineering for a proof-of-concept.

## R7: Logging Pattern

**Decision**: Modelizer lib crate uses `log` facade (debug for batch-level, info for version switch). DemoModel adapter uses `log` facade for classification stats. Binary crate initializes `env_logger`.

**Rationale**: Per the GENERAL architecture diagram, components log via `log` crate; only the application initializes `env_logger`. Consistent with Producer and Consumer crates.

## R8: AFIT Suppression

**Decision**: `#[expect(async_fn_in_trait, reason = "...")]` on the `Model` trait definition, matching all other port traits.

**Rationale**: Consistent with `Buffer1`, `Buffer1Read`, `Buffer2`, `Modelizer`, and `Alarm` trait definitions in domain.
