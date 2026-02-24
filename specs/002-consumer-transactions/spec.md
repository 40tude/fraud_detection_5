# Feature Specification: Consumer Batch Processing

**Feature Branch**: `002-consumer-transactions`
**Created**: 2026-02-22
**Status**: Draft
**Input**: User description: "Consumer extracts from Buffer1 batches of transactions every poll_interval2. Batches are of size N2 which vary at each iteration [1, N2_MAX]. Consumer sends each batch to Modelizer (implementation unknown). When batch returned from Modelizer, Consumer generates alarm for every transaction marked as fraudulent (alarm implementation unknown). Consumer then writes processed batch into Buffer2 (implementation unknown). At any time, Consumer can switch model version (N or N-1) used by Modelizer."

## Clarifications

### Session 2026-02-22

- Q: When an alarm fails for one fraudulent transaction, should remaining alarms be skipped and Buffer2 write aborted? -> A: Best-effort alarms -- attempt all alarms, collect failures, then always write batch to Buffer2.
- Q: How is the model version switch triggered? -> A: Consumer asks the Modelizer to switch via the Modelizer port. The Modelizer owns its version state; the Consumer issues a switch command through the existing hexagonal port.

## User Scenarios & Testing

### User Story 1 - Read Batches from Buffer1 (Priority: P1)

The Consumer reads variable-size batches of transactions from Buffer1. Each iteration, the batch size N2 is randomly chosen in [1, N2_MAX]. Reading happens every poll_interval2 (a configurable delay between iterations). If Buffer1 is empty, the Consumer waits for data. If Buffer1 is closed and drained, the Consumer stops.

**Why this priority**: Without reading from Buffer1, no downstream processing can occur. This is the entry point of the Consumer pipeline.

**Independent Test**: Can be tested by wiring a Buffer1 implementation pre-loaded with transactions and verifying the Consumer extracts batches of correct variable sizes within the configured range.

**Acceptance Scenarios**:

1. **Given** Buffer1 contains 50 transactions and N2_MAX is 10, **When** Consumer reads one batch, **Then** the batch contains between 1 and 10 transactions.
2. **Given** Buffer1 contains 5 transactions and N2_MAX is 20, **When** Consumer reads one batch, **Then** the batch contains between 1 and 5 transactions (capped by available data).
3. **Given** Buffer1 is empty and open, **When** Consumer attempts to read, **Then** Consumer waits without error.
4. **Given** Buffer1 is closed and empty, **When** Consumer attempts to read, **Then** Consumer stops gracefully.

---

### User Story 2 - Send Batches to Modelizer for Inference (Priority: P1)

After reading a batch from Buffer1, the Consumer sends it to the Modelizer for classification. The Modelizer returns the batch with each transaction enriched with inference fields: `predicted_fraud` (true/false), `model_name`, and `model_version`. The Consumer does not know the Modelizer implementation -- it interacts through a hexagonal port.

**Why this priority**: Inference is the core value of the pipeline. Without it, transactions flow through unclassified.

**Independent Test**: Can be tested by providing a mock Modelizer that marks known transactions as fraudulent and verifying the Consumer receives enriched transactions.

**Acceptance Scenarios**:

1. **Given** a batch of 5 transactions, **When** Consumer sends it to the Modelizer, **Then** Consumer receives back 5 inferred transactions each carrying `predicted_fraud`, `model_name`, and `model_version`.
2. **Given** the Modelizer returns an error, **When** Consumer processes the batch, **Then** the error propagates to the caller.

---

### User Story 3 - Generate Alarms for Fraudulent Transactions (Priority: P2)

After receiving inferred transactions from the Modelizer, the Consumer inspects each transaction. For every transaction where `predicted_fraud` is true, the Consumer triggers an alarm. The alarm mechanism is a hexagonal port -- it could be a terminal message, email, or any other notification channel.

**Why this priority**: Alarms are the primary business output of fraud detection. However, the pipeline can function (store results) without alarms.

**Independent Test**: Can be tested by providing a mock Alarm port and verifying it is called exactly once for each fraudulent transaction in a batch, and zero times for legitimate transactions.

**Acceptance Scenarios**:

1. **Given** a batch of 10 inferred transactions where 3 are marked fraudulent, **When** Consumer processes alarms, **Then** exactly 3 alarms are triggered.
2. **Given** a batch where no transactions are fraudulent, **When** Consumer processes alarms, **Then** no alarms are triggered.
3. **Given** the alarm port fails for 1 of 3 fraudulent transactions, **When** Consumer processes alarms, **Then** all 3 alarms are attempted, failures are collected, and the batch is still written to Buffer2.

---

### User Story 4 - Write Processed Batches to Buffer2 (Priority: P1)

After inference and alarm processing, the Consumer writes the entire processed batch (all inferred transactions, both legitimate and fraudulent) into Buffer2. Buffer2 is a hexagonal port -- the Consumer has no knowledge of its implementation.

**Why this priority**: Persisting inferred transactions to Buffer2 is essential for downstream Logger consumption.

**Independent Test**: Can be tested by providing a mock Buffer2 and verifying all inferred transactions from the batch are written regardless of fraud status.

**Acceptance Scenarios**:

1. **Given** a processed batch of 8 inferred transactions, **When** Consumer writes to Buffer2, **Then** all 8 transactions appear in Buffer2.
2. **Given** Buffer2 is full, **When** Consumer attempts to write, **Then** a capacity error propagates to the caller.
3. **Given** Buffer2 is closed, **When** Consumer attempts to write, **Then** a closed error propagates to the caller.

---

### User Story 5 - Switch Model Version at Runtime (Priority: P2)

The Consumer can ask the Modelizer to switch between model version N (latest) and N-1 (previous) at any time via the Modelizer port. The Modelizer owns its version state; the Consumer issues a switch command. The switch takes effect on the next batch sent to the Modelizer. The default model version is the latest (N).

**Why this priority**: Version switching enables A/B comparison and rollback, but the pipeline functions correctly with a single version.

**Independent Test**: Can be tested by switching the model version between iterations and verifying the Modelizer receives the updated version selection for subsequent batches.

**Acceptance Scenarios**:

1. **Given** Consumer is using model version N, **When** Consumer asks Modelizer to switch to N-1, **Then** the next batch inferred by the Modelizer uses version N-1.
2. **Given** Consumer is using model version N-1, **When** Consumer asks Modelizer to switch to N, **Then** the next batch inferred by the Modelizer uses version N.
3. **Given** no version switch has been requested, **When** Consumer starts, **Then** Modelizer uses model version N (latest) by default.

---

### Edge Cases

- What happens when Buffer1 returns a partial batch smaller than the requested N2?
  - Consumer processes whatever is available (between 1 and the available count).
- What happens when the Modelizer is unreachable or returns an error?
  - The error propagates; the batch is not written to Buffer2 or alarmed.
- What happens when a model version switch is requested during an in-flight batch?
  - The switch takes effect only after the current batch completes processing.
- What happens when N2_MAX is 1?
  - Consumer reads exactly 1 transaction per iteration (valid configuration).
- What happens when alarm generation fails for one transaction in a batch?
  - Best-effort: Consumer attempts all alarms in the batch, collects failures, then writes the batch to Buffer2 regardless. Alarm failures are reported to the caller after the batch completes.

## Requirements

### Functional Requirements

- **FR-001**: Consumer MUST read batches from Buffer1 through a hexagonal port (read trait), with no dependency on Buffer1 implementation.
- **FR-002**: Consumer MUST vary batch size N2 randomly in [1, N2_MAX] at each iteration.
- **FR-003**: Consumer MUST accept N2_MAX as configuration (minimum value: 1).
- **FR-004**: Consumer MUST send each read batch to the Modelizer through a hexagonal port (inference trait).
- **FR-005**: Consumer MUST receive inferred transactions from the Modelizer, each enriched with `predicted_fraud`, `model_name`, and `model_version`.
- **FR-006**: Consumer MUST attempt one alarm per fraudulent transaction through a hexagonal port (alarm trait), using best-effort delivery (all alarms attempted even if some fail).
- **FR-007**: Consumer MUST write each fully-processed batch to Buffer2 through a hexagonal port (write trait), regardless of alarm failures.
- **FR-008**: Consumer MUST ask the Modelizer to switch version between N and N-1 at runtime, through the Modelizer hexagonal port. The Modelizer owns its version state.
- **FR-009**: Modelizer MUST default to model version N (latest) at startup.
- **FR-010**: A model version switch issued by the Consumer MUST take effect on the next batch inferred by the Modelizer, not the current in-flight batch.
- **FR-011**: Consumer MUST operate every poll_interval2, defined as a configurable delay between processing iterations.
- **FR-012**: Consumer MUST stop gracefully when Buffer1 is closed and fully drained.
- **FR-013**: Consumer MUST propagate errors from Buffer1, Modelizer, and Buffer2 ports to the caller immediately. Alarm errors MUST be collected across the batch and reported after Buffer2 write completes.

### Key Entities

- **InferredTransaction**: A transaction enriched with inference results. Carries all fields of Transaction plus `predicted_fraud` (boolean), `model_name` (string), and `model_version` (string). Represents the output of the Modelizer.
- **ModelVersion**: Represents the selectable model version. Two variants: N (latest) and N-1 (previous).
- **ConsumerConfig**: Configuration for the Consumer. Contains N2_MAX (maximum batch size) and poll_interval2 (delay between iterations), plus optional iteration limit.
- **Buffer1 (read side)**: Hexagonal port for reading batches of transactions from the first buffer.
- **Buffer2 (write side)**: Hexagonal port for writing batches of inferred transactions to the second buffer.
- **Modelizer**: Hexagonal port for sending a batch of transactions and receiving inferred transactions. Also exposes a version-switch command; the Modelizer owns its version state internally.
- **Alarm**: Hexagonal port for triggering a fraud alert on a single inferred transaction.

## Success Criteria

### Measurable Outcomes

- **SC-001**: Consumer processes batches with variable sizes correctly verified by the full range [1, N2_MAX] being exercised over multiple iterations.
- **SC-002**: 100% of fraudulent transactions in a batch trigger exactly one alarm each; 0% of legitimate transactions trigger alarms.
- **SC-003**: All inferred transactions (both fraudulent and legitimate) are written to Buffer2 after processing.
- **SC-004**: Model version switch issued by Consumer to Modelizer takes effect within one iteration boundary -- never mid-batch.
- **SC-005**: Consumer stops within one iteration after Buffer1 signals closed and is drained.
- **SC-006**: All four hexagonal ports (Buffer1-read, Modelizer, Alarm, Buffer2-write) are swappable without modifying Consumer domain logic.

## Assumptions

- **poll_interval2** follows the same pattern as Producer's poll_interval1: a configurable async delay (duration) inserted between processing iterations.
- **"At any time"** for model version switching means between iterations, not mid-batch. The switch method can be called at any point, but it only affects the next iteration.
- **Buffer1 read port** is a new trait separate from the existing `Buffer1` write trait. The read side returns batches of up to N requested transactions.
- **InferredTransaction** is a new domain type distinct from `Transaction`, defined in the domain crate.
- **Alarm port** receives one inferred transaction at a time (per-transaction granularity), not a batch. Alarm delivery is best-effort: all alarms in a batch are attempted, failures are collected, and the batch is written to Buffer2 regardless.
- **Error handling**: Buffer1, Modelizer, and Buffer2 errors propagate immediately. Alarm errors are collected (best-effort) and reported after Buffer2 write. No retry logic in the Consumer.
- **Model version state** is owned by the Modelizer, not the Consumer. The Consumer issues switch commands through the Modelizer port.
