# Tasks: Logger Batch Persistence

**Feature**: `005-logger-transactions` | **Branch**: `005-logger-transactions` | **Generated**: 2026-02-23
**Input**: Design documents from `specs/005-logger-transactions/`
**TDD**: yes (Constitution Principle III -- red-green-refactor for all logger and adapter tests)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no incomplete task dependencies)
- **[Story]**: User story label (US1-US4); omitted in Setup, Foundational, and Polish phases
- Exact file paths included in each task description

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create logger crate and wire into workspace

- [ ] T001 Create crates/logger/src/lib.rs with module-level doc comment `//! Logger crate: reads InferredTransaction batches from Buffer2, persists as PendingTransaction.`
- [ ] T002 Create crates/logger/Cargo.toml with [package] (name="logger", version="0.1.0", edition="2024") and [dependencies] using workspace = true for: domain, thiserror, log, rand, tokio, uuid
- [ ] T003 Add "crates/logger" to workspace members and add `logger = { path = "crates/logger", version = "0.1.0" }` to [workspace.dependencies] in Cargo.toml (root)
- [ ] T004 Add `logger = { workspace = true }` to [dependencies] in crates/fraud_detection/Cargo.toml

**Checkpoint**: `cargo build` succeeds with empty logger crate

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extend domain crate with PendingTransaction, StorageError, Buffer2Read, and Storage traits; all user story phases depend on these

**CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Add PendingTransaction struct to crates/domain/src/lib.rs: `#[derive(Debug, Clone, PartialEq)]`, fields `inferred_transaction: InferredTransaction` and `prediction_confirmed: bool`, pub convenience method `id(&self) -> uuid::Uuid` delegating to `inferred_transaction.id()`
- [ ] T006 Add StorageError enum to crates/domain/src/lib.rs: `#[derive(Debug, Clone, PartialEq)]`, `#[derive(thiserror::Error)]`, variants `CapacityExceeded { capacity: usize }` and `Unavailable` (unit)
- [ ] T007 Add Buffer2Read AFIT trait to crates/domain/src/lib.rs: `#[expect(async_fn_in_trait, reason="no dyn dispatch needed; internal workspace only")]`, `async fn read_batch(&self, max: usize) -> Result<Vec<InferredTransaction>, BufferError>`
- [ ] T008 Add Storage AFIT trait to crates/domain/src/lib.rs: `#[expect(async_fn_in_trait, reason="no dyn dispatch needed; internal workspace only")]`, `async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError>`
- [ ] T009 Add `#[cfg(test)]` tests for PendingTransaction in crates/domain/src/lib.rs: construction with explicit fields, id() delegates through inferred_transaction.id(), prediction_confirmed accessible, Clone and PartialEq
- [ ] T010 Add `#[cfg(test)]` tests for StorageError in crates/domain/src/lib.rs: CapacityExceeded stores correct capacity usize, Unavailable is unit variant, thiserror Display impls compile

**Checkpoint**: `cargo test --package domain` passes (9 existing + T009/T010 new tests)

---

## Phase 3: User Story 1 - Read Batches from Buffer2 (Priority: P1) -- MVP

**Goal**: Logger reads variable-size batches from Buffer2Read port; N3 randomly chosen in [1, N3_MAX] each iteration

**Independent Test**: MockBuffer2Read with 100 items, N3_MAX=10, call log_once() 20 times, verify each batch size is in [1, 10]; verify batch capped when fewer items available

### Tests for User Story 1

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T011 [US1] Add MockBuffer2Read (Vec<InferredTransaction>, reads up to max, returns Err(BufferError::Closed) when empty+closed flag set) and MockStorage (Vec<PendingTransaction>, collects all writes, optional forced error) in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T012 [US1] Write failing test `test_log_once_batch_size_in_range`: N3_MAX=10, buffer with 100 items, call log_once() 20 times, assert each consumed batch size is in 1..=10 in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T013 [US1] Write failing test `test_log_once_batch_capped_at_available`: N3_MAX=20, buffer with 3 items, call log_once(), assert exactly 3 items consumed (capped at available count) in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T014 [US1] Write failing test `test_log_once_closed_empty_returns_error`: MockBuffer2Read closed+empty, call log_once(), assert Err(LoggerError::Read(BufferError::Closed)) in `#[cfg(test)]` in crates/logger/src/lib.rs

### Implementation for User Story 1

- [ ] T015 [US1] Add LoggerError to crates/logger/src/lib.rs: `#[derive(Debug, thiserror::Error)]`, variants `InvalidConfig { reason: String }`, `Read(#[from] BufferError)`, `Write(#[from] StorageError)`
- [ ] T016 [US1] Add LoggerConfig struct and LoggerConfigBuilder to crates/logger/src/lib.rs: LoggerConfig fields `n3_max: usize`, `speed3: Duration` (default 100ms), `iterations: Option<u64>`, `seed: Option<u64>`; `#[must_use] builder(n3_max: usize) -> LoggerConfigBuilder`; setter methods `speed3`, `iterations`, `seed`; `#[must_use] build() -> Result<LoggerConfig, LoggerError>` rejecting `n3_max == 0` with `InvalidConfig`
- [ ] T017 [US1] Add Logger struct to crates/logger/src/lib.rs: `#[derive(Debug)]`, fields `config: LoggerConfig` and `rng: RefCell<StdRng>`; `#[must_use] new(config: LoggerConfig) -> Self` seeding via `config.seed.map(StdRng::seed_from_u64).unwrap_or_else(StdRng::from_os_rng)`
- [ ] T018 [US1] Implement `Logger::log_once<B: Buffer2Read, S: Storage>(&self, buf2: &B, storage: &S) -> Result<(), LoggerError>` in crates/logger/src/lib.rs: compute `n3 = rng.borrow_mut().random_range(1..=n3_max)`, call `read_batch(n3)`, map each `InferredTransaction` to `PendingTransaction { inferred_transaction: tx, prediction_confirmed: false }`, call `write_batch(batch)`
- [ ] T019 [US1] Add `#[cfg(test)]` tests for LoggerConfig builder in crates/logger/src/lib.rs: `n3_max=5` builds Ok, `n3_max=0` returns Err(InvalidConfig), `speed3` default is 100ms, `speed3` setter overrides, `iterations` defaults to None, `seed` defaults to None

**Checkpoint**: `cargo test --package logger` passes T012-T014 and T019

---

## Phase 4: User Story 2 - Transform to PendingTransaction (Priority: P1)

**Goal**: Each InferredTransaction produces a PendingTransaction with `prediction_confirmed=false` and all original fields preserved

**Independent Test**: Provide 5 InferredTransactions to log_once(), verify 5 PendingTransactions in MockStorage each with `prediction_confirmed=false` and all original fields intact

### Tests for User Story 2

- [ ] T020 [US2] Write test `test_transform_preserves_all_fields`: 5 InferredTransactions with known fields, call log_once(), verify 5 PendingTransactions in storage each with `prediction_confirmed=false` and `inferred_transaction == original` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T021 [US2] Write test `test_transform_predicted_fraud_true_preserved`: InferredTransaction with `predicted_fraud=true`, call log_once(), assert resulting PendingTransaction has `inferred_transaction.predicted_fraud=true` and `prediction_confirmed=false` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T022 [US2] Write test `test_transform_predicted_fraud_false_preserved`: InferredTransaction with `predicted_fraud=false`, call log_once(), assert both fields false and independent in `#[cfg(test)]` in crates/logger/src/lib.rs

**Checkpoint**: `cargo test --package logger` passes T020-T022 (transformation tests pass with existing log_once() from Phase 3)

---

## Phase 5: User Story 3 - Persist to Storage (Priority: P1)

**Goal**: All pending transactions in each batch are written to Storage; StorageError variants propagate immediately as LoggerError::Write

**Independent Test**: 8 InferredTransactions in buffer, call log_once(), verify MockStorage contains exactly 8 PendingTransactions; verify both CapacityExceeded and Unavailable propagate

### Tests for User Story 3

- [ ] T023 [US3] Write test `test_persist_all_items`: MockBuffer2Read with 8 items, call log_once(), assert `mock_storage.items.len() == 8` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T024 [US3] Write test `test_persist_capacity_exceeded_propagates`: MockStorage configured to return `StorageError::CapacityExceeded { capacity: 0 }`, call log_once(), assert `Err(LoggerError::Write(StorageError::CapacityExceeded { capacity: 0 }))` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T025 [US3] Write test `test_persist_unavailable_propagates`: MockStorage returns `StorageError::Unavailable`, call log_once(), assert `Err(LoggerError::Write(StorageError::Unavailable))` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T026 [US3] Add `log::debug!` in log_once() per FR-011 in crates/logger/src/lib.rs: emit `"logger.log_once: batch_size={n3}"` at the start of each call

**Checkpoint**: `cargo test --package logger` passes T023-T025 (storage and error propagation tests)

---

## Phase 6: User Story 4 - Continuous Async Loop (Priority: P1)

**Goal**: run() loops read-transform-persist with speed3 delay; stops gracefully returning Ok(()) when Buffer2 is closed+drained; optional iteration limit for testing

**Independent Test**: iterations=3, buffer with 30 items, run() returns Ok(()), MockStorage has exactly 3 batches worth of items; no-limit run stops cleanly after buffer closes

### Tests for User Story 4

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T027 [US4] Write failing test `test_run_iteration_limit`: Logger with `iterations=Some(3)`, MockBuffer2Read with 30 items, call run(), assert Ok(())) and MockStorage item count equals sum of 3 batches in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T028 [US4] Write failing test `test_run_stops_on_closed`: Logger with `iterations=None`, MockBuffer2Read pre-loaded with 5 batches then closed, call run(), assert Ok(()) in `#[cfg(test)]` in crates/logger/src/lib.rs
- [ ] T029 [US4] Write failing test `test_run_zero_delay`: Logger with `speed3=Duration::ZERO`, `iterations=Some(2)`, buffer with 20 items, call run(), assert Ok(()) completes without panic in `#[cfg(test)]` in crates/logger/src/lib.rs

### Implementation for User Story 4

- [ ] T030 [US4] Implement `Logger::run<B: Buffer2Read, S: Storage>(&self, buf2: &B, storage: &S) -> Result<(), LoggerError>` in crates/logger/src/lib.rs: loop calling log_once(); on `Err(LoggerError::Read(BufferError::Closed))` break Ok(()); propagate all other errors; call `tokio::time::sleep(config.speed3)`; break when iteration counter reaches `config.iterations`
- [ ] T031 [US4] Add `log::info!` in run() per quickstart.md expected output in crates/logger/src/lib.rs: emit `"logger.batch.persisted: iteration={n}"` after each successful log_once() call
- [ ] T032 [US4] Add `log::info!` in run() on graceful exit in crates/logger/src/lib.rs: emit `"logger.run.stopped: buffer closed after {n} iteration(s)"` matching quickstart.md format

**Checkpoint**: `cargo test --package logger` passes all logger tests (T012-T014, T019-T029)

---

## Phase 7: Adapters (ConcurrentBuffer2 + InMemoryStorage)

**Purpose**: Implement adapters for the concurrent pipeline; ConcurrentBuffer2 and InMemoryStorage groups are [P] (different files)

- [ ] T033 Add `pub mod concurrent_buffer2;` and `pub mod in_memory_storage;` declarations to crates/fraud_detection/src/adapters/mod.rs

### ConcurrentBuffer2 Adapter

- [ ] T034 [P] Create ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: `#[derive(Debug)]` struct with `inner: RefCell<ConcurrentBuffer2Inner { data: Vec<InferredTransaction>, closed: bool }>`, `#[must_use] new() -> Self`, `close(&self)` sets `inner.borrow_mut().closed = true`
- [ ] T035 [P] Implement Buffer2 (write_batch) for ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: `write_batch(&self, batch: Vec<InferredTransaction>)` extends `inner.borrow_mut().data`
- [ ] T036 [P] Implement Buffer2Read (read_batch) for ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: drain up to max items from front; if empty+open yield `tokio::task::yield_now()` and retry (borrow MUST drop before `.await`); if empty+closed return `Err(BufferError::Closed)`
- [ ] T037 [P] Add `#[cfg(test)]` tests for ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: write+read roundtrip, ordering preserved, close signals Closed when empty (mirror CB-T01..CB-T06 pattern from ConcurrentBuffer)

### InMemoryStorage Adapter

- [ ] T038 [P] Create InMemoryStorage in crates/fraud_detection/src/adapters/in_memory_storage.rs: `#[derive(Debug)]` struct with `inner: RefCell<Vec<PendingTransaction>>` and `capacity: usize`, `#[must_use] new(capacity: usize) -> Self`
- [ ] T039 [P] Implement Storage (write_batch) for InMemoryStorage in crates/fraud_detection/src/adapters/in_memory_storage.rs: check `inner.borrow().len() + batch.len() > capacity` -> `Err(StorageError::CapacityExceeded { capacity: self.capacity })`, else `inner.borrow_mut().extend(batch)`
- [ ] T040 [P] Add `#[cfg(test)]` tests for InMemoryStorage in crates/fraud_detection/src/adapters/in_memory_storage.rs: write_batch stores all items, CapacityExceeded returned with correct capacity when full, multiple batches accumulate

**Checkpoint**: `cargo test --package fraud_detection` passes including T037 and T040

---

## Phase 8: Pipeline Integration

**Purpose**: Wire Logger into main.rs; replace InMemoryBuffer2 with ConcurrentBuffer2; Consumer closure triggers buffer2.close() to cascade Logger shutdown

- [ ] T041 Replace InMemoryBuffer2 with `ConcurrentBuffer2::new()` as `buffer2` in crates/fraud_detection/src/main.rs; update Consumer call to pass `&buffer2` as Buffer2 write port; update imports
- [ ] T042 Instantiate `InMemoryStorage::new(usize::MAX)` and build `LoggerConfig` via `LoggerConfig::builder(n3_max).speed3(...).build()?` and `Logger::new(logger_config)` in crates/fraud_detection/src/main.rs
- [ ] T043 Add `logger.run(&buffer2, &storage)` as third arm in the `tokio::join!` pipeline alongside producer and consumer in crates/fraud_detection/src/main.rs
- [ ] T044 Update pipeline async block in crates/fraud_detection/src/main.rs so `buffer2.close()` is called after the consumer arm completes (shutdown cascade: buffer1 closed -> consumer finishes -> buffer2.close() -> logger finishes)
- [ ] T045 Verify CTRL+C shutdown path in crates/fraud_detection/src/main.rs: closing buffer1 is sufficient (cascade handles buffer2); confirm logger arm in tokio::join! resolves cleanly after buffer2 drains

**Checkpoint**: `cargo run` with RUST_LOG=info shows `logger.batch.persisted` lines; CTRL+C shows all three stopped messages

---

## Phase 9: Polish & Cross-Cutting Concerns

- [ ] T046 [P] Verify ms-rust compliance in crates/logger/src/lib.rs: `#[must_use]` on all constructors and builder methods, `#[derive(Debug)]` on all public types, `#[expect(..., reason="...")]` used instead of `#[allow]`, add compliance comment `// Rust guideline compliant 2026-02-23`
- [ ] T047 [P] Add compliance comment `// Rust guideline compliant 2026-02-23` and verify `#[must_use]`, `#[derive(Debug)]`, `#[expect]` usage in crates/fraud_detection/src/adapters/concurrent_buffer2.rs and crates/fraud_detection/src/adapters/in_memory_storage.rs
- [ ] T048 Run `cargo test` at workspace root; verify all 70+ tests pass (61 existing + new domain + logger + adapter tests); fix any compilation errors or assertion failures in affected crates/*/src/lib.rs files

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies -- start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 -- BLOCKS all user story phases and adapter phases
- **US1 (Phase 3)**: Depends on Phase 2 -- builds Logger struct, config, log_once() foundation
- **US2 (Phase 4)**: Depends on Phase 3 -- transformation tests reuse log_once() from Phase 3; can run parallel with Phase 5
- **US3 (Phase 5)**: Depends on Phase 3 -- storage tests reuse log_once() from Phase 3; can run parallel with Phase 4
- **US4 (Phase 6)**: Depends on Phase 5 -- run() wraps the fully tested log_once()
- **Adapters (Phase 7)**: Depends on Phase 2 only -- independent of logger logic; can run parallel with Phases 3-6
- **Integration (Phase 8)**: Depends on Phases 6 and 7 -- both logger logic and adapters must be complete
- **Polish (Phase 9)**: Depends on Phase 8

### User Story Dependencies

- **US1 (Phase 3)**: First -- Logger struct, LoggerConfig, mock adapters, complete log_once() implementation
- **US2 (Phase 4)**: After US1 -- transformation verification tests only; no new implementation
- **US3 (Phase 5)**: After US1, parallel with US2 -- storage and error propagation tests
- **US4 (Phase 6)**: After US3 -- run() loop wraps fully-exercised log_once()

### Parallel Opportunities

- T034-T037 (ConcurrentBuffer2 group) [P] vs T038-T040 (InMemoryStorage group) [P]: different files
- T046 [P] vs T047 [P]: different files
- Phase 7 (Adapters) can proceed in parallel with Phases 3-6 (different crates, different files)

---

## Parallel Example: Adapters Phase

```bash
# Two agents can work in parallel (different .rs files):
Agent A: T033 (mod.rs), then T034, T035, T036, T037 (concurrent_buffer2.rs)
Agent B: T038, T039, T040 (in_memory_storage.rs)
```

---

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational domain types (T005-T010)
3. Complete Phase 3: US1 -- log_once() with batch reading, transformation, persistence (T011-T019)
4. **STOP and VALIDATE**: `cargo test --package logger` -- all US1 tests pass
5. Proceed to Phases 4-5 for deeper transformation and persistence coverage

### Incremental Delivery

1. Setup + Foundational -> domain types ready
2. US1 + US2 + US3 -> complete log_once() tested from all angles
3. US4 -> complete run() loop independently testable
4. Adapters -> production concurrent adapters (can overlap with US phases)
5. Integration -> full end-to-end pipeline

### TDD Cycle per User Story Phase

1. Write mock adapters (T011 or reuse from Phase 3)
2. Write failing tests -- verify they FAIL (compile error or assertion failure)
3. Implement production code to make tests pass
4. Refactor if needed
5. Commit at each checkpoint

---

## Notes

- [P] tasks operate on different files; no incomplete task dependencies within the [P] group
- [Story] label maps each task to its user story for traceability
- TDD: tests written and verified FAILING before implementation
- **ConcurrentBuffer2 borrow pattern**: `let items = { let b = inner.borrow(); ... }; drop explicitly before yield_now().await` (same as ConcurrentBuffer)
- **LoggerError #[from]**: both `Read(#[from] BufferError)` and `Write(#[from] StorageError)` are valid since each source type maps to exactly one variant (R7)
- **run() stop condition**: `Err(LoggerError::Read(BufferError::Closed))` -> break and return Ok(()); all other errors propagate
- **Windows 11 / current_thread**: RefCell valid; no Sync required; tokio single-thread flavor
- **Buffer2 already exists** in domain crate (from feature 004); only Buffer2Read and Storage are new (T007, T008)
