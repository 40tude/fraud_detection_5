# Architecture Checklist: Producer -- Transaction Generation

**Purpose**: Retrospective review of architectural contract quality in spec.md -- are hexagonal architecture, port boundaries, DI, and error/backpressure requirements clearly specified?
**Created**: 2026-02-22
**Feature**: [spec.md](../spec.md)
**Scope**: Requirements specification quality only. Implementation concerns (crate names, Rust syntax, lint config) are explicitly excluded.

---

## Port Contract Completeness

- [ ] CHK001 - Does the spec define the synchrony model (sync vs. async) of the Buffer1 port, or is this implicit in design artifacts only? [Completeness, Gap]
- [ ] CHK002 - Is the Buffer1 port contract self-contained in the spec, or does it depend on a separate contracts/ artifact that is not referenced by spec requirements? [Completeness, Spec §FR-007]
- [ ] CHK003 - Does the spec define the mutation model of the Buffer1 port -- specifically, whether implementations are permitted to mutate internal state during writes? [Completeness, Gap]
- [ ] CHK004 - Are the behavioral postconditions of `write_batch` (ordering guarantees, atomicity of batch writes) specified as requirements in the spec? [Completeness, Gap]

## Dependency Inversion Requirements

- [ ] CHK005 - Does spec §FR-004 define "no concrete buffer dependency" with an objectively measurable criterion (e.g., no import of adapter types in Producer source)? [Measurability, Spec §FR-004]
- [ ] CHK006 - Is the injection mechanism (constructor injection, generic parameter, or trait object) for the Buffer1 port specified as a requirement in the spec? [Clarity, Spec §FR-004]
- [ ] CHK007 - Does the spec require that adapters live outside the domain and producer components? [Completeness, Gap]
- [ ] CHK008 - Is the "exclusively through a trait-defined port" language in §FR-004 consistent with the simpler "write them to a buffer" language in §US2, or does the mismatch create ambiguity? [Consistency, Spec §FR-004, §US2]

## Architectural Boundary Definitions

- [ ] CHK009 - Does the spec define component ownership rules for domain entities (which component defines Transaction, BufferError, Buffer1) as an architectural requirement? [Completeness, Gap]
- [ ] CHK010 - Is the boundary between the Producer component and the pipeline orchestrator (wiring layer) specified in the spec? [Completeness, Gap]
- [ ] CHK011 - Does the spec define at which architectural layer the continuous production loop (§FR-006) must reside -- domain logic or application orchestration? [Clarity, Spec §FR-006]
- [ ] CHK012 - Is the hexagonal architecture requirement stated in spec.md itself, or does the spec implicitly rely on the constitution without surfacing it as an explicit requirement? [Completeness, Spec §FR-004]

## Error and Backpressure Contract

- [ ] CHK013 - Does the spec define the error propagation chain (buffer error to producer error to application error) as a requirement, or is this left to design artifacts? [Completeness, Gap]
- [ ] CHK014 - Is FR-007's reference to "the buffer trait contract" sufficient for a reader who has not read the contracts/ artifact, or does the spec need to define the backpressure policy inline? [Clarity, Spec §FR-007]
- [ ] CHK015 - Does the spec specify who owns the retry decision when the buffer reports a recoverable full condition -- the Producer or its caller? [Clarity, Spec §FR-007]
- [ ] CHK016 - Does the spec distinguish between terminal and recoverable buffer error conditions as a requirement, or is this distinction only in the contracts/ artifact? [Completeness, Spec §FR-007, Gap]

## Schema Evolution and Stability

- [ ] CHK017 - Does the spec define a stability contract for the Transaction schema -- specifically, what constitutes a breaking vs. additive change? [Completeness, Gap]
- [ ] CHK018 - Does "schema will change" (§FR-002) include requirements for downstream component compatibility during evolution? [Completeness, Spec §FR-002]
- [ ] CHK019 - Is the mechanism for schema evolution (e.g., optional fields, wrapper types) specified as a requirement, or deferred entirely to future feature specs? [Clarity, Spec §FR-002]
- [ ] CHK020 - Does the spec define how Buffer1 port stability is maintained when the Transaction schema changes? [Completeness, Gap]

## Requirement Consistency and Measurability

- [ ] CHK021 - Is SC-003 ("swapping the adapter requires zero changes to Producer domain code") measurable as stated, or does it need a defined verification method? [Measurability, Spec §SC-003]
- [ ] CHK022 - Does the spec consistently use a single term ("port", "trait", "interface") for the Buffer1 abstraction, or does mixed terminology create ambiguity? [Consistency, Spec §FR-004, §FR-007, §Key Entities]
- [ ] CHK023 - Is SC-005 ("understand the architecture within 10 minutes") an objectively measurable criterion, or is it too subjective to serve as a formal success criterion? [Measurability, Spec §SC-005]

## Notes

- Items marked [Gap] indicate requirements absent from spec.md that exist only in plan.md, data-model.md, or contracts/ -- evaluate whether the gap is intentional.
- This checklist is retrospective: findings inform future feature specs (002-consumer, etc.), not changes to this feature's implementation.
- Exclude items related to Rust syntax, crate tooling, or lint configuration -- these are implementation concerns outside spec scope.
