# Retrospective Review Checklist: Modelizer Inference

**Purpose**: Post-feature author self-review -- all four requirement clusters + error handling gaps
**Created**: 2026-02-23
**Feature**: [spec.md](../spec.md)
**Scope**: Hexagonal architecture, probabilistic behavior, version switching, integration, error handling
**Depth**: Standard | **Audience**: Author (retrospective)

---

## Requirement Completeness

- [X] CHK001 - Are the data types of all three InferredTransaction enrichment fields (`predicted_fraud: bool`, `model_name: String`, `model_version: String`) explicitly stated in FR-002, or only named? [Completeness, Spec §FR-002]
  - PASS: FR-002 states "true or false" (bool), "model name (string)", "model version (string representation of the version number)". Types are explicit.
- [ ] CHK002 - Are batch size constraints for Modelizer input specified, or implicitly inherited from Consumer config without cross-reference? [Completeness, Spec §FR-001]
  - FAIL: Assumptions states "Batch sizes are determined by the Consumer" but gives no cross-reference to feature 002 spec or N2_MAX value. Implicit only.
- [ ] CHK003 - Are all four Model port methods (`classify`, `switch_version`, `name`, `active_version`) fully specified by name and sync/async distinction in requirements? [Completeness, Spec §FR-013, §FR-014]
  - FAIL: FR-013 and FR-014 describe the methods conceptually but never name all four or state which are async. Only data-model.md has the full method table.
- [ ] CHK004 - Is the RNG seeding mechanism (seed type, injection point, constructor vs runtime) specified for reproducibility in FR-011? [Completeness, Spec §FR-011]
  - FAIL: FR-011 only says "seeded randomness". Seed type (u64), injection point (constructor `Option<u64>`), and default (OS rng) are resolved in plan/research, not spec.
- [X] CHK005 - Is the crate-level location of the DEMO adapter (fraud_detection, not modelizer) stated in requirements, or only resolved in plan? [Completeness, Spec §FR-016]
  - PASS: FR-016 explicitly names "a DEMO Model adapter (fraud_detection crate)".

## Requirement Clarity

- [X] CHK006 - Is "same order" in FR-001 defined precisely enough to exclude all permutations of output items? [Clarity, Spec §FR-001]
  - PASS: "same size and order" in FR-001 and SC-001 combined with acceptance scenario (5 in, 5 out with original tx data) excludes all permutations.
- [ ] CHK007 - Is "self-describing" in FR-014 specific enough to state when `name` and `active_version` are read (once per batch vs. once per transaction vs. per call)? [Clarity, Spec §FR-014]
  - FAIL: FR-014 says "when building each InferredTransaction" (implies per-tx), but research R6 and implementation read once per batch. Ambiguity between spec wording and actual semantics.
- [ ] CHK008 - Is "next inference call" in FR-009 defined relative to concurrent execution, or only for sequential calls? [Clarity, Spec §FR-009]
  - FAIL: FR-009 only addresses sequential calls. Concurrent execution is neither specified nor explicitly excluded.
- [ ] CHK009 - Is "passes ModelVersion values through without interpreting them" in FR-015 specific enough to exclude any conditional logic in the Modelizer? [Clarity, Spec §FR-015]
  - FAIL: Stated as intent. No structural constraint (e.g., "modelizer crate MUST NOT contain a match on ModelVersion").
- [X] CHK010 - Is "poll_interval2" in the Input section defined or cross-referenced to feature 002 spec, or assumed as shared knowledge? [Clarity, Spec Input]
  - PASS: Assumptions section defines "poll_interval2" as the Consumer's iteration cadence.
- [ ] CHK011 - Is the scope of FR-012 ("never depend on concrete implementation") stated in terms of import/type-level constraints, not just intent? [Clarity, Spec §FR-012]
  - FAIL: "never on a concrete model implementation" is stated as intent. No import/crate-dependency constraint is specified.

## Requirement Consistency

- [X] CHK012 - Is "next inference call" (FR-009) consistent with "very next inference call" (SC-004) -- does SC-004 add a stricter guarantee not expressed in FR-009? [Conflict, Spec §FR-009, §SC-004]
  - PASS: Semantically equivalent. "Starting from the next call" (FR-009) = "the very next call" (SC-004). No additional strictness.
- [X] CHK013 - Does FR-010 (probabilistic, not deterministic) conflict with FR-011 (seeded, reproducible) -- is the distinction between "non-deterministic by attribute" and "seeded RNG" clarified? [Conflict, Spec §FR-010, §FR-011]
  - PASS: No conflict. FR-010 excludes determinism by transaction attribute (e.g., amount-based rules); FR-011 allows reproducible probabilistic RNG. A seeded RNG is still probabilistic.
- [X] CHK014 - Are version identifiers ("3"/"4" as strings) stated consistently across FR-004, FR-005, FR-006, and the Assumptions section without format discrepancy? [Consistency, Spec §FR-004, Assumptions]
  - PASS: FR-004 uses "version 3" and "version 4"; FR-002 says "string representation"; Assumptions says `"3"` and `"4"`. Consistent.
- [X] CHK015 - Are tolerance bounds in SC-002/SC-003 ([3%,5%] and [2%,4%]) consistent with the ±1 percentage point description given in the same section? [Consistency, Spec §SC-002, §SC-003]
  - PASS: 4% ± 1pp = [3%,5%]; 3% ± 1pp = [2%,4%]. Correct.

## Acceptance Criteria Quality

- [ ] CHK016 - Are the statistical tolerance bounds in SC-002/SC-003 accompanied by a minimum sample size justification, or is 10,000 an arbitrary threshold? [Measurability, Spec §SC-002, §SC-003]
  - FAIL: "10,000" stated without statistical justification (no confidence level, no derivation).
- [X] CHK017 - Can SC-004 ("very next inference call reflects new version") be deterministically verified in a unit test without timing or ordering dependencies? [Measurability, Spec §SC-004]
  - PASS: Sequential async context (single-threaded) makes this deterministically verifiable. Tests T019-T021 confirm.
- [X] CHK018 - Is SC-005 ("identical fraud predictions") expressed as sequence equality, or does it allow statistical equivalence that cannot be mechanically asserted? [Measurability, Spec §SC-005]
  - PASS: "Identical fraud predictions" unambiguously means element-wise sequence equality. Test T025 asserts this.
- [X] CHK019 - Does SC-001 address the empty-batch case explicitly, or only by implication from the general "exactly as many items" rule? [Coverage, Spec §SC-001]
  - PASS: SC-001 covers empty batch by implication ("exactly as many items" = 0); edge case section confirms explicitly.

## Scenario Coverage

- [X] CHK020 - Are requirements defined for calling `switch_version` with the already-active version (explicit no-op guarantee)? [Coverage, Spec Edge Cases]
  - PASS: Edge cases: "No-op, no error."
- [X] CHK021 - Are requirements defined for internal state consistency (version, RNG) across many successive batch calls on the same Modelizer instance? [Coverage, Spec Edge Cases]
  - PASS: Edge cases: "Internal state (version, RNG) remains consistent and correct."
- [ ] CHK022 - Are concurrent access scenarios (parallel `infer` calls, racing `switch_version` + `infer`) addressed or explicitly excluded as out of scope? [Coverage, Gap]
  - FAIL: No mention of concurrent access anywhere. Not specified and not explicitly excluded.
- [ ] CHK023 - Is the behavior of `infer` on a batch containing duplicate transaction IDs specified or explicitly excluded? [Coverage, Gap]
  - FAIL: Not mentioned. Neither specified nor excluded.

## Error Handling (Gap)

- [ ] CHK024 - Are requirements defined for what the Modelizer returns when `Model::classify` returns an error for one or more transactions? [Gap]
  - FAIL: No requirements for classify error propagation. data-model.md notes "classify must return Ok(true) or Ok(false) -- never errors for DEMO" but this is adapter-specific, not a general Modelizer requirement.
- [ ] CHK025 - Is behavior specified when the Model port is unavailable, panics, or returns an unexpected state? [Gap]
  - FAIL: Not addressed in spec.
- [ ] CHK026 - Does the spec define partial batch failure behavior -- e.g., how many inferred transactions to return if some classifications fail? [Gap]
  - FAIL: Not addressed.
- [ ] CHK027 - Are `ModelizerError` variants (if any) specified at the requirements level, or only implicitly defined in the plan/implementation? [Gap]
  - FAIL: ModelizerError appears in Key Entities as "already defined in domain" but variants (InferenceFailed, SwitchFailed) are only in data-model.md, not in requirements.

## Hexagonal Architecture Requirements

- [ ] CHK028 - Does FR-012 state a structural constraint (no concrete type import in modelizer crate) or only an intent ("never depend on")? [Clarity, Spec §FR-012]
  - FAIL: Same finding as CHK011 -- intent only, no crate-level import constraint.
- [ ] CHK029 - Is there a requirement preventing the Model port from holding pipeline-level concerns (buffer access, logging, metrics)? [Completeness, Gap]
  - FAIL: No such requirement in spec.
- [X] CHK030 - Is the per-transaction boundary of the Model port (FR-013) stated clearly enough to exclude a per-batch adapter interpretation? [Clarity, Spec §FR-013]
  - PASS: FR-013: "The Model port MUST operate per-transaction (single classification decision). The Modelizer is responsible for iterating over the batch." Excludes per-batch interpretation.

## Dependencies & Assumptions

- [ ] CHK031 - Is the assumption that Consumer (not Producer or Logger) is the sole orchestrator of `switch_version` stated explicitly in requirements? [Assumption, Spec §FR-008]
  - FAIL: FR-008 says "The Consumer MUST be able to switch" -- confirms Consumer can switch, but does not state it is the sole orchestrator.
- [ ] CHK032 - Is the assumption that `ModelVersion` is already defined in the domain crate cross-referenced to prior feature specs, not just stated? [Dependency, Spec §FR-015, Assumptions]
  - FAIL: Assumptions says "reused as-is" without naming which prior feature introduced ModelVersion (feature 001 or 002).

## Notes

- Reviewed 2026-02-23 against spec.md, research.md, data-model.md, and implementation (feature 003 complete).
- **Summary**: 14 PASS, 18 FAIL.
- PASS: CHK001, CHK005, CHK006, CHK010, CHK012, CHK013, CHK014, CHK015, CHK017, CHK018, CHK019, CHK020, CHK021, CHK030.
- FAIL: CHK002, CHK003, CHK004, CHK007, CHK008, CHK009, CHK011, CHK016, CHK022, CHK023, CHK024, CHK025, CHK026, CHK027, CHK028, CHK029, CHK031, CHK032.
- Items marked [Gap] indicate requirements absent from spec -- flag for next feature spec if needed.
- Items marked [Conflict] have two FR/SC entries that may impose overlapping or contradictory constraints.
- Items marked [Assumption] are documented in spec Assumptions section but not validated against prior feature deliverables.
- Key patterns to address in future specs: (1) always name and type-annotate all port methods in requirements; (2) state structural constraints for hexagonal boundaries (not just intent); (3) define error propagation policy for partial failures; (4) justify statistical sample sizes; (5) cross-reference dependencies to prior feature specs by name.
