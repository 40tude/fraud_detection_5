# Pre-Implementation Checklist: Logger Batch Persistence

**Purpose**: Validate spec/plan/tasks completeness and quality before writing any code -- author self-review gate
**Created**: 2026-02-23
**Feature**: [spec.md](../spec.md) | [plan.md](../plan.md) | [tasks.md](../tasks.md)
**Scope**: All layers -- domain types, logger crate, adapters, pipeline integration
**Shutdown cascade items are MANDATORY GATES** (marked [GATE] -- must be resolved before Phase 8)

---

## Requirement Completeness

- [ ] CHK001 - Are `LoggerConfig` field defaults (poll_interval3 initial value, iterations initial value) explicitly specified in the spec, or only inferred from prior crate conventions? [Completeness, Gap, Spec §FR-007, FR-008]
- [ ] CHK002 - Is the builder pattern (`LoggerConfig::builder`) a stated spec requirement, or only implied by plan.md and producer/consumer precedent? [Completeness, Gap]
- [ ] CHK003 - Are `LoggerError` variant names and wrapping semantics (Read wraps BufferError, Write wraps StorageError) specified in the spec, or only in plan/tasks? [Completeness, Gap]
- [ ] CHK004 - Is the `PendingTransaction::id()` convenience method justified by a user scenario, or is it a technical convenience not grounded in requirements? [Completeness, Spec §Key Entities]
- [ ] CHK005 - Are adapter-level requirements (ConcurrentBuffer2 yield-on-empty behavior, InMemoryStorage capacity semantics) specified anywhere in the spec, or only in tasks.md? [Completeness, Gap]
- [ ] CHK006 - Is it explicitly documented that InMemoryStorage never returns `StorageError::Unavailable` -- as a requirement or stated assumption? [Completeness, Spec §Assumptions]

---

## Requirement Clarity

- [ ] CHK007 - Is the RNG distribution for batch size N3 specified (uniform), or is "randomly chosen in [1, N3_MAX]" ambiguous about distribution? [Clarity, Spec §FR-002]
- [ ] CHK008 - Does FR-011 specify the log level (debug vs. info), the exact log key format, and which call site (log_once vs. run) emits each entry? [Ambiguity, Spec §FR-011]
- [ ] CHK009 - Is "delay between iterations" (poll_interval3) precisely defined -- does it apply before the first iteration, after the last, or strictly between consecutive iterations? [Clarity, Spec §FR-007]
- [ ] CHK010 - Is `StorageError::CapacityExceeded { capacity: usize }` clearly defined -- does `capacity` represent the total store capacity or remaining available space? [Ambiguity, Spec §Key Entities]
- [ ] CHK011 - Is the conceptual boundary between `BufferError::Closed` (buffer protocol) and `StorageError::Unavailable` (storage protocol) clearly articulated with distinct semantics? [Clarity, Spec §Key Entities]
- [ ] CHK012 - Is SC-004 ("stops within one iteration") precisely defined -- does the current in-flight iteration complete before the Logger exits the loop? [Clarity, Spec §SC-004]

---

## Requirement Consistency

- [ ] CHK013 - Does FR-005's explicit field list (id, amount, last_name, predicted_fraud, model_name, model_version) match the actual InferredTransaction fields from the feature-003 domain crate? [Consistency, Spec §FR-005]
- [ ] CHK014 - Are log message format strings consistent across spec (FR-011), quickstart.md, and tasks.md (T031/T032) -- same key names and levels in all three? [Consistency, Spec §FR-011]
- [ ] CHK015 - Is FR-008 (optional iteration limit) consistent with US4 acceptance scenario 1 (iterations=Some(3)) -- same semantics, same type? [Consistency, Spec §FR-008, §US4]
- [ ] CHK016 - Does the PendingTransaction composition structure specified in Key Entities match the clarification answer (composition, not flat) and the InferredTransaction nesting pattern? [Consistency, Spec §Key Entities, §Clarifications]

---

## Acceptance Criteria Quality

- [ ] CHK017 - Can SC-001 ("full range [1, N3_MAX] exercised over multiple iterations") be objectively verified -- is the required number of iterations to satisfy this criterion specified? [Measurability, Spec §SC-001]
- [ ] CHK018 - Can SC-005 ("hexagonal ports are swappable without modifying Logger domain logic") be objectively measured -- what constitutes concrete evidence of swappability? [Measurability, Spec §SC-005]
- [ ] CHK019 - Do the P1 acceptance scenarios (US1-US4) collectively cover FR-001 through FR-011 without gaps -- is every FR traceable to at least one scenario? [Completeness, Spec §US1-US4]

---

## Scenario Coverage

- [ ] CHK020 - Are requirements defined for the partial-read scenario (Buffer2 returns fewer items than requested N3)? [Coverage, Spec §Edge Cases]
- [ ] CHK021 - Are requirements defined for what happens if Logger's run() returns an error while Producer and Consumer are still running in the concurrent pipeline? [Coverage, Exception Flow, Gap]
- [ ] CHK022 - Are requirements for N3_MAX=1 (minimum valid boundary) explicitly covered, not just mentioned as an edge case? [Coverage, Spec §Edge Cases]
- [ ] CHK023 - Are requirements defined for concurrent access to Buffer2 -- Consumer writing while Logger reads -- or is thread-safety handled purely by architecture assumption? [Coverage, NFR, Gap]

---

## Edge Case Coverage

- [ ] CHK024 - Are atomicity semantics for batch writes to Storage defined -- is partial persistence (some items stored, error mid-batch) explicitly prohibited? [Edge Case, Spec §FR-010, §Edge Cases]
- [ ] CHK025 - Is the drain-after-close behavior (Logger continues reading until Buffer2 is both closed AND empty) explicitly required, not only listed as an edge case note? [Coverage, Spec §Edge Cases]
- [ ] CHK026 - Is zero-delay poll_interval3 (Duration::ZERO) defined as a valid configuration, and is any associated behavior (e.g., immediate iteration) specified? [Edge Case, Gap]

---

## [GATE] Shutdown Cascade Requirements

> **MANDATORY**: All three items below must be resolved (or explicitly accepted as assumptions) before Phase 8 (pipeline integration) begins.

- [ ] CHK027 [GATE] - Is the shutdown cascade responsibility explicitly specified as a requirement -- FR-009 says "stops when Buffer2 is closed" but does not name which component closes buffer2 or when? [GATE, Gap, Spec §FR-009]
- [ ] CHK028 [GATE] - Is SC-006 sufficient as a pipeline integration requirement, or must it explicitly state that Logger shutdown is triggered by Consumer completing (cascade), not by CTRL+C directly? [GATE, Completeness, Spec §SC-006]
- [ ] CHK029 [GATE] - Is the CTRL+C path explicitly specified (only buffer1.close() called; buffer2 cascade follows from consumer_then_close), or is this derived by analogy with feature 004 without spec coverage? [GATE, Coverage, Spec §Assumptions]

---

## Non-Functional Requirements

- [ ] CHK030 - Is the single-thread (current_thread Tokio) constraint stated as a requirement or only as an implicit platform assumption -- and are its consequences for adapter design (RefCell, no Sync) documented? [NFR, Assumption]
- [ ] CHK031 - Are concurrency safety requirements for new adapters (ConcurrentBuffer2, InMemoryStorage) documented, or assumed from platform constraints only? [NFR, Assumption, Spec §Assumptions]

---

## Dependencies & Assumptions

- [ ] CHK032 - Is the assumption that Buffer2 (write trait) already exists from feature 004 and requires no modification explicitly validated and documented? [Assumption, Spec §Assumptions]
- [ ] CHK033 - Are backward-compatibility requirements defined -- must all 61 existing tests from features 001-004 remain passing after adding new domain types? [Dependency, Gap]
- [ ] CHK034 - Is the `log` facade (not a concrete logging backend) confirmed as the correct abstraction for all log emissions in the spec or assumptions? [Assumption, Spec §Assumptions]

---

## Ambiguities & Conflicts

- [ ] CHK035 - Is there a potential conflict between FR-010 ("propagate errors immediately") and the edge case note ("current batch not partially written") -- does the spec define atomicity semantics for batch writes? [Conflict, Spec §FR-010, §Edge Cases]
- [ ] CHK036 - Is the FR-011 log-level split (debug for batch_size in log_once, info for iteration number in run) specified in the spec, or only decomposed in tasks.md (T026/T031) without explicit spec backing? [Ambiguity, Spec §FR-011]
- [ ] CHK037 - Is "gracefully" in FR-009 ("stops gracefully") defined with measurable criteria that distinguish graceful from non-graceful termination? [Ambiguity, Spec §FR-009]

---

## Notes

- Check items off as completed: `[x]`
- GATE items (CHK027-CHK029) block Phase 8; resolve or formally accept as assumptions before integration work
- Add inline findings: `- [ ] CHK0XX ... -- Finding: <text>`
- Items reference spec sections for direct lookup; `[Gap]` marks absent requirements
