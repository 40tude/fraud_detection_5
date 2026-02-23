# Retrospective Review Checklist: Modelizer Inference

**Purpose**: Post-feature author self-review -- all four requirement clusters + error handling gaps
**Created**: 2026-02-23
**Feature**: [spec.md](../spec.md)
**Scope**: Hexagonal architecture, probabilistic behavior, version switching, integration, error handling
**Depth**: Standard | **Audience**: Author (retrospective)

---

## Requirement Completeness

- [ ] CHK001 - Are the data types of all three InferredTransaction enrichment fields (`predicted_fraud: bool`, `model_name: String`, `model_version: String`) explicitly stated in FR-002, or only named? [Completeness, Spec §FR-002]
- [ ] CHK002 - Are batch size constraints for Modelizer input specified, or implicitly inherited from Consumer config without cross-reference? [Completeness, Spec §FR-001]
- [ ] CHK003 - Are all four Model port methods (`classify`, `switch_version`, `name`, `active_version`) fully specified by name and sync/async distinction in requirements? [Completeness, Spec §FR-013, §FR-014]
- [ ] CHK004 - Is the RNG seeding mechanism (seed type, injection point, constructor vs runtime) specified for reproducibility in FR-011? [Completeness, Spec §FR-011]
- [ ] CHK005 - Is the crate-level location of the DEMO adapter (fraud_detection, not modelizer) stated in requirements, or only resolved in plan? [Completeness, Spec §FR-016]

## Requirement Clarity

- [ ] CHK006 - Is "same order" in FR-001 defined precisely enough to exclude all permutations of output items? [Clarity, Spec §FR-001]
- [ ] CHK007 - Is "self-describing" in FR-014 specific enough to state when `name` and `active_version` are read (once per batch vs. once per transaction vs. per call)? [Clarity, Spec §FR-014]
- [ ] CHK008 - Is "next inference call" in FR-009 defined relative to concurrent execution, or only for sequential calls? [Clarity, Spec §FR-009]
- [ ] CHK009 - Is "passes ModelVersion values through without interpreting them" in FR-015 specific enough to exclude any conditional logic in the Modelizer? [Clarity, Spec §FR-015]
- [ ] CHK010 - Is "speed2" in the Input section defined or cross-referenced to feature 002 spec, or assumed as shared knowledge? [Clarity, Spec Input]
- [ ] CHK011 - Is the scope of FR-012 ("never depend on concrete implementation") stated in terms of import/type-level constraints, not just intent? [Clarity, Spec §FR-012]

## Requirement Consistency

- [ ] CHK012 - Is "next inference call" (FR-009) consistent with "very next inference call" (SC-004) -- does SC-004 add a stricter guarantee not expressed in FR-009? [Conflict, Spec §FR-009, §SC-004]
- [ ] CHK013 - Does FR-010 (probabilistic, not deterministic) conflict with FR-011 (seeded, reproducible) -- is the distinction between "non-deterministic by attribute" and "seeded RNG" clarified? [Conflict, Spec §FR-010, §FR-011]
- [ ] CHK014 - Are version identifiers ("3"/"4" as strings) stated consistently across FR-004, FR-005, FR-006, and the Assumptions section without format discrepancy? [Consistency, Spec §FR-004, Assumptions]
- [ ] CHK015 - Are tolerance bounds in SC-002/SC-003 ([3%,5%] and [2%,4%]) consistent with the ±1 percentage point description given in the same section? [Consistency, Spec §SC-002, §SC-003]

## Acceptance Criteria Quality

- [ ] CHK016 - Are the statistical tolerance bounds in SC-002/SC-003 accompanied by a minimum sample size justification, or is 10,000 an arbitrary threshold? [Measurability, Spec §SC-002, §SC-003]
- [ ] CHK017 - Can SC-004 ("very next inference call reflects new version") be deterministically verified in a unit test without timing or ordering dependencies? [Measurability, Spec §SC-004]
- [ ] CHK018 - Is SC-005 ("identical fraud predictions") expressed as sequence equality, or does it allow statistical equivalence that cannot be mechanically asserted? [Measurability, Spec §SC-005]
- [ ] CHK019 - Does SC-001 address the empty-batch case explicitly, or only by implication from the general "exactly as many items" rule? [Coverage, Spec §SC-001]

## Scenario Coverage

- [ ] CHK020 - Are requirements defined for calling `switch_version` with the already-active version (explicit no-op guarantee)? [Coverage, Spec Edge Cases]
- [ ] CHK021 - Are requirements defined for internal state consistency (version, RNG) across many successive batch calls on the same Modelizer instance? [Coverage, Spec Edge Cases]
- [ ] CHK022 - Are concurrent access scenarios (parallel `infer` calls, racing `switch_version` + `infer`) addressed or explicitly excluded as out of scope? [Coverage, Gap]
- [ ] CHK023 - Is the behavior of `infer` on a batch containing duplicate transaction IDs specified or explicitly excluded? [Coverage, Gap]

## Error Handling (Gap)

- [ ] CHK024 - Are requirements defined for what the Modelizer returns when `Model::classify` returns an error for one or more transactions? [Gap]
- [ ] CHK025 - Is behavior specified when the Model port is unavailable, panics, or returns an unexpected state? [Gap]
- [ ] CHK026 - Does the spec define partial batch failure behavior -- e.g., how many inferred transactions to return if some classifications fail? [Gap]
- [ ] CHK027 - Are `ModelizerError` variants (if any) specified at the requirements level, or only implicitly defined in the plan/implementation? [Gap]

## Hexagonal Architecture Requirements

- [ ] CHK028 - Does FR-012 state a structural constraint (no concrete type import in modelizer crate) or only an intent ("never depend on")? [Clarity, Spec §FR-012]
- [ ] CHK029 - Is there a requirement preventing the Model port from holding pipeline-level concerns (buffer access, logging, metrics)? [Completeness, Gap]
- [ ] CHK030 - Is the per-transaction boundary of the Model port (FR-013) stated clearly enough to exclude a per-batch adapter interpretation? [Clarity, Spec §FR-013]

## Dependencies & Assumptions

- [ ] CHK031 - Is the assumption that Consumer (not Producer or Logger) is the sole orchestrator of `switch_version` stated explicitly in requirements? [Assumption, Spec §FR-008]
- [ ] CHK032 - Is the assumption that `ModelVersion` is already defined in the domain crate cross-referenced to prior feature specs, not just stated? [Dependency, Spec §FR-015, Assumptions]

## Notes

- Items marked [Gap] indicate requirements absent from spec -- flag for next feature spec if needed.
- Items marked [Conflict] have two FR/SC entries that may impose overlapping or contradictory constraints.
- Items marked [Assumption] are documented in spec Assumptions section but not validated against prior feature deliverables.
