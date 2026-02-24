# Tasks: Modelizer Inference

**Input**: Design documents from `/specs/003-modelizer-inference/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: TDD is NON-NEGOTIABLE per constitution (Principle III). Test tasks MUST be written and seen to FAIL before any implementation task in the same story begins.

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no incomplete-task dependencies)
- **[Story]**: User story label (US1-US4)
- Exact file paths in every description

---

## Phase 1: Setup (Crate Infrastructure)

**Purpose**: Create `modelizer` lib crate and register it in the workspace.

- [X] T001 Add `"crates/modelizer"` to `members` in root `Cargo.toml`
- [X] T002 [P] Create `crates/modelizer/Cargo.toml` (name=modelizer, edition=2024, deps: domain path, log workspace; dev-deps: tokio workspace)
- [X] T003 [P] Create `crates/modelizer/src/lib.rs` stub (empty module-level doc comment; verify `cargo build -p modelizer` passes)

---

## Phase 2: Foundational (Model Hexagonal Port in Domain)

**Purpose**: Add the `Model` port trait to the domain crate. This blocks all user stories.

**CRITICAL**: No user story work begins until Phase 2 is complete.

- [X] T004 Add test `model_trait_compiles_with_minimal_impl` in `crates/domain/src/lib.rs` under `#[cfg(test)]` -- declare a zero-field struct implementing all four `Model` methods; test must FAIL (does not compile) before T005
- [X] T005 Add `Model` hexagonal port trait to `crates/domain/src/lib.rs`: `async fn classify(&self, tx: &Transaction) -> Result<bool, ModelizerError>`; `fn name(&self) -> &str`; `fn active_version(&self) -> &str`; `async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError>`; add `#[expect(async_fn_in_trait, reason="...")]` on trait def; update module docstring to list `Model`
- [X] T006 Verify `cargo test -p domain` passes (T004 green)

**Checkpoint**: `Model` trait defined; `domain` tests pass.

---

## Phase 3: User Story 1 - Batch Inference (Priority: P1) MVP

**Goal**: Generic `Modelizer<M>` struct in `modelizer` crate implements `domain::Modelizer` by delegating to a `Model` impl. Returns one `InferredTransaction` per input, same order.

**Independent Test**: Inject a `MockModel`, send varied batch sizes (0, 1, 5), verify count, order, and enrichment fields.

### Tests (write first -- MUST FAIL before T010)

- [X] T007 Add `MockModel` test helper + test `empty_batch_returns_empty` in `#[cfg(test)]` of `crates/modelizer/src/lib.rs` (MockModel returns name="MOCK", version="v0", classify=false; send 0 txs, assert 0 out)
- [X] T008 Add test `batch_inference_returns_same_count_in_order` in `crates/modelizer/src/lib.rs` (5 txs in, 5 out; verify IDs match input order)
- [X] T009 Add test `inferred_tx_carries_model_name_and_version` in `crates/modelizer/src/lib.rs` (1 tx in; assert model_name=="MOCK", model_version=="v0", predicted_fraud==false)

### Implementation

- [X] T010 [US1] Implement `Modelizer<M: Model>` struct + `#[must_use] pub fn new(model: M) -> Self` in `crates/modelizer/src/lib.rs`
- [X] T011 [US1] Implement `domain::Modelizer` for `Modelizer<M>`: read `model.name()` and `model.active_version()` once before the loop; iterate batch calling `model.classify(&tx).await`; build one `InferredTransaction` per tx; collect into `Vec` in `crates/modelizer/src/lib.rs`
- [X] T012 Checkpoint -- `cargo test -p modelizer` passes (T007-T009 green)

---

## Phase 4: User Story 2 - Default Model Version (Priority: P2)

**Goal**: `DemoModel` adapter starts with `ModelVersion::N` (version 4). A freshly created `Modelizer<DemoModel>` carries `model_name="DEMO"` and `model_version="4"` without any switch call.

**Independent Test**: Create `DemoModel::new(None)`, assert `name()=="DEMO"` and `active_version()=="4"`.

### Tests (write first -- MUST FAIL before T016)

- [X] T013 [US2] Create `crates/fraud_detection/src/adapters/demo_model.rs` with `DemoModel` struct skeleton (field `current_version: RefCell<ModelVersion>`; no RNG yet; `use domain::{Model, ModelizerError, ModelVersion, Transaction};`); add `pub mod demo_model;` to `crates/fraud_detection/src/adapters/mod.rs`
- [X] T014 [US2] Add test `demo_model_name_is_demo` in `#[cfg(test)]` of `crates/fraud_detection/src/adapters/demo_model.rs` (assert `DemoModel::new(None).name() == "DEMO"`; FAIL before T016)
- [X] T015 [US2] Add test `demo_model_default_active_version_is_4` in demo_model.rs (assert `DemoModel::new(None).active_version() == "4"`; FAIL before T016)

### Implementation

- [X] T016 [US2] Implement `DemoModel::new(seed: Option<u64>) -> Self` (ignore seed for now; init `current_version = RefCell::new(ModelVersion::N)`) in `crates/fraud_detection/src/adapters/demo_model.rs`; add `#[must_use]` and `#[derive(Debug)]`
- [X] T017 [US2] Implement `domain::Model` for `DemoModel`: `name` returns `"DEMO"`; `active_version` returns `"4"` for N and `"3"` for NMinus1 (match on `*self.current_version.borrow()`); stub `classify` returns `Ok(false)`; stub `switch_version` is a no-op returning `Ok(())` in `crates/fraud_detection/src/adapters/demo_model.rs`
- [X] T018 Checkpoint -- `cargo test -p fraud_detection` passes T014-T015

---

## Phase 5: User Story 3 - Version Switching (Priority: P2)

**Goal**: Consumer can switch `DemoModel` between `ModelVersion::N` (version 4) and `ModelVersion::NMinus1` (version 3). `Modelizer::switch_version` delegates to `model.switch_version`. Switch takes effect on next `infer` call.

**Independent Test**: Call `switch_version(NMinus1)`, assert `active_version()=="3"`; call `switch_version(N)`, assert back to `"4"`.

### Tests (write first -- MUST FAIL before T022/T023)

- [X] T019 [US3] Add test `switch_to_nminus1_active_version_is_3` in demo_model.rs tests (call `model.switch_version(ModelVersion::NMinus1).await`, assert `active_version()=="3"`; FAIL before T022)
- [X] T020 [US3] Add test `switch_to_n_active_version_is_4` in demo_model.rs tests (switch to NMinus1 then back to N, assert `active_version()=="4"`; FAIL before T022)
- [X] T021 [P] [US3] Add test `modelizer_switch_delegates_to_model` in `crates/modelizer/src/lib.rs` tests (extend MockModel with `switch_call: Cell<Option<ModelVersion>>`; call `Modelizer::switch_version(NMinus1)`, assert MockModel recorded the call; FAIL before T023)

### Implementation

- [X] T022 [US3] Implement `domain::Model::switch_version` for `DemoModel`: update `*self.current_version.borrow_mut() = version` in `crates/fraud_detection/src/adapters/demo_model.rs`
- [X] T023 [P] [US3] Implement `domain::Modelizer::switch_version` for `Modelizer<M>`: delegate `self.model.switch_version(version).await` in `crates/modelizer/src/lib.rs`
- [X] T024 Checkpoint -- `cargo test` passes US3 tests (T019-T021 green)

---

## Phase 6: User Story 4 - Probabilistic Fraud Detection (Priority: P3)

**Goal**: `DemoModel::classify` uses a seeded `StdRng`. Version 4 (N) detects fraud ~4% of the time; version 3 (N-1) ~3%. With the same seed, two instances produce identical sequences.

**Independent Test**: 10k sample at version 4 -- observed rate in `[3%, 5%]`. 10k sample at version 3 -- rate in `[2%, 4%]`. Seeded pairs are identical.

### Tests (write first -- MUST FAIL before T029)

- [X] T025 [US4] Add test `classify_seeded_is_deterministic` in demo_model.rs tests (two `DemoModel::new(Some(42))`, same 100 classify calls, assert identical results; FAIL before T029)
- [X] T026 [US4] Add test `fraud_rate_v4_is_approx_4pct` in demo_model.rs tests (10_000 classify calls, version N; assert `3.0 <= rate_pct && rate_pct <= 5.0`; FAIL before T029)
- [X] T027 [US4] Add test `fraud_rate_v3_is_approx_3pct` in demo_model.rs tests (switch to NMinus1; 10_000 calls; assert `2.0 <= rate_pct && rate_pct <= 4.0`; FAIL before T029)

### Implementation

- [X] T028 [US4] Add `rand = { workspace = true }` to `[dependencies]` in `crates/fraud_detection/Cargo.toml`
- [X] T029 [US4] Add `rng: RefCell<StdRng>` field to `DemoModel`; update `new(seed)` to init via `StdRng::seed_from_u64(s)` or `StdRng::from_os_rng()`; add `use rand::{Rng, SeedableRng, rngs::StdRng};` in `crates/fraud_detection/src/adapters/demo_model.rs`
- [X] T030 [US4] Implement `domain::Model::classify` for `DemoModel`: compute `fraud_rate` (0.04 for N, 0.03 for NMinus1); roll `self.rng.borrow_mut().random::<f64>() < fraud_rate`; return `Ok(roll)` in `crates/fraud_detection/src/adapters/demo_model.rs`
- [X] T031 Checkpoint -- `cargo test -p fraud_detection` passes T025-T027 (US4 green)

---

## Final Phase: Integration & Polish

**Purpose**: Wire `Modelizer<DemoModel>` into the binary, remove `DemoModelizer`, add logging, final build check.

- [X] T032 Add `modelizer = { path = "../modelizer" }` to `[dependencies]` in `crates/fraud_detection/Cargo.toml`
- [X] T033 [P] Delete `crates/fraud_detection/src/adapters/demo_modelizer.rs`
- [X] T034 Remove `pub mod demo_modelizer;` from `crates/fraud_detection/src/adapters/mod.rs`; add `pub mod demo_model;` if not already present (should be from T013)
- [X] T035 [P] Add `log::debug!` to `Modelizer::infer` (batch size) and `log::info!` to `Modelizer::switch_version` (new version) in `crates/modelizer/src/lib.rs`
- [X] T036 [P] Add `log::debug!` to `DemoModel::classify` (fraud decision) and `log::info!` to `DemoModel::switch_version` (version change) in `crates/fraud_detection/src/adapters/demo_model.rs`
- [X] T037 Update `crates/fraud_detection/src/main.rs`: remove `DemoModelizer` import; add `use adapters::demo_model::DemoModel;` and `use modelizer::Modelizer;`; replace `DemoModelizer::new(false)` with `Modelizer::new(DemoModel::new(None))`
- [X] T038 Add `// Rust guideline compliant 2026-02-16` compliance comment to all new/modified `.rs` files (`crates/modelizer/src/lib.rs`, `crates/domain/src/lib.rs`, `crates/fraud_detection/src/adapters/demo_model.rs`, `crates/fraud_detection/src/adapters/mod.rs`, `crates/fraud_detection/src/main.rs`)
- [X] T039 Run `cargo test --workspace` and verify all tests pass; run `cargo build --release` and confirm pipeline binary compiles

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies -- start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1
- **Phase 3 (US1)**: Depends on Phase 2 (Model trait must exist)
- **Phase 4 (US2)**: Depends on Phase 3 (DemoModel must impl Model; tests in modelizer already pass)
- **Phase 5 (US3)**: Depends on Phase 4 (DemoModel struct must exist)
- **Phase 6 (US4)**: Depends on Phase 4 (DemoModel struct must exist; parallel with Phase 5)
- **Final Phase**: Depends on all preceding phases

### User Story Dependencies

- **US1 (P1)**: Starts after Foundational (Model trait) -- only needs MockModel
- **US2 (P2)**: Starts after US1 (DemoModel uses the Model trait proven in US1)
- **US3 (P2)**: Starts after US2 (DemoModel struct exists); parallel with US4
- **US4 (P3)**: Starts after US2 (DemoModel struct exists); parallel with US3

### Within Each User Story

1. Write all test tasks (must fail/not compile)
2. Implement until tests pass
3. Verify checkpoint before moving on

---

## Parallel Opportunities

### Phase 1 (after T001)

```text
T002 Create crates/modelizer/Cargo.toml
T003 Create crates/modelizer/src/lib.rs stub
```

### Phase 5 (tests: different files)

```text
T019 + T020: switch tests in demo_model.rs
T021:        switch test in modelizer/src/lib.rs   [P - different file]
```

### Phase 5 (impl: different files, same intent)

```text
T022 DemoModel::switch_version (demo_model.rs)
T023 Modelizer::switch_version (modelizer/src/lib.rs)  [P - different file]
```

### Final Phase (after T032 + T033)

```text
T035 Logging in modelizer/src/lib.rs
T036 Logging in demo_model.rs             [P - different file]
```

---

## Implementation Strategy

### MVP (User Story 1 only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (Model trait)
3. Complete Phase 3: US1 (Modelizer<M> struct with MockModel)
4. **VALIDATE**: `cargo test -p modelizer` -- all passing

### Incremental Delivery

1. Phase 1 + 2 + 3 → `Modelizer<M>` generic component working
2. Phase 4 → `DemoModel` with name + version; `Modelizer<DemoModel>` usable
3. Phase 5 → Version switching end-to-end
4. Phase 6 → Probabilistic fraud rates correct
5. Final Phase → Pipeline binary updated; DemoModelizer removed

### Parallel Strategy (phases 5 and 6)

Once Phase 4 is complete:
- Developer A: Phase 5 (US3 -- version switching)
- Developer B: Phase 6 (US4 -- probabilistic classification)
Both modify different files within `demo_model.rs` vs `demo_model.rs` (same file, actually sequential) but the modelizer-side tasks (T021, T023) are parallel to the demo_model-side tasks.

---

## Notes

- `[P]` = different files, no incomplete-task dependencies
- `[Story]` maps task to user story for traceability
- TDD: red before green; never skip the failing state
- `classify` stub (`Ok(false)`) in Phase 4 intentionally stays simple until Phase 6
- `switch_version` no-op stub in Phase 4 intentionally stays simple until Phase 5
- `rand` dep added in T028, before T029/T030 need it to compile
- `DemoModelizer` is only removed in Final Phase to keep binary compilable throughout development
