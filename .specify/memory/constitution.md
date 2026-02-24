<!--
  Sync Impact Report
  ==================
  Version change: 0.0.0 (template) -> 1.0.0 (initial ratification)
  Modified principles: N/A (initial creation)
  Added sections:
    - I. Hexagonal Architecture with Dependency Inversion
    - II. Modular Monolith (Cargo Workspace)
    - III. Test-Driven Development (NON-NEGOTIABLE)
    - IV. Pedagogical Clarity
    - V. Asynchronous Pipeline with Variable Batching
    - Architectural Constraints (pipeline components, buffers, storage)
    - Development Workflow
    - Governance
  Removed sections: All template placeholders replaced
  Templates requiring updates:
    - .specify/templates/plan-template.md: OK (Constitution Check section is generic)
    - .specify/templates/spec-template.md: OK (user stories + requirements generic)
    - .specify/templates/tasks-template.md: OK (phase structure compatible)
    - .specify/templates/checklist-template.md: OK (generic)
    - .specify/templates/agent-file-template.md: OK (generic)
  Follow-up TODOs: None

  Version change: 1.0.0 -> 1.0.1 (patch: clarifications, 2026-02-23)
  Modified sections:
    - Model Versions: removed hardcoded ~5%/~8% rates (contradicted DEMO spec 4%/3%);
      replaced with adapter-defined rates; fixed "N-1: Improved detection" label
    - Pipeline table: Modelizer row field names model/version -> model_name/model_version
  Templates requiring updates: None (patch-level only)
  Follow-up TODOs: None

  Version change: 1.0.1 -> 1.1.0 (minor: added Principle VI, 2026-02-23)
  Added sections:
    - VI. Concurrent Component Lifecycle
  Modified sections:
    - Governance: compliance line updated I-V -> I-VI
  Templates requiring updates:
    - .specify/templates/plan-template.md: OK (Constitution Check section is generic)
    - .specify/templates/spec-template.md: OK (no principle-driven mandatory sections added)
    - .specify/templates/tasks-template.md: OK (phase structure compatible)
    - .specify/templates/checklist-template.md: OK (generic)
    - .specify/templates/agent-file-template.md: OK (generic)
  Follow-up TODOs: None

  Version change: 1.1.0 -> 1.2.0 (minor: technology conventions in Principle II, 2026-02-23)
  Modified sections:
    - II. Modular Monolith: added error-handling and logging crate conventions
  Templates requiring updates:
    - .specify/templates/plan-template.md: OK (Constitution Check section is generic)
    - .specify/templates/spec-template.md: OK
    - .specify/templates/tasks-template.md: OK
    - .specify/templates/checklist-template.md: OK
    - .specify/templates/agent-file-template.md: OK
  Follow-up TODOs: None
-->

# Fraud Detection Pipeline Constitution

## Core Principles

### I. Hexagonal Architecture with Dependency Inversion

Every component (Producer, Consumer, Modelizer, Logger) MUST follow
hexagonal architecture. Ports are Rust traits that define component
boundaries. Adapters are concrete implementations of those traits.

- No component may depend on a concrete implementation of a buffer,
  storage backend, or sibling component.
- All cross-boundary dependencies MUST point inward (domain depends
  on nothing; adapters depend on domain traits).
- Swapping an adapter (e.g., in-memory buffer to channel-based buffer)
  MUST NOT require changes to domain logic.

### II. Modular Monolith (Cargo Workspace)

The application is a Cargo workspace. Each pipeline component is a
separate library crate. A single binary crate orchestrates them.

- Crate boundaries: `producer`, `consumer`, `modelizer`, `logger`,
  plus shared types in a `domain` (or `common`) crate.
- Inter-crate dependencies flow through trait-based interfaces defined
  in the domain crate, never through concrete adapter crates.
- External services (databases, message brokers) are out of scope for
  v1; all adapters are in-process.
- **Error handling**: library crates (components, domain) MUST use
  `thiserror` for typed, structured errors. The binary crate MUST use
  `anyhow` for ergonomic error propagation in `main`.
- **Logging**: library crates MUST use the `log` facade crate
  (`log::info!`, `log::debug!`, etc.) and MUST NOT initialize a logger.
  The binary crate initializes `env_logger` once at startup before
  launching pipeline components.

### III. Test-Driven Development (NON-NEGOTIABLE)

Red-Green-Refactor cycle is mandatory for every unit of behavior.

- Tests MUST be written before implementation code.
- Tests MUST fail (red) before any production code is added.
- Only the minimal code to pass the failing test may be written (green).
- Refactoring happens only when all tests pass.
- Each crate MUST have its own `#[cfg(test)]` module and/or `tests/` directory.

### IV. Pedagogical Clarity

Every architectural decision MUST be explicit, traceable, and
understandable by a developer learning hexagonal architecture and
dependency inversion in Rust.

- Favor clarity over cleverness; favor explicitness over abstraction.
- Comments explain *why*, code explains *what*.
- No hidden magic: avoid proc macros or metaprogramming unless the
  alternative is significantly less clear.
- Each design choice (trait boundary, crate split, async strategy)
  MUST be justifiable in one sentence.

### V. Asynchronous Pipeline with Variable Batching

Communication between pipeline stages, buffers, and storage MUST be
asynchronous. Each stage operates at its own speed.

- Producer emits batches of size `N1` in `[1, N1_MAX]` per iteration.
- Consumer reads batches of size `N2` in `[1, N2_MAX]` from Buffer1.
- Modelizer classifies transactions (legit / fraudulent) using a
  selectable model version (N = latest, N-1 = previous).
- Logger reads batches of size `N3` in `[1, N3_MAX]` from Buffer2
  and persists to Storage.
- Batch sizes vary per iteration; components MUST handle variable-size
  input gracefully.

### VI. Concurrent Component Lifecycle

All pipeline components (Producer, Consumer, Logger) MUST run
concurrently as cooperative async tasks. No component may block
another from making progress.

- The binary crate MUST launch all components via `tokio::join!` on a
  single-thread Tokio runtime (`current_thread` flavor).
- Each component MUST run indefinitely (`iterations = None`) by default.
- Graceful shutdown MUST be supported via two mechanisms:
  1. Buffer closure: receiving `BufferError::Closed` on a read or write
     attempt is a normal stop signal; the component MUST return `Ok(())`.
  2. CTRL+C: the binary crate MUST handle `tokio::signal::ctrl_c()` and
     propagate shutdown to all components by closing the upstream buffer.
- `RefCell`-based adapters are valid under this principle because
  `tokio::join!` polls all futures on a single OS thread; no `Sync`
  bound is required on buffers or storage.
- `tokio::spawn` and multi-thread runtimes are out of scope for v1.

## Architectural Constraints

### Pipeline Components

| Component  | Input               | Output              | Key Behavior                              |
|------------|---------------------|---------------------|-------------------------------------------|
| Producer   | None                | Buffer1             | Generates batches of transactions every poll_interval1 |
| Consumer   | Buffer1             | Modelizer           | Read batches of transaction every poll_interval2                |
| Modelizer  | Consumer            | Consumer            | Infer and add fields `predicted_fraud`, `model_name` and `model_version` to each transaction. Model version (N, N-1) is selectable |
| Consumer   | Modelizer           | Buffer2 + Alert     | Forwards batches of inferred transactions every poll_interval2     |
| Logger     | Buffer2             | Storage             | Add fields `is_reviewed` (bool, false) and `actual_fraud` (Option<bool>, None) to the inferred transactions then persists the batch to Storage |

### Buffers and Storage

- Buffer1, Buffer2, and Storage are **abstract** (trait-defined).
- Valid concrete implementations include: `Vec`, `HashMap`, `BTreeMap`,
  channels (`mpsc`), files -- chosen per adapter, not per component.
- Each buffer/storage trait MUST define behavior for capacity limits
  (what happens when full).

### Transaction Model

A transaction carries at minimum: `id`, `amount`, `last_name`.
After inference it additionally carries: `predicted_fraud` (T|F), `model_name` and `model_version`.

### Model Versions

- **N**: Latest model version (default). Fraud detection rate is adapter-defined (e.g., DEMO: ~4%).
- **N-1**: Previous model version. Fraud detection rate is adapter-defined (e.g., DEMO: ~3%).
- Consumer selects which version to use; default is N (latest).

## Development Workflow

1. **Specify** -- Write or update feature spec (`/speckit.specify`).
2. **Plan** -- Produce implementation plan (`/speckit.plan`).
3. **Task** -- Generate task list (`/speckit.tasks`).
4. **Implement** -- TDD cycle per task: write test, see it fail,
   implement, refactor.
5. **Validate** -- Run full test suite (`cargo test --workspace`).
   Evaluate throughput with varying N1, N2, N3 values.

## Governance

- This constitution supersedes all other development practices in this
  repository.
- Amendments require: (a) documented rationale, (b) version bump per
  semver rules below, (c) updated Sync Impact Report.
- Versioning policy:
  - MAJOR: Principle removal or backward-incompatible redefinition.
  - MINOR: New principle or materially expanded guidance.
  - PATCH: Clarifications, wording, or non-semantic refinements.
- All code reviews MUST verify compliance with principles I-VI.
- Complexity MUST be justified; if a simpler alternative exists and
  meets the pedagogical goal, use it.

**Version**: 1.2.0 | **Ratified**: 2026-02-21 | **Last Amended**: 2026-02-23
