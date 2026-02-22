# Tasks: Producer -- Transaction Generation

**Input**: Design documents from `/specs/001-producer-transactions/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/buffer1-trait.md, quickstart.md

**Tests**: Included -- TDD mandated by spec.md (SC-004) and plan.md (III: TDD non-negotiable).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Exact file paths included in every description

---

## Phase 1: Setup (Cargo Workspace)

**Purpose**: Initialize workspace structure, dependency versions, and lint configuration.

- [ ] T001 Create root Cargo.toml with [workspace] members (domain, producer, fraud_detection), [workspace.dependencies] (uuid 1, thiserror 2, log 0.4, rand 0.9, tokio 1 with rt+macros+time, env_logger 0.11, anyhow 1), and [workspace.lints.clippy] / [workspace.lints.rust] per ms-rust M-STATIC-VERIFICATION in Cargo.toml
- [ ] T002 [P] Create crates/domain/Cargo.toml with [package] metadata, [lints] workspace = true, and [dependencies] referencing workspace deps (uuid, thiserror, log, tokio) in crates/domain/Cargo.toml
- [ ] T003 [P] Create crates/producer/Cargo.toml with [package] metadata, [lints] workspace = true, and [dependencies] referencing workspace deps (domain path dep, rand, thiserror, log, tokio) in crates/producer/Cargo.toml
- [ ] T004 [P] Create crates/fraud_detection/Cargo.toml with [[bin]] entry, [lints] workspace = true, and [dependencies] referencing workspace deps (domain, producer path deps, anyhow, env_logger, log, tokio) in crates/fraud_detection/Cargo.toml
- [ ] T005 Create .cargo/config.toml with [build] target-dir redirect and [target.x86_64-pc-windows-msvc] rustflags = ["-C", "target-cpu=native"] in .cargo/config.toml

**Checkpoint**: `cargo build` compiles an empty workspace without errors.

---

## Phase 2: Foundational (domain crate)

**Purpose**: Shared domain types that ALL user stories depend on. No user story work can begin until this phase is complete.

**CRITICAL**: These are shared across all pipeline components (FR-003). Implement before any producer logic.

- [ ] T006 Write failing tests transaction_fields and buffer_error_variants in crates/domain/src/lib.rs (add empty type stubs so tests compile but assertions fail -- TDD red step)
- [ ] T007 Write failing test buffer1_impl in crates/domain/src/lib.rs (define a local TestBuffer struct implementing a stub Buffer1 trait; test that write_batch stores transactions -- TDD red step)
- [ ] T008 Implement Transaction struct with #[derive(Debug, Clone, PartialEq)] and pub fields: id: uuid::Uuid, amount: f64, last_name: String in crates/domain/src/lib.rs
- [ ] T009 Implement BufferError enum with #[derive(Debug, Clone, PartialEq)] and thiserror::Error (Full { capacity: usize }, Closed variants with #[error] messages) in crates/domain/src/lib.rs
- [ ] T010 Implement Buffer1 trait with #[expect(async_fn_in_trait, reason = "no dyn dispatch; internal workspace only")] and async fn write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError> in crates/domain/src/lib.rs

**Checkpoint**: `cargo test -p domain` passes 3 tests (transaction_fields, buffer_error_variants, buffer1_impl).

---

## Phase 3: User Story 1 -- Generate a Single Batch (Priority: P1) MVP

**Goal**: Producer generates random transaction batches of correct size with valid UUID, amount, and last_name fields.

**Independent Test**: Call `Producer::generate_batch` once via a configured Producer; assert batch size is in [1, N1_MAX] and every transaction has a parseable UUID, an amount in [0.01, 10_000.00], and a non-empty last_name.

### Tests for User Story 1 (TDD: write first, ensure FAIL before implementation)

- [ ] T011 [US1] Write failing test config_rejects_zero (ProducerConfig::builder(0).build() returns Err(ProducerError::InvalidConfig)) in crates/producer/src/lib.rs
- [ ] T012 [P] [US1] Write failing test batch_size_bounds (seed-fixed Producer with n1_max=10, generate 100 batches, assert all sizes in [1, 10]) in crates/producer/src/lib.rs
- [ ] T013 [P] [US1] Write failing test tx_fields_valid (generate one batch, assert id parses as UUID, amount in [0.01, 10_000.00], last_name non-empty) in crates/producer/src/lib.rs

### Implementation for User Story 1

- [ ] T014 [US1] Implement ProducerError with thiserror::Error (#[derive(Debug)], InvalidConfig { reason: String }, Buffer { #[from] source: BufferError }) in crates/producer/src/lib.rs
- [ ] T015 [US1] Implement ProducerConfigBuilder (fields: n1_max, speed1, iterations, seed) and ProducerConfig::builder(n1_max: usize) -> ProducerConfigBuilder with defaults (speed1 = 100ms, iterations = None, seed = None); build() validates n1_max >= 1 and returns Result<ProducerConfig, ProducerError> in crates/producer/src/lib.rs
- [ ] T016 [US1] Implement Producer<B: Buffer1> with StdRng initialized from seed (StdRng::seed_from_u64) or StdRng::from_os_rng(), const LAST_NAMES: [&str; N] array, and generate_batch (rng.random_range(1..=n1_max) size, UUID via Builder::from_random_bytes + rng.fill_bytes, amount as rng.random_range(1..=1_000_000) as f64 / 100.0, random last_name index) in crates/producer/src/lib.rs

**Checkpoint**: `cargo test -p producer` passes config_rejects_zero, batch_size_bounds, tx_fields_valid. US1 is fully testable in isolation.

---

## Phase 4: User Story 2 -- Write Batches to Buffer1 via Port (Priority: P1)

**Goal**: Producer writes generated batches to Buffer1 exclusively through the trait port; InMemoryBuffer adapter stores batches retrievable by tests.

**Independent Test**: Inject InMemoryBuffer (implements Buffer1), call produce_once, read back stored transactions and verify UUIDs match exactly.

### Tests for User Story 2 (TDD: write first, ensure FAIL before implementation)

- [ ] T017 [US2] Write failing test produce_and_write (create InMemoryBuffer, build Producer with seed, call produce_once, assert buffer.transactions().len() == batch size) in crates/producer/src/lib.rs
- [ ] T018 [P] [US2] Write failing test in_memory_buffer_stores_batch (construct 5 Transactions with known UUIDs, call write_batch, assert stored UUIDs match in order) in crates/fraud_detection/src/adapters/in_memory_buffer.rs

### Implementation for User Story 2

- [ ] T019 [US2] Create crates/fraud_detection/src/adapters/mod.rs with pub mod in_memory_buffer; in crates/fraud_detection/src/adapters/mod.rs
- [ ] T020 [US2] Implement InMemoryBuffer struct (#[derive(Debug)]) with RefCell<Vec<Transaction>> inner field, transactions() -> Ref<Vec<Transaction>> accessor, and impl Buffer1 (write_batch extends inner vec; never returns Full or Closed for PoC) in crates/fraud_detection/src/adapters/in_memory_buffer.rs
- [ ] T021 [US2] Implement Producer::produce_once(&self, buffer: &B) -> Result<(), ProducerError> (calls self.generate_batch() then buffer.write_batch(batch).await, mapping BufferError via From) in crates/producer/src/lib.rs
- [ ] T022 [US2] Implement crates/fraud_detection/src/main.rs with #[tokio::main], env_logger::init(), ProducerConfig::builder(100).build()?, InMemoryBuffer::new(), Producer::new(config), producer.produce_once(&buffer).await?, log::info! with batch count in crates/fraud_detection/src/main.rs

**Checkpoint**: `cargo test -p producer` passes produce_and_write; `cargo test -p fraud_detection` passes in_memory_buffer_stores_batch. `cargo run` produces a logged batch.

---

## Phase 5: User Story 3 -- Continuous Production at speed1 (Priority: P2)

**Goal**: Producer runs in a loop for a configurable number of iterations (or indefinitely), sleeping speed1 between each batch; handles Closed termination cleanly.

**Independent Test**: Start Producer with iterations = Some(5) and a zero-duration speed1; assert buffer received exactly 5 batches each of valid size. Start with a buffer that returns Closed; assert Producer::run returns Ok(()).

### Tests for User Story 3 (TDD: write first, ensure FAIL before implementation)

- [ ] T023 [US3] Write failing test run_n_iterations (Producer::run with iterations=Some(5), speed1=0ms, InMemoryBuffer; assert 5 batches written and total tx count in expected bounds) in crates/producer/src/lib.rs
- [ ] T024 [P] [US3] Write failing test run_stops_on_closed (buffer returns BufferError::Closed on first write; Producer::run returns Ok(())) in crates/producer/src/lib.rs
- [ ] T025 [P] [US3] Write failing test run_propagates_full (buffer returns BufferError::Full on first write; Producer::run returns Err(ProducerError::Buffer { source: BufferError::Full { .. } })) in crates/producer/src/lib.rs

### Implementation for User Story 3

- [ ] T026 [US3] Implement Producer::run(&self, buffer: &B) -> Result<(), ProducerError> with loop: generate_batch, write via produce_once, match on ProducerError::Buffer { source: BufferError::Closed } -> break Ok(()), other errors -> return Err in crates/producer/src/lib.rs
- [ ] T027 [US3] Add iterations handling to Producer::run: if config.iterations == Some(n) track count and break after n batches; if None loop indefinitely until Closed or error in crates/producer/src/lib.rs
- [ ] T028 [US3] Add tokio::time::sleep(self.config.speed1) after each successful write in Producer::run in crates/producer/src/lib.rs
- [ ] T029 [US3] Update crates/fraud_detection/src/main.rs to call producer.run(&buffer).await with anyhow::Context for error propagation; add RUST_LOG=info logging guidance comment in crates/fraud_detection/src/main.rs

**Checkpoint**: `cargo test -p producer` passes run_n_iterations, run_stops_on_closed, run_propagates_full. Total: 7 producer tests green.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: ms-rust compliance, lint gate, and final validation across all crates.

- [ ] T030 [P] Apply ms-rust compliance across workspace: #[must_use] on ProducerConfig::builder and ProducerConfigBuilder::build; #[derive(Debug)] on all public types; replace any #[allow] with #[expect(..., reason = "...")] in crates/domain/src/lib.rs, crates/producer/src/lib.rs, crates/fraud_detection/src/adapters/in_memory_buffer.rs
- [ ] T031 [P] Run cargo clippy --workspace -- -D warnings and fix all violations; add // Rust guideline compliant YYYY-MM-DD comment to each source file in all workspace crates
- [ ] T032 Run cargo test --workspace and confirm all 11 tests pass (domain: 3, producer: 7, fraud_detection: 1)
- [ ] T033 [P] Run cargo build --release and verify zero warnings
- [ ] T034 Validate quickstart.md: run RUST_LOG=info cargo run and confirm log output shows iteration count and batch size; run RUST_LOG=debug cargo test and confirm per-transaction debug output in crates/fraud_detection/

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies -- start immediately
- **Foundational (Phase 2)**: Requires Phase 1 -- BLOCKS all user stories
- **User Story 1 (Phase 3)**: Requires Phase 2; no dependencies on US2 or US3
- **User Story 2 (Phase 4)**: Requires Phase 2 and US1 (produce_once depends on generate_batch)
- **User Story 3 (Phase 5)**: Requires US2 (run calls produce_once)
- **Polish (Phase 6)**: Requires all user story phases complete

### User Story Dependencies

- **US1 (P1)**: Can start after Phase 2 -- foundational only; generates transactions independently
- **US2 (P1)**: Requires US1 (produce_once = generate_batch + write_batch); InMemoryBuffer can be developed in parallel with US1 implementation
- **US3 (P2)**: Requires US2 (run wraps produce_once); loop/sleep/iterations build on top

### Within Each User Story

1. Write tests FIRST (TDD red step) -- ensure they FAIL before implementation
2. Implement types in dependency order (errors before config, config before producer)
3. Run tests to confirm green (TDD green step)
4. Refactor if needed (TDD refactor step)

### Parallel Opportunities

- T002, T003, T004: All Cargo.toml files have no mutual dependencies
- T006, T007: Both write tests to the same file -- sequence them or merge
- T012, T013: Write two independent producer tests (different assertions, same file)
- T018: InMemoryBuffer test can be written in parallel with T017 (different files)
- T024, T025: Two independent closed/full error-path tests
- T030, T031: Compliance check and clippy run are independent
- T033, T034: Build release and quickstart validation are independent

---

## Parallel Example: User Story 1

```bash
# After Phase 2 completes, these 2 tests can be written simultaneously:
Task T012: "Write batch_size_bounds test in crates/producer/src/lib.rs"
Task T013: "Write tx_fields_valid test in crates/producer/src/lib.rs"

# After tests are written and failing, implementations have strict order:
Task T014: ProducerError (needed by T015, T016)
Task T015: ProducerConfig + Builder (needed by T016)
Task T016: Producer + generate_batch
```

## Parallel Example: User Story 2

```bash
# After US1 tests are written, these two test tasks can run in parallel:
Task T017: "Write produce_and_write test in crates/producer/src/lib.rs"
Task T018: "Write in_memory_buffer_stores_batch test in crates/fraud_detection/..."

# Implementations have a dependency order:
Task T019: adapters/mod.rs (unblocks T020)
Task T020: InMemoryBuffer (unblocks T017 implementation, T022)
Task T021: produce_once (unblocks T022, T026)
Task T022: main.rs wiring
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL -- blocks all stories)
3. Complete Phase 3: User Story 1 (transaction generation)
4. Complete Phase 4: User Story 2 (buffer write via trait)
5. **STOP and VALIDATE**: `cargo run` produces a logged batch; `cargo test --workspace` is green
6. Demo architecture: swap InMemoryBuffer for any other Buffer1 impl with zero Producer changes

### Incremental Delivery

1. Setup + Foundational -> empty workspace compiles
2. US1 -> transaction generation is independently testable
3. US2 -> end-to-end produce_once works; binary runs and logs output
4. US3 -> continuous loop with configurable speed; binary demonstrates pipeline throughput
5. Polish -> workspace is lint-clean and SC-004 (TDD) compliance is verified

### Single Developer Strategy

1. Phase 1 (T001-T005) -- 30 min
2. Phase 2 (T006-T010) -- 45 min
3. Phase 3 (T011-T016) -- 60 min -- checkpoint: cargo test -p producer
4. Phase 4 (T017-T022) -- 60 min -- checkpoint: cargo run
5. Phase 5 (T023-T029) -- 60 min -- checkpoint: cargo test --workspace
6. Phase 6 (T030-T034) -- 30 min -- checkpoint: clean build + quickstart

---

## Notes

- [P] tasks target different files or independent concerns -- safe to parallelize
- [USN] label maps each task to its user story for traceability and independent delivery
- TDD order within each story: write failing tests -> implement -> confirm green -> refactor
- Commit after each phase checkpoint
- ms-rust compliance: integrate into implementation (not deferred to polish only)
- `cargo_common_metadata` and `multiple_crate_versions` lints must be disabled in workspace.lints (internal workspace, not published)
- `string_to_string` clippy lint: do not include (renamed/removed in Rust 1.93+)
- AFIT lint suppression: #[expect] on trait definition only, not on impl blocks
