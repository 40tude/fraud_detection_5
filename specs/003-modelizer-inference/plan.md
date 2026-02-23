# Implementation Plan: Modelizer Inference

**Branch**: `003-modelizer-inference` | **Date**: 2026-02-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-modelizer-inference/spec.md`

## Summary

Create a generic Modelizer component (new `modelizer` lib crate) that delegates per-transaction classification to a `Model` hexagonal port (new trait in domain). Implement a DEMO adapter (in fraud_detection) with probabilistic fraud detection: version 4 at 4%, version 3 at 3%. Remove the existing `DemoModelizer` adapter.

## Technical Context

**Language/Version**: Rust edition 2024
**Primary Dependencies**: domain (workspace), log, rand (DemoModel adapter only), thiserror (domain), tokio
**Storage**: N/A (in-memory buffers from previous features)
**Testing**: `cargo test` -- TDD per constitution
**Target Platform**: Windows 11
**Project Type**: lib crate (modelizer) + adapter in binary crate (fraud_detection)
**Performance Goals**: N/A (proof-of-concept)
**Constraints**: N/A
**Scale/Scope**: Single pipeline component; ~4 files modified, ~3 files created

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Hexagonal Architecture | PASS | `Model` is a port (trait in domain); `DemoModel` is an adapter (in fraud_detection); Modelizer struct depends only on `Model` trait |
| II. Modular Monolith | PASS | New `modelizer` lib crate added to workspace; inter-crate deps through domain traits only |
| III. TDD | ENFORCED | All tasks will follow Red-Green-Refactor |
| IV. Pedagogical Clarity | PASS | Clear separation: `Model` trait (per-tx classification) vs `Modelizer` trait (per-batch orchestration); no magic |
| V. Async Pipeline | PASS | `Model` trait uses AFIT; consistent with Buffer1, Buffer2, Alarm ports |

No violations. No complexity tracking needed.

## Project Structure

### Documentation (this feature)

```text
specs/003-modelizer-inference/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
crates/
├── domain/src/lib.rs                          # ADD Model trait
├── modelizer/                                 # NEW lib crate
│   ├── Cargo.toml
│   └── src/lib.rs                             # Modelizer struct (impl domain::Modelizer)
├── fraud_detection/
│   ├── Cargo.toml                             # ADD modelizer dep; REMOVE direct domain::Modelizer usage
│   ├── src/main.rs                            # REPLACE DemoModelizer with Modelizer<DemoModel>
│   └── src/adapters/
│       ├── mod.rs                             # REPLACE demo_modelizer -> demo_model
│       ├── demo_model.rs                      # NEW: DemoModel adapter (impl domain::Model)
│       └── demo_modelizer.rs                  # DELETE
├── Cargo.toml (root)                          # ADD "crates/modelizer" to workspace members
```

**Structure Decision**: New `modelizer` crate sits parallel to `producer` and `consumer`. `DemoModel` adapter lives in `fraud_detection/src/adapters/` alongside other adapters, following the existing hexagonal layout.

## Complexity Tracking

> No violations -- section intentionally empty.
