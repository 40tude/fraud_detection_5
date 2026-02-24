# Tasks: Logger Batch Persistence

**Feature**: `005-logger-transactions` | **Branch**: `005-logger-transactions` | **Generated**: 2026-02-23
**Input**: Design documents from `specs/005-logger-transactions/`
**TDD**: yes (Constitution Principle III -- red-green-refactor for all logger and adapter tests)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no incomplete task dependencies)
- **[Story]**: User story label (US1-US4); omitted in Setup, Foundational, and Polish phases
- Exact file paths included in each task description
- Task IDs within Phase 3 are not strictly ascending: all test tasks precede implementation tasks (TDD requirement)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create logger crate and wire into workspace

- [X] T001 Create crates/logger/src/lib.rs with module-level doc comment `//! Logger crate: reads InferredTransaction batches from Buffer2, persists as PendingTransaction.`
- [X] T002 Create crates/logger/Cargo.toml with [package] (name="logger", version="0.1.0", edition="2024") and [dependencies] using workspace = true for: domain, thiserror, log, rand, tokio; add [dev-dependencies] with uuid = { workspace = true } (uuid needed only for InferredTransaction test fixtures; mirrors modelizer pattern)
- [X] T003 Add "crates/logger" to workspace members and add `logger = { path = "crates/logger", version = "0.1.0" }` to [workspace.dependencies] in Cargo.toml (root)
- [X] T004 Add `logger = { workspace = true }` to [dependencies] in crates/fraud_detection/Cargo.toml

**Checkpoint**: `cargo build` succeeds with empty logger crate

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extend domain crate with PendingTransaction, StorageError, Buffer2Read, and Storage traits; all user story phases depend on these

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 Add PendingTransaction struct to crates/domain/src/lib.rs: `#[derive(Debug, Clone, PartialEq)]`, fields `inferred_transaction: InferredTransaction` and `prediction_confirmed: bool`, pub convenience method `id(&self) -> uuid::Uuid` delegating to `inferred_transaction.id()`
- [X] T006 Add StorageError enum to crates/domain/src/lib.rs: `#[derive(Debug, Clone, PartialEq)]`, `#[derive(thiserror::Error)]`, variants `CapacityExceeded { capacity: usize }` and `Unavailable` (unit)
- [X] T007 Add Buffer2Read AFIT trait to crates/domain/src/lib.rs: `#[expect(async_fn_in_trait, reason="no dyn dispatch needed; internal workspace only")]`, `async fn read_batch(&self, max: usize) -> Result<Vec<InferredTransaction>, BufferError>`
- [X] T008 Add Storage AFIT trait to crates/domain/src/lib.rs: `#[expect(async_fn_in_trait, reason="no dyn dispatch needed; internal workspace only")]`, `async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError>`
- [X] T009 Add `#[cfg(test)]` tests for PendingTransaction in crates/domain/src/lib.rs: construction with explicit fields, id() delegates through inferred_transaction.id(), prediction_confirmed accessible, Clone and PartialEq
- [X] T010 Add `#[cfg(test)]` tests for StorageError in crates/domain/src/lib.rs: CapacityExceeded stores correct capacity usize, Unavailable is unit variant, thiserror Display impls compile

**Checkpoint**: `cargo test --package domain` passes (9 existing + T009/T010 new tests)

---

## Phase 3: User Story 1+2+3 -- Core Logger Logic (Priority: P1) -- MVP

**Goal**: Complete log_once() covering all three P1 stories: read batches (US1), transform to PendingTransaction (US2), persist to Storage (US3)

**Why combined**: log_once() atomically implements US1+US2+US3 in a single method. Constitution Principle III requires ALL tests to be written and FAILING before T018 (the implementation). Task IDs are intentionally non-sequential within this phase to enforce that ordering.

**Independent Test**: All T012-T025 must fail (compile error or assertion) before T018 is written; after T018 all pass in a single `cargo test` run.

### Step 1 -- Write ALL failing tests first (T011-T025, T019)

> **Write every test below BEFORE writing any production code. Verify each FAILS.**

- [X] T011 [US1] Add MockBuffer2Read (Vec<InferredTransaction>, reads up to max, returns Err(BufferError::Closed) when empty+closed flag set) and MockStorage (Vec<PendingTransaction>, collects all writes, optional forced error) in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T012 [US1] Write failing test `test_log_once_batch_size_in_range`: N3_MAX=10, buffer with 100 items, call log_once() 20 times, assert each consumed batch size is in 1..=10 in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T013 [US1] Write failing test `test_log_once_batch_capped_at_available`: N3_MAX=20, buffer with 3 items, call log_once(), assert exactly 3 items consumed (capped at available count) in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T014 [US1] Write failing test `test_log_once_closed_empty_returns_error`: MockBuffer2Read closed+empty, call log_once(), assert Err(LoggerError::Read(BufferError::Closed)) in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T019 [US1] Write failing tests for LoggerConfig builder in crates/logger/src/lib.rs: `n3_max=5` builds Ok, `n3_max=0` returns Err(InvalidConfig), `poll_interval3` default is 100ms, `poll_interval3` setter overrides, `iterations` defaults to None, `seed` defaults to None
- [X] T020 [US2] Write failing test `test_transform_preserves_all_fields`: 5 InferredTransactions with known fields, call log_once(), verify 5 PendingTransactions in storage each with `prediction_confirmed=false` and `inferred_transaction == original` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T021 [US2] Write failing test `test_transform_predicted_fraud_true_preserved`: InferredTransaction with `predicted_fraud=true`, call log_once(), assert resulting PendingTransaction has `inferred_transaction.predicted_fraud=true` and `prediction_confirmed=false` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T022 [US2] Write failing test `test_transform_predicted_fraud_false_preserved`: InferredTransaction with `predicted_fraud=false`, call log_once(), assert both flags false and independent in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T023 [US3] Write failing test `test_persist_all_items`: MockBuffer2Read with 8 items, call log_once(), assert `mock_storage.items.len() == 8` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T024 [US3] Write failing test `test_persist_capacity_exceeded_propagates`: MockStorage configured to return `StorageError::CapacityExceeded { capacity: 0 }`, call log_once(), assert `Err(LoggerError::Write(StorageError::CapacityExceeded { capacity: 0 }))` in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T025 [US3] Write failing test `test_persist_unavailable_propagates`: MockStorage returns `StorageError::Unavailable`, call log_once(), assert `Err(LoggerError::Write(StorageError::Unavailable))` in `#[cfg(test)]` in crates/logger/src/lib.rs

### Step 2 -- Implement production code to make all tests pass (T015-T018)

- [X] T015 [US1] Add LoggerError to crates/logger/src/lib.rs: `#[derive(Debug, thiserror::Error)]`, variants `InvalidConfig { reason: String }`, `Read(#[from] BufferError)`, `Write(#[from] StorageError)`
- [X] T016 [US1] Add LoggerConfig struct and LoggerConfigBuilder to crates/logger/src/lib.rs: LoggerConfig fields `n3_max: usize`, `poll_interval3: Duration` (default 100ms), `iterations: Option<u64>`, `seed: Option<u64>`; `#[must_use] builder(n3_max: usize) -> LoggerConfigBuilder`; setter methods `poll_interval3`, `iterations`, `seed`; `#[must_use] build() -> Result<LoggerConfig, LoggerError>` rejecting `n3_max == 0` with `InvalidConfig`
- [X] T017 [US1] Add Logger struct to crates/logger/src/lib.rs: `#[derive(Debug)]`, fields `config: LoggerConfig` and `rng: RefCell<StdRng>`; `#[must_use] new(config: LoggerConfig) -> Self` seeding via `config.seed.map(StdRng::seed_from_u64).unwrap_or_else(StdRng::from_os_rng)`
- [X] T018 [US1] Implement `Logger::log_once<B: Buffer2Read, S: Storage>(&self, buf2: &B, storage: &S) -> Result<(), LoggerError>` in crates/logger/src/lib.rs: compute `n3 = rng.borrow_mut().random_range(1..=n3_max)`, call `read_batch(n3)`, map each `InferredTransaction` to `PendingTransaction { inferred_transaction: tx, prediction_confirmed: false }`, call `write_batch(batch)`

**Checkpoint**: `cargo test --package logger` passes ALL of T012-T014, T019-T025

---

## Phase 4: User Story 2 - Transform to PendingTransaction (Priority: P1)

**Goal**: Confirm transformation tests T020-T022 pass with log_once() from Phase 3

**No new tasks**: T020-T022 were written (red) in Phase 3 before T018. This phase is a verification checkpoint only.

**Checkpoint**: `cargo test --package logger` shows T020-T022 GREEN -- each InferredTransaction becomes a PendingTransaction with `prediction_confirmed=false` and all original fields intact

---

## Phase 5: User Story 3 - Persist to Storage (Priority: P1)

**Goal**: Confirm storage tests T023-T025 pass; add FR-011 batch_size logging

**Independent Test**: T023-T025 written (red) in Phase 3 verify all 8 items persisted and both StorageError variants propagate; T026 completes FR-011 coverage

- [X] T026 [US3] Add `log::debug!` in log_once() in crates/logger/src/lib.rs per FR-011 (batch size half): emit `"logger.log_once: batch_size={n3}"` at the start of each call -- FR-011 iteration number half is covered by T031

**Checkpoint**: `cargo test --package logger` passes T023-T025 (storage and error propagation tests)

---

## Phase 6: User Story 4 - Continuous Async Loop (Priority: P1)

**Goal**: run() loops read-transform-persist with poll_interval3 delay; stops gracefully returning Ok(()) when Buffer2 is closed+drained; optional iteration limit for testing

**Independent Test**: iterations=3, buffer with 30 items, run() returns Ok(()), MockStorage has exactly 3 batches worth of items; no-limit run stops cleanly after buffer closes

### Tests for User Story 4

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T027 [US4] Write failing test `test_run_iteration_limit`: Logger with `iterations=Some(3)`, MockBuffer2Read with 30 items, call run(), assert Ok(()) and MockStorage item count equals sum of 3 batches in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T028 [US4] Write failing test `test_run_stops_on_closed`: Logger with `iterations=None`, MockBuffer2Read pre-loaded with 5 batches then closed, call run(), assert Ok(()) in `#[cfg(test)]` in crates/logger/src/lib.rs
- [X] T029 [US4] Write failing test `test_run_zero_delay`: Logger with `poll_interval3=Duration::ZERO`, `iterations=Some(2)`, buffer with 20 items, call run(), assert Ok(()) completes without panic in `#[cfg(test)]` in crates/logger/src/lib.rs

### Implementation for User Story 4

- [X] T030 [US4] Implement `Logger::run<B: Buffer2Read, S: Storage>(&self, buf2: &B, storage: &S) -> Result<(), LoggerError>` in crates/logger/src/lib.rs: loop calling log_once(); on `Err(LoggerError::Read(BufferError::Closed))` break Ok(()); propagate all other errors; call `tokio::time::sleep(config.poll_interval3)`; break when iteration counter reaches `config.iterations`
- [X] T031 [US4] Add `log::info!` in run() per quickstart.md and FR-011 (iteration number half) in crates/logger/src/lib.rs: emit `"logger.batch.persisted: iteration={n}"` after each successful log_once() call
- [X] T032 [US4] Add `log::info!` in run() on graceful exit in crates/logger/src/lib.rs: emit `"logger.run.stopped: buffer closed after {n} iteration(s)"` matching quickstart.md format

**Checkpoint**: `cargo test --package logger` passes all logger tests (T012-T014, T019-T025, T027-T029)

---

## Phase 7: Adapters (ConcurrentBuffer2 + InMemoryStorage)

**Purpose**: Implement adapters for the concurrent pipeline; ConcurrentBuffer2 and InMemoryStorage groups are [P] (different files)

- [X] T033 Add `pub mod concurrent_buffer2;` and `pub mod in_memory_storage;` declarations to crates/fraud_detection/src/adapters/mod.rs

### ConcurrentBuffer2 Adapter

- [X] T034 [P] Create ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: `#[derive(Debug)]` struct with `inner: RefCell<ConcurrentBuffer2Inner { data: Vec<InferredTransaction>, closed: bool }>`, `#[must_use] new() -> Self`, `close(&self)` sets `inner.borrow_mut().closed = true`
- [X] T035 [P] Implement Buffer2 (write_batch) for ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: `write_batch(&self, batch: Vec<InferredTransaction>)` extends `inner.borrow_mut().data`
- [X] T036 [P] Implement Buffer2Read (read_batch) for ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: drain up to max items from front; if empty+open yield `tokio::task::yield_now()` and retry (borrow MUST drop before `.await`); if empty+closed return `Err(BufferError::Closed)`
- [X] T037 [P] Add `#[cfg(test)]` tests for ConcurrentBuffer2 in crates/fraud_detection/src/adapters/concurrent_buffer2.rs: write+read roundtrip, ordering preserved, close signals Closed when empty (mirror CB-T01..CB-T06 pattern from ConcurrentBuffer)

### InMemoryStorage Adapter

- [X] T038 [P] Create InMemoryStorage in crates/fraud_detection/src/adapters/in_memory_storage.rs: `#[derive(Debug)]` struct with `inner: RefCell<Vec<PendingTransaction>>` and `capacity: usize`, `#[must_use] new(capacity: usize) -> Self`; note: InMemoryStorage only returns CapacityExceeded; Unavailable is reserved for future adapters
- [X] T039 [P] Implement Storage (write_batch) for InMemoryStorage in crates/fraud_detection/src/adapters/in_memory_storage.rs: check `inner.borrow().len() + batch.len() > capacity` -> `Err(StorageError::CapacityExceeded { capacity: self.capacity })`, else `inner.borrow_mut().extend(batch)`
- [X] T040 [P] Add `#[cfg(test)]` tests for InMemoryStorage in crates/fraud_detection/src/adapters/in_memory_storage.rs: write_batch stores all items, CapacityExceeded returned with correct capacity when full, multiple batches accumulate

**Checkpoint**: `cargo test --package fraud_detection` passes including T037 and T040

---

## Phase 8: Pipeline Integration

**Purpose**: Wire Logger into main.rs; replace InMemoryBuffer2 with ConcurrentBuffer2; consumer arm closes buffer2 on completion to cascade Logger shutdown

- [X] T041 Replace InMemoryBuffer2 with `ConcurrentBuffer2::new()` as `buffer2` in crates/fraud_detection/src/main.rs; update Consumer call to pass `&buffer2` as Buffer2 write port; update imports; confirm main.rs uses `anyhow::Result` in return type (carried from feature 004); InMemoryBuffer2 is no longer used in binary -- retain as test-only infrastructure (it already follows the InMemoryBuffer pattern of test-only usage; add `#[expect(dead_code, reason="test-only adapter")]` on struct and new() if dead_code warnings appear)
- [X] T042 Instantiate `InMemoryStorage::new(usize::MAX)` and build `LoggerConfig` via `LoggerConfig::builder(n3_max).poll_interval3(...).build()?` and `Logger::new(logger_config)` in crates/fraud_detection/src/main.rs
- [X] T043 Add `logger.run(&buffer2, &storage)` as third arm in the `tokio::join!` pipeline alongside producer and consumer in crates/fraud_detection/src/main.rs
- [X] T044 Restructure pipeline async block in crates/fraud_detection/src/main.rs to implement R8 shutdown cascade; replace the current 2-arm join with: `let consumer_then_close = async { let r = consumer.run(&buffer1, &buffer2).await; buffer2.close(); r }; let pipeline = async { let (p, c, l) = tokio::join!(producer.run(&buffer1), consumer_then_close, logger.run(&buffer2, &storage)); p.and(c).and(l) };` -- on CTRL+C only buffer1.close() is needed (buffer2 cascade follows automatically)
- [X] T045 Verify CTRL+C shutdown path in crates/fraud_detection/src/main.rs: `tokio::select!` branch calls only `buffer1.close()`; buffer2 closes via cascade in consumer_then_close; all three join arms resolve cleanly

**Checkpoint**: `cargo run` with RUST_LOG=info shows `logger.batch.persisted` lines; CTRL+C shows all three stopped messages

---

## Phase 9: Polish & Cross-Cutting Concerns

- [X] T046 [P] Verify ms-rust compliance in crates/logger/src/lib.rs: `#[must_use]` on all constructors and builder methods, `#[derive(Debug)]` on all public types, `#[expect(..., reason="...")]` used instead of `#[allow]`, add compliance comment `// Rust guideline compliant 2026-02-16`
- [X] T047 [P] Add compliance comment `// Rust guideline compliant 2026-02-16` and verify `#[must_use]`, `#[derive(Debug)]`, `#[expect]` usage in crates/fraud_detection/src/adapters/concurrent_buffer2.rs
- [X] T048 [P] Add compliance comment `// Rust guideline compliant 2026-02-16` and verify `#[must_use]`, `#[derive(Debug)]`, `#[expect]` usage in crates/fraud_detection/src/adapters/in_memory_storage.rs
- [X] T049 Run `cargo test --workspace` and verify all 70+ tests pass (61 existing + new domain + logger + adapter tests); fix any compilation errors or assertion failures in affected source files

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies -- start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 -- BLOCKS all user story phases and adapter phases
- **Core Logic (Phase 3)**: Depends on Phase 2 -- all US1+US2+US3 tests written red, then log_once() turns them green
- **US2 checkpoint (Phase 4)**: Depends on Phase 3 -- verification only, no new tasks
- **US3 + FR-011 log (Phase 5)**: Depends on Phase 3 -- one implementation task (T026)
- **US4 (Phase 6)**: Depends on Phase 5 -- run() wraps the fully tested log_once()
- **Adapters (Phase 7)**: Depends on Phase 2 only -- independent of logger logic; can run parallel with Phases 3-6
- **Integration (Phase 8)**: Depends on Phases 6 and 7 -- both logger logic and adapters must be complete
- **Polish (Phase 9)**: Depends on Phase 8

### User Story Dependencies

- **US1+US2+US3 (Phase 3)**: First -- all three stories implemented atomically in log_once(); all tests written red before T018
- **US2 (Phase 4)**: After Phase 3 -- checkpoint confirms T020-T022 green
- **US3 (Phase 5)**: After Phase 3 -- T026 adds FR-011 batch_size logging
- **US4 (Phase 6)**: After Phase 5 -- run() loop adds iteration tracking (FR-011 iteration number half via T031)

### Parallel Opportunities

- T034-T037 (ConcurrentBuffer2) [P] vs T038-T040 (InMemoryStorage) [P]: different files
- T046 [P] / T047 [P] / T048 [P]: three different files
- Phase 7 (Adapters) can proceed in parallel with Phases 3-6 (different crates)

---

## Parallel Example: Adapters Phase

```bash
# Two agents can work in parallel (different .rs files):
Agent A: T033 (mod.rs), then T034, T035, T036, T037 (concurrent_buffer2.rs)
Agent B: T038, T039, T040 (in_memory_storage.rs)
```

---

## Implementation Strategy

### MVP First (US1+US2+US3 Core)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational domain types (T005-T010)
3. Complete Phase 3: Write ALL tests red (T011-T025, T019), then implement (T015-T018)
4. **STOP and VALIDATE**: `cargo test --package logger` -- all tests green
5. Proceed to Phase 5 (T026) and Phase 6 (US4) for run() loop

### Incremental Delivery

1. Setup + Foundational -> domain types ready
2. Phase 3 -> complete log_once() tested from all angles (US1+US2+US3 in one cycle)
3. Phase 6 -> complete run() loop
4. Adapters (Phase 7, overlap with Phases 3-6) -> production concurrent adapters
5. Integration (Phase 8) -> full end-to-end pipeline

### TDD Cycle for Phase 3 (Critical Path)

```
1. Write T011 (mocks) -- compile error expected
2. Write T012-T014, T019-T025 -- all compile errors
3. Add T015 (LoggerError) -- some compile errors resolve
4. Add T016 (LoggerConfig) -- more compile errors resolve
5. Add T017 (Logger struct) -- tests now compile but FAIL at runtime
6. Add T018 (log_once impl) -- ALL T012-T025 now PASS
```

---

## Notes

- [P] tasks operate on different files; no incomplete task dependencies within the [P] group
- [Story] label maps each task to its user story for traceability
- **TDD Phase 3**: task IDs T011-T025 and T019 are listed out of numeric order by design -- all tests precede all implementations
- **ConcurrentBuffer2 borrow pattern**: `let items = { let b = inner.borrow(); ... }; drop explicitly before yield_now().await` (same as ConcurrentBuffer)
- **LoggerError #[from]**: both `Read(#[from] BufferError)` and `Write(#[from] StorageError)` valid since each source type maps to exactly one variant (R7)
- **run() stop condition**: `Err(LoggerError::Read(BufferError::Closed))` -> break and return Ok(()); all other errors propagate
- **FR-011 split**: batch_size logged in log_once() via T026 (debug level); iteration number logged in run() via T031 (info level); together they fulfill FR-011
- **InMemoryStorage::Unavailable**: StorageError::Unavailable is part of the Storage trait contract but InMemoryStorage never returns it; Unavailable is tested via MockStorage in T025 only
- **Windows 11 / current_thread**: RefCell valid; no Sync required; tokio single-thread flavor
- **Buffer2 already exists** in domain crate (from feature 004); only Buffer2Read and Storage are new (T007, T008)
