# Error Propagation Checklist: Consumer Batch Processing

**Purpose**: Pre-implementation author sanity check -- validates domain-crate requirements quality with emphasis on error propagation semantics, port contracts, and entity definitions.
**Created**: 2026-02-22
**Feature**: [spec.md](../spec.md) | [plan.md](../plan.md)
**Scope**: Consumer domain crate + port trait contracts only (binary/adapter wiring excluded)

---

## Error Propagation Semantics

- [ ] CHK001 - Is the return type/shape of the alarm error collection channel specified in the spec (e.g., distinct Result variant, out-param, separate field)? [Completeness, Spec §FR-013]
- [ ] CHK002 - Does the spec define what happens to collected alarm errors when the subsequent Buffer2 write fails -- are both errors surfaced or is one lost? [Edge Case, Spec §FR-013]
- [ ] CHK003 - Is "propagates to the caller immediately" (FR-013) defined in terms of Consumer lifecycle -- does it abort the current iteration, skip to the next, or halt the Consumer loop entirely? [Clarity, Spec §FR-013]
- [ ] CHK004 - Are error propagation effects on Consumer lifecycle consistent across all three immediate-propagation ports (Buffer1, Modelizer, Buffer2)? [Consistency, Spec §FR-013, US2 AC2, US4 AC2-3]
- [ ] CHK005 - Does the spec define whether a Buffer2 write failure terminates the Consumer permanently or allows recovery on the next iteration? [Clarity, Spec §FR-013, US4 AC2-3]
- [ ] CHK006 - Is "best-effort" delivery (FR-006) defined precisely enough to distinguish at-most-once from at-least-once alarm trigger semantics? [Clarity, Spec §FR-006]
- [ ] CHK007 - Does SC-002 ("100% of fraudulent transactions trigger exactly one alarm each") conflict with the best-effort delivery model where a trigger attempt may fail? [Conflict, Spec §SC-002 vs FR-006]
- [ ] CHK008 - Is the required ordering of operations (all alarms attempted, then Buffer2 write) explicitly a requirement, or left as an implementation detail? [Clarity, Spec §FR-006, FR-007]
- [ ] CHK009 - Are requirements defined for what happens when Buffer1 returns a non-empty, non-closed error (e.g., I/O failure mid-read)? [Coverage, Gap]
- [ ] CHK010 - Does the spec define whether alarm failures from a batch should be aggregated into a single error or reported individually? [Clarity, Spec §FR-013]

---

## Port Contract Completeness

- [ ] CHK011 - Does the Buffer1Read port spec define which signal (error variant or sentinel) distinguishes "buffer closed and drained" from "buffer temporarily empty but open"? [Completeness, Spec §Assumptions, US1 AC3-4]
- [ ] CHK012 - Is the async-blocking wait behavior (US1 AC3: "Consumer waits without error") a requirement on the Buffer1Read port contract or delegated to each implementation? [Clarity, Spec §US1 AC3]
- [ ] CHK013 - Does the Modelizer port spec define whether `switch_version` can be called concurrently with `infer`, and what the ordering guarantee is? [Coverage, Gap]
- [ ] CHK014 - Is the requirement that Modelizer defaults to version N at startup (FR-009) specified as a port-level contract applicable to all implementations, or only an expectation for the demo adapter? [Clarity, Spec §FR-009]
- [ ] CHK015 - Does the Alarm port spec define or prohibit calling `trigger` with a non-fraudulent (legitimate) transaction? [Coverage, Gap]
- [ ] CHK016 - Are both Buffer2 failure modes (full, closed) required to be modeled as distinct error variants in the Buffer2 port spec, or inferred from the existing BufferError pattern? [Completeness, Spec §FR-007, US4 AC2-3]
- [ ] CHK017 - Is the ownership contract for the transaction reference passed to `Alarm::trigger` (borrow vs. owned) specified in the port definition? [Clarity, Spec §Key Entities - Alarm]
- [ ] CHK018 - Does the spec require static dispatch (generics, no dyn) for all port interactions, and is this consistent with the plan constraint ("no dyn dispatch")? [Consistency, Spec §plan.md constraints]

---

## Entity and Type Definitions

- [ ] CHK019 - Is InferredTransaction's structural relationship to Transaction (composition wrapping vs. flat struct with copied fields) specified in the entity definition? [Clarity, Spec §Key Entities - InferredTransaction]
- [ ] CHK020 - Are ModelVersion's two variants (N, NMinus1) specified as the exhaustive closed set, or is extensibility left open? [Completeness, Spec §Key Entities - ModelVersion]
- [ ] CHK021 - Is the string format or valid-value contract for `model_name` and `model_version` fields in InferredTransaction specified? [Clarity, Spec §Key Entities - InferredTransaction]
- [ ] CHK022 - Does ConsumerConfig specify all required fields and their valid ranges, including defaults for optional fields (speed2, iterations, seed)? [Completeness, Spec §Key Entities - ConsumerConfig]

---

## Acceptance Criteria Quality

- [ ] CHK023 - Is SC-001 ("full range [1, N2_MAX] exercised over multiple iterations") measurable without specifying a minimum iteration count or statistical bound? [Measurability, Spec §SC-001]
- [ ] CHK024 - Is SC-004 ("switch takes effect within one iteration boundary") tied to an observable, testable artifact (e.g., model_version field in returned InferredTransaction)? [Measurability, Spec §SC-004]
- [ ] CHK025 - Does SC-005 ("Consumer stops within one iteration") define the observable stop signal -- exception, None, structured shutdown, or loop exit? [Clarity, Spec §SC-005]
- [ ] CHK026 - Are US3 acceptance scenarios (AC3: best-effort alarms) consistent with FR-013's requirement that alarm errors are reported after Buffer2 write completes? [Consistency, Spec §US3 AC3 vs FR-013]

---

## Edge Case Coverage

- [ ] CHK027 - Is the behavior specified when N2_MAX is configured as 0 -- which error type is raised and at which point (config validation vs. runtime)? [Completeness, Spec §FR-003]
- [ ] CHK028 - Are requirements defined for a batch where all transactions are fraudulent (100% alarm trigger rate), and its effect on timing/throughput guarantees? [Coverage, Gap]
- [ ] CHK029 - Is Consumer behavior specified when speed2 is set to zero (no delay between iterations), particularly regarding Buffer1 pressure? [Coverage, Gap]

---

## Notes

- Check items off as completed: `[x]`
- Add findings inline (e.g., `[x] CHK003 - Resolved: FR-013 clarification added`)
- Items marked `[Gap]` indicate missing requirements -- add to spec.md before implementation
- Items marked `[Conflict]` require explicit resolution in spec.md before tasks begin
