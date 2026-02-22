# Ports and Adapters Checklist: Consumer Batch Processing

**Purpose**: Pre-implementation author sanity check -- validates that port trait contracts, mock adapter behavioral specs, and demo adapter requirements are complete and clear enough to implement correctly without ambiguity.
**Created**: 2026-02-22
**Feature**: [spec.md](../spec.md) | [plan.md](../plan.md) | [data-model.md](../data-model.md)
**Scope**: All 4 port traits + 4 mock adapters + 4 demo/production adapters (binary crate)

---

## Port Trait Contracts

- [ ] CHK001 - Is Buffer2's "same error semantics as Buffer1::write_batch" requirement specified precisely rather than by cross-reference? [Clarity, data-model.md §Buffer2]
- [ ] CHK002 - Does the Buffer2 port spec define behavior when `write_batch` is called with an empty `Vec`? [Coverage, Gap]
- [ ] CHK003 - Does the Modelizer port spec define what happens when `switch_version` is called with the already-active version -- silent no-op or error? [Coverage, Gap]
- [ ] CHK004 - Is the naming asymmetry between `Buffer1Read` (explicit "Read" suffix) and `Buffer2` (no "Write" suffix) documented as a deliberate decision? [Clarity, Gap]
- [ ] CHK005 - Is the `#[expect(async_fn_in_trait)]` suppression requirement documented at spec or data-model level, or only implied by tasks? [Completeness, Gap]

---

## ConsumerError-to-Port-Error Mapping

- [ ] CHK006 - Does the spec define which `ConsumerError` variant wraps `ModelizerError::SwitchFailed` when `switch_model_version` fails? [Completeness, data-model.md §ConsumerError]
- [ ] CHK007 - Is the reuse of `ConsumerError::Inference` for both `infer` failures and `switch_version` failures explicitly documented as a deliberate design decision? [Clarity, Gap]
- [ ] CHK008 - Does the spec define how the `run` loop surfaces alarm errors when `consume_once` returns `Ok(non-empty-vec)` -- logged per iteration, accumulated, or returned to caller? [Completeness, Spec §FR-013, Gap]

---

## Mock Adapter Behavioral Specifications

- [ ] CHK009 - Does the spec or data-model define `MockModelizer`'s initial model version state (N at construction, before any `switch_version` call)? [Completeness, Gap]
- [ ] CHK010 - Is `MockAlarm`'s failure mode defined precisely -- which calls fail (all, first N, every Nth, configurable index)? [Clarity, Gap]
- [ ] CHK011 - Does `MockBuffer1Read`'s spec define behavior when the closed flag is set but the internal transaction Vec is not yet empty? [Clarity, Gap]
- [ ] CHK012 - Does `MockBuffer2`'s spec define at which point the optional error mode triggers -- on first write, always, or after N successful writes? [Clarity, Gap]
- [ ] CHK013 - Is `MockModelizer`'s "configurable predicted_fraud flag" defined as applying uniformly to all transactions in a batch, or is per-transaction control required? [Clarity, Gap]

---

## Demo/Production Adapter Behavioral Specifications

- [ ] CHK014 - Is `DemoModelizer`'s config bool for fraud marking documented in data-model.md or spec.md (rather than only in tasks.md T034)? [Completeness, Gap]
- [ ] CHK015 - Are `DemoModelizer`'s hardcoded field values (`model_name="DINN"`, `model_version="v1"`) specified as requirements in the spec, or are they free implementation choices? [Completeness, Gap]
- [ ] CHK016 - Is `InMemoryBuffer2`'s capacity configuration -- the threshold that triggers `BufferError::Full` -- specified in data-model.md? [Completeness, Gap]
- [ ] CHK017 - Is the requirement that `InMemoryBuffer` implements both `Buffer1` (write) and `Buffer1Read` (read) on the same struct specified in spec or plan? [Completeness, Gap]
- [ ] CHK018 - Is `LogAlarm`'s behavioral contract defined with respect to failure -- specifically, is it required to always return `Ok(())`, making `AlarmError::DeliveryFailed` unreachable in demo mode? [Clarity, Gap]
- [ ] CHK019 - Are `LogAlarm`'s log level and message format (e.g., `warn!` with transaction id) specified as requirements or left as free implementation choices? [Clarity, Gap]

---

## Hexagonal Boundary Requirements

- [ ] CHK020 - Does the spec explicitly require all concrete adapter structs to reside in the binary crate (`fraud_detection`), not in `domain` or `consumer`? [Completeness, Spec §plan.md]
- [ ] CHK021 - Is the prohibition on domain and consumer crates depending on adapter implementations documented as an enforced architectural requirement? [Clarity, Gap]

---

## Adapter-Port Contract Alignment

- [ ] CHK022 - Does the `InMemoryBuffer::read_batch` spec require consumed transactions to be removed (drained) from the buffer, ruling out copy semantics? [Clarity, Gap]
- [ ] CHK023 - Does the spec define how `DemoModelizer` satisfies the "switch takes effect on next infer call" requirement (FR-010) -- specifically, what internal state it must maintain? [Completeness, Spec §FR-010]
- [ ] CHK024 - Is it specified whether `DemoModelizer::switch_version` can return `SwitchFailed` in demo mode, or whether it is required to always succeed? [Completeness, Gap]
- [ ] CHK025 - Are the alarm error logging responsibilities inside `Consumer::run` specified in spec or plan, or only discoverable in tasks.md T038? [Completeness, Spec §FR-013, Gap]

---

## Notes

- Check items off as completed: `[x]`
- Add findings inline (e.g., `[x] CHK006 - Resolved: data-model.md updated to clarify ConsumerError::Inference covers switch failures`)
- Items marked `[Gap]` indicate missing requirements -- add to spec.md or data-model.md before Phase 8 tasks begin
- See also: `error-propagation.md` CHK011-CHK018 for port-level error semantics (not duplicated here)
