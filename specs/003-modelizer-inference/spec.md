# Feature Specification: Modelizer Inference

**Feature Branch**: `003-modelizer-inference`
**Created**: 2026-02-23
**Status**: Draft
**Input**: User description: "Modelizer receives batches of transactions from Consumer at speed2. Batches are of size N2 which vary at each iteration [1, N2_MAX]. Modelizer sends back batches of inferred transactions. inferenced_transactions = transactions + {inference (T/F), model_name, version_num}. Consumer can select the revision of the model to be used to make the inferences. Only version N and N-1 are available. By default, the Modelizer uses the latest version of the Model. Modelizer simulates calls to the Model in charge of the inference. In a first version the Model can: Be named 'DEMO', Propose versions 3 and 4, Which respectively detect fraudulent transactions 3% and 4% of the time."

## User Scenarios & Testing

### User Story 1 - Batch Inference (Priority: P1)

The Modelizer receives a batch of transactions from the Consumer and returns a batch of inferred transactions of the same size, in the same order. Each inferred transaction is the original transaction enriched with three fields: a fraud prediction (true/false), the model name, and the model version number.

**Why this priority**: Core pipeline functionality -- nothing else works without inference.

**Independent Test**: Can be fully tested by sending a batch of known transactions through the Modelizer and verifying each output carries the correct enrichment fields plus the original transaction data.

**Acceptance Scenarios**:

1. **Given** a batch of 5 transactions, **When** the Modelizer infers, **Then** 5 inferred transactions are returned, each containing the original transaction plus `predicted_fraud`, `model_name`, and `model_version`.
2. **Given** a batch of 1 transaction, **When** the Modelizer infers, **Then** exactly 1 inferred transaction is returned with all enrichment fields.
3. **Given** an empty batch, **When** the Modelizer infers, **Then** an empty batch is returned (no error).

---

### User Story 2 - Default Model Version (Priority: P2)

The Modelizer uses the latest available model version (N) by default at startup. No explicit version selection is required for normal operation.

**Why this priority**: Ensures the system works out-of-the-box before introducing version switching.

**Independent Test**: Can be tested by creating a Modelizer, running inference without any version switch, and verifying the output carries the latest version identifier.

**Acceptance Scenarios**:

1. **Given** a freshly created Modelizer, **When** inference runs without any version switch, **Then** all inferred transactions carry the latest version number (version 4 for the DEMO model).
2. **Given** a Modelizer with the DEMO model, **When** no switch is called, **Then** `model_name` is "DEMO" and `model_version` reflects version 4.

---

### User Story 3 - Version Switching (Priority: P2)

The Consumer can instruct the Modelizer to switch between model version N (latest) and N-1 (previous). Only these two versions are available. The switch takes effect on the next inference call.

**Why this priority**: Version selection is essential for the pipeline to support model comparison and rollback scenarios.

**Independent Test**: Can be tested by switching to N-1, running inference, and verifying the version in the output changes accordingly.

**Acceptance Scenarios**:

1. **Given** a Modelizer defaulting to version N, **When** Consumer switches to N-1, **Then** subsequent inferences carry the N-1 version identifier.
2. **Given** a Modelizer switched to N-1, **When** Consumer switches back to N, **Then** subsequent inferences carry the N version identifier.
3. **Given** a Modelizer, **When** a switch is requested, **Then** in-progress or previous inference results are not affected; the change applies starting from the next call.

---

### User Story 4 - Probabilistic Fraud Detection (Priority: P3)

The DEMO model classifies transactions as fraudulent based on a configurable probability. Version 3 detects fraud 3% of the time; version 4 detects fraud 4% of the time. Over a sufficiently large sample, the observed fraud rate converges to the target rate.

**Why this priority**: Probabilistic behavior gives the pipeline realistic demo characteristics but is not structurally required.

**Independent Test**: Can be tested by running inference on a large batch (e.g. 10,000 transactions) and verifying the observed fraud rate falls within a statistical confidence interval around the target.

**Acceptance Scenarios**:

1. **Given** version 4 (N) is active and a large batch is inferred, **When** counting fraudulent predictions, **Then** the observed fraud rate approximates 4%.
2. **Given** version 3 (N-1) is active and a large batch is inferred, **When** counting fraudulent predictions, **Then** the observed fraud rate approximates 3%.
3. **Given** a seeded RNG, **When** the same batch is inferred twice, **Then** identical fraud predictions are produced (deterministic replay).

---

### Edge Cases

- What happens when an empty batch is submitted for inference? Returns an empty batch, no error.
- What happens when `switch_version` is called with the already-active version? No-op, no error.
- What happens if the same Modelizer instance is used for many successive batches? Internal state (version, RNG) remains consistent and correct.

## Requirements

### Functional Requirements

- **FR-001**: Modelizer MUST accept a batch of transactions and return a batch of inferred transactions of the same size and order.
- **FR-002**: Each inferred transaction MUST contain: the original transaction, a fraud prediction (true or false), the model name (string), and the model version (string representation of the version number).
- **FR-003**: The DEMO model MUST be named "DEMO".
- **FR-004**: The DEMO model MUST offer exactly two versions: version 3 (N-1) and version 4 (N, latest).
- **FR-005**: DEMO version 3 MUST classify transactions as fraudulent approximately 3% of the time.
- **FR-006**: DEMO version 4 MUST classify transactions as fraudulent approximately 4% of the time.
- **FR-007**: The Modelizer MUST default to the latest version (N = version 4) at startup.
- **FR-008**: The Consumer MUST be able to switch the Modelizer between version N and version N-1. Only these two versions are available.
- **FR-009**: A version switch MUST take effect starting from the next inference call, not the current one.
- **FR-010**: The fraud detection MUST be probabilistic (random per transaction at the configured rate), not deterministic by transaction attribute.
- **FR-011**: The Modelizer MUST support seeded randomness for reproducible test results.
- **FR-012**: The Modelizer MUST depend on a Model hexagonal port (trait defined in the domain crate) -- never on a concrete model implementation. The DEMO model is the first adapter for this port.
- **FR-013**: The Model port MUST operate per-transaction (single classification decision). The Modelizer is responsible for iterating over the batch and delegating each transaction to the Model.
- **FR-014**: The Model port MUST be self-describing: it exposes its own name and active version string. The Modelizer reads these when building each InferredTransaction, rather than maintaining its own metadata mapping.
- **FR-015**: The Model adapter MUST map abstract ModelVersion (N / NMinus1) to its own concrete version numbers. The Modelizer passes ModelVersion values through without interpreting them. For DEMO: N maps to version 4, NMinus1 maps to version 3.
- **FR-016**: The existing DemoModelizer adapter MUST be removed and replaced by a generic Modelizer component (modelizer crate) that delegates to a DEMO Model adapter (fraud_detection crate).

### Key Entities

- **Modelizer**: Pipeline component that receives transaction batches, delegates classification to a Model port, and returns enriched (inferred) transactions. Holds the current model version. Depends on the Model hexagonal port -- never on a concrete model implementation.
- **Model (port)**: Hexagonal port (trait) defined in the domain crate. Represents any classification model the Modelizer can delegate to. Concrete adapters (e.g. DEMO) implement this port. Future adapters may call external services (e.g. MLFlow API).
- **DEMO model (adapter)**: First concrete adapter for the Model port. Named "DEMO", with two versions (3 and 4) and probabilistic fraud detection rates (3% and 4% respectively).
- **InferredTransaction**: A transaction enriched with `predicted_fraud`, `model_name`, and `model_version` fields (already defined in domain).
- **ModelVersion**: Selector for N (latest) or N-1 (previous) (already defined in domain).

## Success Criteria

### Measurable Outcomes

- **SC-001**: Every inferred batch contains exactly as many items as the input batch, in the same order.
- **SC-002**: Over 10,000 inferred transactions with version 4, the observed fraud rate falls within [3%, 5%] (1 percentage point tolerance around 4%).
- **SC-003**: Over 10,000 inferred transactions with version 3, the observed fraud rate falls within [2%, 4%] (1 percentage point tolerance around 3%).
- **SC-004**: After a version switch, the very next inference call reflects the new version in its output.
- **SC-005**: With identical RNG seeds, two Modelizer instances produce identical fraud predictions for the same input.
- **SC-006**: The Modelizer works correctly within the existing pipeline (Producer -> Buffer1 -> Consumer -> Modelizer -> Buffer2) with no changes to Consumer.

## Clarifications

### Session 2026-02-23

- Q: Where should the Model trait (hexagonal port) be defined? → A: In the domain crate, alongside Modelizer, Buffer1, Buffer2, and Alarm traits.
- Q: Should the Model port operate per-transaction or per-batch? → A: Per-transaction. Model classifies one transaction; Modelizer iterates over the batch.
- Q: Who owns model metadata (name, version string)? → A: Self-describing. Model exposes its own name and active version; Modelizer reads them when building InferredTransaction.
- Q: Who maps abstract ModelVersion (N/NMinus1) to concrete versions? → A: The Model adapter. Modelizer passes ModelVersion::N/NMinus1; DEMO adapter maps N=4, NMinus1=3 internally.
- Q: What happens to the existing DemoModelizer adapter? → A: Replace it. Remove DemoModelizer; create a generic Modelizer (modelizer crate) + DEMO Model adapter (fraud_detection crate).

## Assumptions

- The existing `Modelizer` trait in the domain crate and the `InferredTransaction` / `ModelVersion` types are reused as-is.
- The DEMO model version numbers (3 and 4) are presented as strings ("3" and "4") in the `model_version` field of `InferredTransaction`.
- The existing `DemoModelizer` adapter in the `fraud_detection` crate will be removed and replaced by: (1) a generic Modelizer component in the `modelizer` lib crate, and (2) a DEMO Model adapter in the `fraud_detection` binary crate.
- "speed2" refers to the Consumer's iteration cadence; the Modelizer itself does not manage its own pacing (it responds synchronously to Consumer calls via the trait).
- Batch sizes are determined by the Consumer (already implemented in feature 002); the Modelizer processes whatever batch size it receives.
