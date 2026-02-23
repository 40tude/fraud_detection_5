# Tasks: Concurrent Pipeline

**Input**: Design documents from `/specs/004-concurrent-pipeline/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: TDD is NON-NEGOTIABLE per constitution (Principle III). ConcurrentBuffer tests
are written before implementation (Red-Green-Refactor). Test tasks are included in Phase 2.

**Organization**: Tasks organized by user story. ConcurrentBuffer adapter is Foundational
(Phase 2) -- it blocks all three user stories.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Exact file paths included in all descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the tokio `signal` feature required for FR-004 (`tokio::signal::ctrl_c`).

- [ ] T001 Add `"signal"` to workspace tokio features in `Cargo.toml`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Implement `ConcurrentBuffer` -- the only new adapter in this feature. All three
user stories depend on it. TDD order: write tests first (Red), then implement (Green).

**WARNING**: No user story work begins until this phase is complete.

- [ ] T002 Write ConcurrentBuffer test module (CB-T01 to CB-T06: write/read roundtrip, empty+closed error, write-to-closed error, drain from front, idempotent close, yield-unblocks scenario) in `crates/fraud_detection/src/adapters/concurrent_buffer.rs` (TDD Red: create file with test stubs only, no impl)
- [ ] T003 Implement `ConcurrentBufferInner { data: Vec<Transaction>, closed: bool }` wrapped in `RefCell`, outer `ConcurrentBuffer` struct, `new()` (#[must_use]), `close()` (idempotent, sets closed=true) in `crates/fraud_detection/src/adapters/concurrent_buffer.rs`
- [ ] T004 Implement `Buffer1` trait (`write_batch`: appends if open, returns `Err(BufferError::Closed)` if closed) for `ConcurrentBuffer` in `crates/fraud_detection/src/adapters/concurrent_buffer.rs`
- [ ] T005 Implement `Buffer1Read` trait (`read_batch`: returns data via `Vec::drain(..count)` if available; drops borrow then calls `tokio::task::yield_now().await` and retries if empty+open; returns `Err(BufferError::Closed)` if empty+closed) for `ConcurrentBuffer` in `crates/fraud_detection/src/adapters/concurrent_buffer.rs`
- [ ] T006 Register `pub mod concurrent_buffer` in `crates/fraud_detection/src/adapters/mod.rs`; run `cargo test --workspace` -- all 6 CB tests green, all 55 prior tests still pass

**Checkpoint**: ConcurrentBuffer fully tested and registered -- user story implementation can now begin

---

## Phase 3: User Story 1 -- Concurrent Operation (Priority: P1) -- MVP

**Goal**: Producer and Consumer run simultaneously via `tokio::join!`, sharing a
`ConcurrentBuffer`. Neither stage waits for the other to finish.

**Independent Test**: Set `iterations(10)` in `ProducerConfig` in `main.rs`; run with
`$env:RUST_LOG='info'; cargo run` -- observe Producer and Consumer log lines interleaving;
app exits cleanly on its own with no error.

### Implementation for User Story 1

- [ ] T007 [US1] Replace `InMemoryBuffer` with `ConcurrentBuffer` in `crates/fraud_detection/src/main.rs`; wrap producer call in `let producer_task = async { let r = producer.run(&buffer1).await; buffer1.close(); r };` to propagate finite-mode shutdown via buffer closure
- [ ] T008 [US1] Replace sequential `producer.run()`/`consumer.run()` calls with `let (p, c) = tokio::join!(producer_task, consumer.run(&buffer1, &modelizer, &alarm)).await; p.context("producer failed")?; c.context("consumer failed")?;` in `crates/fraud_detection/src/main.rs`
- [ ] T009 [US1] Set `ConsumerConfig` `speed2` to `Duration::from_millis(25)` (ensures Consumer yields regularly so Producer gets CPU time) in `crates/fraud_detection/src/main.rs`

**Checkpoint**: `tokio::join!` wired; app exits cleanly after finite Producer run (set iterations(10) to test); US1 scenarios verified manually

---

## Phase 4: User Story 2 -- Graceful Shutdown via CTRL+C (Priority: P2)

**Goal**: `tokio::select!` races `ctrl_c()` against `join!()`; on CTRL+C, `buffer1.close()`
is called, both tasks detect `BufferError::Closed`, app exits with code 0 in under 5 seconds.

**Independent Test**: Run app with no iteration limit (`$env:RUST_LOG='info'; cargo run`);
wait for several interleaved cycles; press CTRL+C; verify clean exit (exit code 0, no panic,
no hang, shutdown message logged at info level).

### Implementation for User Story 2

- [ ] T010 [US2] Wrap `tokio::join!` in `tokio::select!` with two arms: `_ = tokio::signal::ctrl_c() => { log::info!("main.shutdown: ctrl_c received, closing buffer"); buffer1.close(); }` and `(p, c) = tokio::join!(producer_task, consumer.run(...)) => { p.context(...)?; c.context(...)?; }` in `crates/fraud_detection/src/main.rs`
- [ ] T011 [US2] Set `ProducerConfig` `iterations` to `None` (infinite mode default) in `crates/fraud_detection/src/main.rs` so CTRL+C is the only way to stop the default run

**Checkpoint**: Running app requires CTRL+C to stop; press it, verify clean exit (exit code 0, shutdown log line present); US2 scenarios verified manually

---

## Phase 5: User Story 3 -- Demo / Test Mode with Finite Iterations (Priority: P3)

**Goal**: FR-009 iteration-limit log messages added to Producer and Consumer; all 55 existing
automated tests pass with no modifications to test code (FR-008 / SC-003).

**Independent Test**: `cargo test --workspace` passes (55 tests, 0 failures). Manually set
`iterations(10)` in `main.rs`, run -- app exits automatically with interleaved log output.

### Implementation for User Story 3

- [ ] T012 [P] [US3] Add `log::info!("producer.run.stopped: iteration limit reached")` in the iteration-limit return branch of `Producer::run()` in `crates/producer/src/lib.rs` (FR-009)
- [ ] T013 [P] [US3] Add `log::info!("consumer.run.stopped: iteration limit reached")` in the iteration-limit return branch of `Consumer::run()` in `crates/consumer/src/lib.rs` (FR-009)
- [ ] T014 [US3] Run `cargo test --workspace` and confirm all 55 tests pass; fix any regression before proceeding (FR-008 / SC-003)

**Checkpoint**: All 55 tests green; FR-009 logs visible in demo run; US3 scenarios verified manually

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: ms-rust compliance review and quickstart validation.

- [ ] T015 [P] Review changed files for ms-rust compliance (`#[derive(Debug)]`, `#[must_use]` on constructors, `#[expect(..., reason="...")]` not `#[allow]`, doc comments, compliance comment) in `crates/fraud_detection/src/adapters/concurrent_buffer.rs`, `crates/producer/src/lib.rs`, `crates/consumer/src/lib.rs`, `crates/fraud_detection/src/main.rs`
- [ ] T016 Run quickstart.md validation: (1) `$env:RUST_LOG='info'; cargo run` in infinite mode -- observe interleaved logs, press CTRL+C, verify exit 0; (2) set `iterations(10)` in `main.rs`, rerun -- verify auto-exit with no manual intervention; (3) `cargo test --workspace` -- confirm 55 tests pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies -- start immediately
- **Foundational (Phase 2)**: Depends on T001 -- BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 completion (T002-T006)
- **US2 (Phase 4)**: Depends on US1 (T007-T009) -- `select!` wraps the `join!` established in US1
- **US3 (Phase 5)**: T012/T013 can run in parallel from Phase 2 onwards (different crates, no shared deps); T014 depends on T012+T013
- **Polish (Phase 6)**: Depends on all user stories complete (T001-T014)

### User Story Dependencies

- **US1 (P1)**: Starts after Phase 2 -- no dependency on US2 or US3
- **US2 (P2)**: Depends on US1 (builds `select!` around `join!` from T008)
- **US3 (P3)**: T012/T013 fully independent (different files); T014 must follow T012+T013

### Within Each User Story

- T002 (tests) MUST exist and fail before T003-T005 (Red-Green-Refactor, NON-NEGOTIABLE)
- T003 before T004/T005 (struct must exist before trait impls)
- T004 and T005 are in the same file -- sequential
- T007 before T008 (ConcurrentBuffer + wrapper must exist before join! call)
- T010 before T011 (select! must exist before setting infinite mode)
- T012 and T013 are fully parallel (different crates)

---

## Parallel Opportunities

### Foundational Phase (ConcurrentBuffer -- same file, sequential)

```
T002 -> T003 -> T004 -> T005 -> T006
```

### User Story 3 (different crates, parallel)

```
# T012 and T013 run in parallel:
T012: crates/producer/src/lib.rs  -- FR-009 log
T013: crates/consumer/src/lib.rs  -- FR-009 log

# Then T014:
T014: cargo test --workspace
```

### Polish (T015 independent of T016)

```
T015: compliance review (read-only analysis)
T016: quickstart validation (run + observe)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. T001 -- Add tokio signal feature
2. T002-T006 -- ConcurrentBuffer (TDD: Red then Green)
3. T007-T009 -- Concurrent join! in main.rs
4. **STOP and VALIDATE**: Set `iterations(10)`, run, observe interleaved logs, clean exit
5. US1 complete -- pipeline is concurrent

### Incremental Delivery

1. T001 -> setup done
2. T002-T006 -> ConcurrentBuffer tested, 55 prior tests still green
3. T007-T009 -> concurrent pipeline running (US1 MVP)
4. T010-T011 -> CTRL+C shutdown (US2)
5. T012-T014 -> FR-009 logs + full regression check (US3)
6. T015-T016 -> polish + quickstart validation

---

## Notes

- [P] = different files, no cross-task deps -- safe to run concurrently
- TDD: T002 must be committed (file with failing tests) before T003 starts -- constitution Principle III
- `ConcurrentBuffer` is the ONLY new type; no domain changes (FR-007)
- All 55 existing tests must stay green at every checkpoint (FR-008)
- ms-rust: `#[derive(Debug)]` on all public types, `#[must_use]` on `new()`, `#[expect(..., reason="...")]` over `#[allow]`
- Windows PowerShell: use `$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG` (not Unix export syntax)
