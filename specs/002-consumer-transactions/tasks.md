# Tasks: Consumer Batch Processing

**Input**: Design documents from `/specs/002-consumer-transactions/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: TDD is NON-NEGOTIABLE per constitution (Principle III). Test tasks precede
implementation tasks in each phase. In Rust, compile failure = RED; test pass after
implementation = GREEN.

**Organization**: Tasks are grouped by user story to enable independent implementation
and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no shared-state dependencies)
- **[Story]**: Which user story this task belongs to (US1-US5 from spec.md)
- Exact file paths included in every description

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Cargo workspace extension and consumer crate initialization.

- [ ] T001 Add `consumer` to `members` list in `Cargo.toml`
- [ ] T002 Create `crates/consumer/Cargo.toml` (package name, edition 2024, workspace deps: domain, rand, thiserror, log, tokio with rt+macros features)
- [ ] T003 Create `crates/consumer/src/lib.rs` (crate-level doc comment, clippy deny/warn attributes matching producer pattern)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Domain type extensions and Consumer skeleton -- ALL user stories depend on these.

**CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T004 Add `InferredTransaction` struct (transaction: Transaction, predicted_fraud: bool, model_name: String, model_version: String; derives: Debug, Clone, PartialEq; `#[must_use]` id(&self) accessor delegating to self.transaction.id) to `crates/domain/src/lib.rs`
- [ ] T005 Add `ModelVersion` enum (N, NMinus1; derives: Debug, Clone, Copy, PartialEq, Eq) to `crates/domain/src/lib.rs`
- [ ] T006 Add `ModelizerError` enum (InferenceFailed { reason: String }, SwitchFailed { reason: String }; thiserror::Error Display) to `crates/domain/src/lib.rs`
- [ ] T007 Add `AlarmError` enum (DeliveryFailed { reason: String }; thiserror::Error Display) to `crates/domain/src/lib.rs`
- [ ] T008 Add `Buffer1Read` trait (async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError>; `#[expect(async_fn_in_trait, reason="...")]` on trait only) to `crates/domain/src/lib.rs`
- [ ] T009 Add `Buffer2` trait (async fn write_batch(&self, batch: Vec<InferredTransaction>) -> Result<(), BufferError>; `#[expect(async_fn_in_trait, reason="...")]` on trait only) to `crates/domain/src/lib.rs`
- [ ] T010 Add `Modelizer` trait (async fn infer(&self, batch: Vec<Transaction>) -> Result<Vec<InferredTransaction>, ModelizerError>; async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError>; `#[expect(async_fn_in_trait, reason="...")]` on trait only) to `crates/domain/src/lib.rs`
- [ ] T011 Add `Alarm` trait (async fn trigger(&self, transaction: &InferredTransaction) -> Result<(), AlarmError>; `#[expect(async_fn_in_trait, reason="...")]` on trait only) to `crates/domain/src/lib.rs`
- [ ] T012 Write domain unit tests for all new types and traits (inferred_transaction_fields, model_version_variants, modelizer_error_variants, alarm_error_variants, port_trait_struct_impl verifying AFIT compiles) in `crates/domain/src/lib.rs`
- [ ] T013 Add `ConsumerError` enum (InvalidConfig { reason: String }, Read(BufferError), Inference(ModelizerError), Write(BufferError); thiserror::Error; no `#[from]` on Read/Write -- same source type) to `crates/consumer/src/lib.rs`
- [ ] T014 Add `ConsumerConfig` struct (n2_max: usize, speed2: Duration, iterations: Option<u64>, seed: Option<u64>), `ConsumerConfigBuilder`, and `ConsumerConfig::builder(n2_max: usize)` factory with n2_max >= 1 validation returning Result<ConsumerConfig, ConsumerError> to `crates/consumer/src/lib.rs`
- [ ] T015 Write failing tests for ConsumerConfig validation (config_rejects_zero_n2_max, builder_defaults_speed2, builder_with_seed, builder_with_iterations) in `crates/consumer/src/lib.rs`
- [ ] T016 Add `Consumer` struct (config: ConsumerConfig, rng: RefCell<StdRng>) and `#[must_use] Consumer::new(config: ConsumerConfig) -> Self` (seeds StdRng from config.seed or OS RNG) to `crates/consumer/src/lib.rs`
- [ ] T017 Add `#[cfg(test)]` mock adapters (MockBuffer1Read with preloaded Vec<Transaction> + closed flag, MockModelizer with configurable predicted_fraud flag + call counter, MockAlarm with call counter + optional failure mode, MockBuffer2 with captured Vec<InferredTransaction> + optional error mode) to `crates/consumer/src/lib.rs`

**Checkpoint**: consumer crate compiles, domain has all 4 port traits and 4 new types

---

## Phase 3: User Story 1 - Read Batches from Buffer1 (Priority: P1) MVP

**Goal**: Consumer reads variable-size batches from Buffer1 respecting N2_MAX, operates
at speed2, and stops gracefully when Buffer1 is closed and drained.

**Independent Test**: Wire MockBuffer1Read preloaded with transactions; verify batch size
within [1, N2_MAX]; verify Consumer stops when MockBuffer1Read signals Closed.

### Tests for User Story 1

> **TDD**: Tests T018-T019 fail to compile until T020-T021 implement consume_once/run

- [ ] T018 [US1] Write failing tests for read behavior (batch_size_within_n2_max_range, batch_size_capped_by_available_data, seeded_batch_size_is_deterministic) using MockBuffer1Read in `crates/consumer/src/lib.rs`
- [ ] T019 [US1] Write failing tests for run loop (run_processes_n_iterations, run_stops_gracefully_on_closed) using MockBuffer1Read in `crates/consumer/src/lib.rs`

### Implementation for User Story 1

- [ ] T020 [US1] Implement `Consumer::consume_once<B1, M, A, B2>` (draw N2 from rng in [1, n2_max], call buf1.read_batch(N2), call modelizer.infer(batch), write to buf2.write_batch; alarm loop placeholder returns Ok(vec![])) in `crates/consumer/src/lib.rs`
- [ ] T021 [US1] Implement `Consumer::run<B1, M, A, B2>` (loop: call consume_once, sleep speed2, stop on ConsumerError::Read wrapping BufferError::Closed, propagate other errors) in `crates/consumer/src/lib.rs`

**Checkpoint**: US1 tests pass; Consumer reads batches and stops on closed signal

---

## Phase 4: User Story 2 - Send Batches to Modelizer for Inference (Priority: P1)

**Goal**: Consumer sends each read batch to Modelizer, receives InferredTransactions
enriched with predicted_fraud/model_name/model_version, propagates ModelizerError.

**Independent Test**: Wire MockModelizer configured to mark all transactions fraudulent;
verify returned InferredTransactions carry correct enrichment fields; verify
ModelizerError surfaces as ConsumerError::Inference.

### Tests for User Story 2

- [ ] T022 [US2] Write failing tests for Modelizer interaction (consume_once_sends_full_batch_to_modelizer, inference_error_propagates_as_consumer_error_inference) using MockModelizer in `crates/consumer/src/lib.rs`
- [ ] T023 [US2] Write test verifying InferredTransaction enrichment fields (predicted_fraud, model_name, model_version from MockModelizer) survive through consume_once pipeline in `crates/consumer/src/lib.rs`

### Implementation for User Story 2

- [ ] T024 [US2] Verify/fix consume_once Modelizer integration (ModelizerError mapped to ConsumerError::Inference via manual .map_err(), not #[from]) in `crates/consumer/src/lib.rs`

**Checkpoint**: US2 tests pass; Modelizer errors propagate correctly

---

## Phase 5: User Story 4 - Write Processed Batches to Buffer2 (Priority: P1)

**Goal**: All inferred transactions (fraudulent and legitimate) are written to Buffer2
after processing; Buffer2 errors propagate as ConsumerError::Write.

**Independent Test**: Wire MockBuffer2; run consume_once; verify MockBuffer2 captured all
InferredTransactions from MockModelizer; verify BufferError::Full/Closed map to
ConsumerError::Write.

### Tests for User Story 4

- [ ] T025 [US4] Write failing tests for Buffer2 write (all_inferred_tx_written_to_buf2_regardless_of_fraud, buf2_full_propagates_as_consumer_error_write, buf2_closed_propagates_as_consumer_error_write) using MockBuffer2 in `crates/consumer/src/lib.rs`

### Implementation for User Story 4

- [ ] T026 [US4] Verify/fix consume_once Buffer2 integration (BufferError from write_batch mapped to ConsumerError::Write via manual .map_err(); all InferredTransactions passed regardless of predicted_fraud) in `crates/consumer/src/lib.rs`

**Checkpoint**: US4 tests pass; Buffer2 receives complete inferred batch

---

## Phase 6: User Story 3 - Generate Alarms for Fraudulent Transactions (Priority: P2)

**Goal**: Consumer triggers exactly one alarm per fraudulent transaction per batch;
uses best-effort delivery (all alarms attempted); collects AlarmError into Ok(vec);
always writes to Buffer2 regardless of alarm failures.

**Independent Test**: Wire MockAlarm with call counter; configure MockModelizer with N
fraudulent transactions; run consume_once; verify MockAlarm.call_count == N;
verify Buffer2 written even when alarms fail.

### Tests for User Story 3

> **TDD**: T027-T028 fail against Phase 3 placeholder (Ok(vec![])) until T029 adds alarm loop

- [ ] T027 [US3] Write failing tests for alarm count (exactly_n_alarms_for_n_fraudulent_tx, no_alarms_for_zero_fraudulent_tx, zero_alarms_when_all_legitimate) using MockAlarm in `crates/consumer/src/lib.rs`
- [ ] T028 [US3] Write failing tests for best-effort alarm delivery (all_alarms_attempted_on_partial_failure, alarm_failures_returned_in_ok_vec, buf2_write_not_blocked_by_alarm_failure) using MockAlarm with failure mode in `crates/consumer/src/lib.rs`

### Implementation for User Story 3

- [ ] T029 [US3] Extend `Consumer::consume_once` alarm loop (iterate inferred batch; for predicted_fraud==true call alarm.trigger, collect Err into Vec<AlarmError>; write buf2.write_batch regardless; return Ok(alarm_errors)) in `crates/consumer/src/lib.rs`

**Checkpoint**: US3 tests pass; alarms triggered best-effort, failures collected

---

## Phase 7: User Story 5 - Switch Model Version at Runtime (Priority: P2)

**Goal**: Consumer delegates version switch to Modelizer port; Consumer owns no version
state; ModelizerError from switch_version surfaces as ConsumerError::Inference.

**Independent Test**: Wire MockModelizer tracking last switch_version call; call
Consumer::switch_model_version with ModelVersion::NMinus1; verify
MockModelizer.last_version == NMinus1.

### Tests for User Story 5

- [ ] T030 [US5] Write failing tests for model version switch (switch_to_n_minus1_calls_modelizer_switch_version, switch_to_n_calls_modelizer_switch_version, switch_error_maps_to_consumer_error_inference, default_model_version_is_n -- verify MockModelizer starts with version N and infer is called with no prior switch_version call) using MockModelizer in `crates/consumer/src/lib.rs`

### Implementation for User Story 5

- [ ] T031 [US5] Implement `Consumer::switch_model_version<M: Modelizer>(&self, modelizer: &M, version: ModelVersion) -> Result<(), ConsumerError>` (call modelizer.switch_version(version).await; map ModelizerError to ConsumerError::Inference) in `crates/consumer/src/lib.rs`

**Checkpoint**: US5 tests pass; model version switching delegates to Modelizer

---

## Phase 8: Adapters and Binary Wiring

**Purpose**: Concrete port adapters in the binary crate and Consumer integration in main.

> T032-T035 and T037 can run in parallel after Phase 2 (domain traits available).
> T036 depends on T032-T035 (modules must exist before mod.rs declares them).
> T038 depends on T036 + T037 (adapters + consumer dep must be present).

- [ ] T032 [P] Update `InMemoryBuffer` to implement `Buffer1Read` (add `async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError>` draining up to max items; return Err(BufferError::Closed) when closed and empty) in `crates/fraud_detection/src/adapters/in_memory_buffer.rs`
- [ ] T033 [P] Create `InMemoryBuffer2` struct implementing `Buffer2` trait (wraps `RefCell<VecDeque<InferredTransaction>>`; write_batch appends all; returns BufferError::Full when over capacity; #[must_use] new) in `crates/fraud_detection/src/adapters/in_memory_buffer2.rs`
- [ ] T034 [P] Create `DemoModelizer` struct implementing `Modelizer` trait (marks all transactions fraudulent or legitimate per config bool; populates model_name="DINN", model_version="v1"; initializes current_version=ModelVersion::N (FR-009); logs infer and switch_version calls) in `crates/fraud_detection/src/adapters/demo_modelizer.rs`
- [ ] T035 [P] Create `LogAlarm` struct implementing `Alarm` trait (logs fraud alert via `log::warn!` with transaction id; returns Ok(()); #[must_use] new) in `crates/fraud_detection/src/adapters/log_alarm.rs`
- [ ] T036 Update `crates/fraud_detection/src/adapters/mod.rs` to declare `in_memory_buffer2`, `demo_modelizer`, and `log_alarm` submodules (pub use types)
- [ ] T037 [P] Add `consumer` workspace dependency to `crates/fraud_detection/Cargo.toml`
- [ ] T038 Update `crates/fraud_detection/src/main.rs` to construct `ConsumerConfig`, `Consumer`, and call `consumer.run` with `&in_memory_buffer` (Buffer1Read), `&demo_modelizer`, `&log_alarm`, `&in_memory_buffer2`; `run` returns `Result<(), ConsumerError>` -- alarm errors are logged inside `run` per-iteration when `consume_once` returns `Ok(alarm_errors)` with non-empty vec; `main.rs` only handles the outer `Result`

**Checkpoint**: `cargo build --release` succeeds; full pipeline runs end-to-end

---

## Phase 9: Polish and Cross-Cutting Concerns

- [ ] T039 Run `cargo test --workspace` and fix any failing tests
- [ ] T040 Run `cargo clippy --workspace --all-targets` and fix all lint warnings (check #[must_use], doc comment format < 15 words summary, Ref<'_> lifetimes, #[expect] not #[allow])
- [ ] T041 Add compliance comment `// Rust guideline compliant 2026-02-22` to `crates/consumer/src/lib.rs` and new adapter files; verify quickstart.md build/run steps execute successfully

---

## Dependencies and Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No deps -- start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 -- BLOCKS all user story phases
- **US1 (Phase 3)**: Depends on Phase 2 -- entry point, no story deps
- **US2 (Phase 4)**: Depends on Phase 3 -- consume_once must exist and read from Buffer1
- **US4 (Phase 5)**: Depends on Phase 4 -- inferred transactions must flow through Modelizer
- **US3 (Phase 6)**: Depends on Phase 5 -- alarm loop extends the established pipeline
- **US5 (Phase 7)**: Depends on Phase 2 -- independent of US1-4; needs domain traits + Consumer struct
- **Adapters (Phase 8)**: T032-T035, T037 can start after Phase 2; T036 needs T032-T035; T038 needs Phase 7 + T036 + T037
- **Polish (Phase 9)**: Depends on Phase 8

### User Story Dependencies

- **US1 (P1)**: After Phase 2 -- no story deps
- **US2 (P1)**: After US1 -- consume_once must read before it can send to Modelizer
- **US4 (P1)**: After US2 -- inferred batch must exist before Buffer2 write
- **US3 (P2)**: After US4 -- alarm loop inserted after core pipeline is stable
- **US5 (P2)**: After Phase 2 -- switch_model_version is independent of consume_once

### Within Each Phase

- TDD order: write failing tests FIRST (RED), then implement (GREEN)
- Rust RED = compile error when method/type does not exist yet
- All tasks in a phase modify the same file -- execute sequentially

### Parallel Opportunities

- **Phase 8 adapters**: T032, T033, T034, T035, T037 -- all different files, run in parallel
- **Phase 3-7 phases**: After Phase 2, US5 (Phase 7) can be worked in parallel with US1-US4 pipeline

---

## Parallel Example: Phase 8 Adapters

```bash
# All 5 tasks use different files -- run in parallel after Phase 2:
Task T032: Update in_memory_buffer.rs -- add Buffer1Read impl
Task T033: Create in_memory_buffer2.rs -- Buffer2 adapter
Task T034: Create mock_modelizer.rs -- Modelizer adapter
Task T035: Create log_alarm.rs -- Alarm adapter
Task T037: Update fraud_detection/Cargo.toml -- add consumer dep
```

---

## Implementation Strategy

### MVP First (P1 User Stories: US1, US2, US4)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL -- blocks all stories)
3. Complete Phase 3: US1 -- Consumer reads from Buffer1
4. Complete Phase 4: US2 -- Consumer sends to Modelizer
5. Complete Phase 5: US4 -- Consumer writes to Buffer2
6. **STOP and VALIDATE**: `cargo test --workspace` -- core pipeline green
7. Wire adapters (Phase 8) and run demo

### Incremental Delivery

1. Setup + Foundational -> Consumer crate skeleton ready
2. US1 + US2 + US4 -> Core pipeline (Buffer1 -> Modelizer -> Buffer2)
3. US3 -> Add best-effort alarm delivery
4. US5 -> Add runtime model version switching
5. Phase 8 -> Full end-to-end binary demo
6. Phase 9 -> Clippy clean, compliance comments, quickstart verified

---

## Notes

- [P] tasks use different files -- no shared-state conflicts between them
- [Story] labels trace each task to spec.md user story acceptance scenarios
- `consume_once` grows incrementally: T020 (US1 stub) -> T024 (US2 fix) -> T026 (US4 fix) -> T029 (US3 alarm loop)
- Phase 8 adapters implement port traits in the binary crate -- Consumer never imports concrete adapters (hexagonal architecture)
- Consumer uses 4 generic type params per call, no dyn dispatch (per research.md Decision 3)
- ConsumerError::Read and ConsumerError::Write both wrap BufferError -- no #[from], use manual .map_err() (research.md Decision 6)
