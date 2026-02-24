# Feature Specification: Logger Batch Persistence

**Feature Branch**: `005-logger-transactions`
**Created**: 2026-02-23
**Status**: Draft
**Input**: User description: "Logger extracts from Buffer2 batches of inferred_transaction at speed3. Batches are of size N3 which vary at each iteration [1, N3_MAX]. Logger writes pending_transactions. pending_transaction = inferred_transaction + {prediction_confirmed (=F)}. The field prediction_confirmed of each transaction is set to False. The field prediction_confirmed will be updated later once the transaction is fully checked. The Logger has no idea about Buffer2 nor Storage implementation (decoupling with ports and adapters). Similarly to Producer, the Logger runs asynchronously until the app ends."

## Clarifications

### Session 2026-02-23

- Q: PendingTransaction composition or flat struct? → A: Composition -- wraps `InferredTransaction` (consistent with `InferredTransaction` wrapping `Transaction`).
- Q: Storage error type: reuse BufferError or new enum? → A: New `StorageError` enum with distinct variant names (`Unavailable`, `CapacityExceeded`) -- Storage is not a buffer.
- Q: Include Logger integration in main.rs (concurrent pipeline) in this feature? → A: Yes, full end-to-end including wiring into `tokio::join!`.

## User Scenarios & Testing

### User Story 1 - Read Batches from Buffer2 (Priority: P1)

The Logger reads variable-size batches of inferred transactions from Buffer2. Each iteration, the batch size N3 is randomly chosen in [1, N3_MAX]. Reading happens at speed3 (a configurable delay between iterations). If Buffer2 is empty, the Logger waits for data. If Buffer2 is closed and drained, the Logger stops.

**Why this priority**: Without reading from Buffer2, no downstream persistence can occur. This is the entry point of the Logger pipeline.

**Independent Test**: Can be tested by wiring a Buffer2Read implementation pre-loaded with inferred transactions and verifying the Logger extracts batches of correct variable sizes within the configured range.

**Acceptance Scenarios**:

1. **Given** Buffer2 contains 50 inferred transactions and N3_MAX is 10, **When** Logger reads one batch, **Then** the batch contains between 1 and 10 inferred transactions.
2. **Given** Buffer2 contains 3 inferred transactions and N3_MAX is 20, **When** Logger reads one batch, **Then** the batch contains between 1 and 3 transactions (capped by available data).
3. **Given** Buffer2 is empty and open, **When** Logger attempts to read, **Then** Logger waits without error.
4. **Given** Buffer2 is closed and empty, **When** Logger attempts to read, **Then** Logger stops gracefully.

---

### User Story 2 - Transform to PendingTransaction (Priority: P1)

After reading a batch of inferred transactions from Buffer2, the Logger converts each one into a pending transaction by adding `is_reviewed = false` and `actual_fraud = None`. All original fields from the inferred transaction are preserved. `is_reviewed` and `actual_fraud` are updated later by an external review process (out of scope for this feature).

**Why this priority**: This transformation is the core business logic of the Logger. Without it, inferred transactions cannot be persisted in the correct format for downstream verification.

**Independent Test**: Can be tested by providing a batch of inferred transactions and verifying each resulting pending transaction carries all original fields plus `is_reviewed = false` and `actual_fraud = None`.

**Acceptance Scenarios**:

1. **Given** a batch of 5 inferred transactions, **When** Logger transforms them, **Then** 5 pending transactions are produced, each with `is_reviewed = false`, `actual_fraud = None`, and all original fields intact.
2. **Given** an inferred transaction with `predicted_fraud = true`, **When** Logger transforms it, **Then** the pending transaction retains `predicted_fraud = true` and has `is_reviewed = false`, `actual_fraud = None`.
3. **Given** an inferred transaction with `predicted_fraud = false`, **When** Logger transforms it, **Then** the pending transaction retains `predicted_fraud = false` and has `is_reviewed = false`, `actual_fraud = None`.

---

### User Story 3 - Persist to Storage (Priority: P1)

After transformation, the Logger writes each batch of pending transactions to Storage through a hexagonal port. The Logger has no knowledge of the Storage implementation -- it could be an in-memory collection, a file, or a database.

**Why this priority**: Persistence is the final output of the pipeline. Without it, classified transactions are lost.

**Independent Test**: Can be tested by providing a mock Storage and verifying all pending transactions from the batch are written.

**Acceptance Scenarios**:

1. **Given** a batch of 8 pending transactions, **When** Logger writes to Storage, **Then** all 8 transactions appear in Storage.
2. **Given** Storage is full, **When** Logger attempts to write, **Then** `StorageError::CapacityExceeded` propagates to the caller.
3. **Given** Storage is unavailable, **When** Logger attempts to write, **Then** `StorageError::Unavailable` propagates to the caller.

---

### User Story 4 - Continuous Async Loop (Priority: P1)

The Logger runs as a continuous async loop: read batch, transform, persist, wait speed3, repeat. It runs indefinitely by default (no iteration limit). An optional iteration limit can be set for testing. The Logger stops gracefully when Buffer2 signals closed-and-drained.

**Why this priority**: The run loop ties US1-US3 together into the complete Logger lifecycle required by the concurrent pipeline.

**Independent Test**: Can be tested by configuring a finite iteration limit and verifying the Logger performs exactly that many read-transform-persist cycles, then stops.

**Acceptance Scenarios**:

1. **Given** Logger configured with iteration limit 3 and Buffer2 has enough data, **When** Logger runs, **Then** exactly 3 read-transform-persist cycles execute.
2. **Given** Logger configured with no iteration limit and Buffer2 is closed after 5 batches, **When** Logger runs, **Then** Logger stops gracefully after processing 5 batches.
3. **Given** Logger configured with speed3 = 200ms, **When** Logger processes iterations, **Then** there is a delay of approximately 200ms between each iteration.

---

### Edge Cases

- What happens when Buffer2 returns a partial batch smaller than the requested N3?
  - Logger processes whatever is available (between 1 and the available count).
- What happens when N3_MAX is 1?
  - Logger reads exactly 1 inferred transaction per iteration (valid configuration).
- What happens when Storage returns an error mid-pipeline?
  - The error propagates immediately to the caller; the current batch is not partially written.
- What happens when Buffer2 is closed but still has unread data?
  - Logger continues draining remaining data. It stops only when Buffer2 is both closed and empty.

## Requirements

### Functional Requirements

- **FR-001**: Logger MUST read batches from Buffer2 through a hexagonal port (read trait), with no dependency on Buffer2 implementation.
- **FR-002**: Logger MUST vary batch size N3 randomly in [1, N3_MAX] at each iteration.
- **FR-003**: Logger MUST accept N3_MAX as configuration (minimum value: 1).
- **FR-004**: Logger MUST create one pending transaction per inferred transaction, setting `is_reviewed = false` and `actual_fraud = None`.
- **FR-005**: Logger MUST preserve all fields from the source inferred transaction in the resulting pending transaction (id, amount, last_name, predicted_fraud, model_name, model_version).
- **FR-006**: Logger MUST write each batch of pending transactions to Storage through a hexagonal port (storage trait), with no dependency on Storage implementation.
- **FR-007**: Logger MUST operate at speed3, defined as a configurable delay (duration) between processing iterations.
- **FR-008**: Logger MUST run indefinitely by default (no iteration limit). An optional iteration limit MAY be configured for testing.
- **FR-009**: Logger MUST stop gracefully when Buffer2 is closed and fully drained, returning success (not an error).
- **FR-010**: Logger MUST propagate errors from Buffer2 (`BufferError`) and Storage (`StorageError`) ports to the caller immediately.
- **FR-011**: Logger MUST log iteration metadata (batch size, iteration number) using the log facade.

### Key Entities

- **PendingTransaction**: Composition struct wrapping `InferredTransaction` plus `is_reviewed: bool` (initially false) and `actual_fraud: Option<bool>` (initially None). Follows the same nesting pattern as `InferredTransaction { transaction: Transaction, ... }`. Represents a transaction awaiting human review and ground-truth labeling.
- **Buffer2Read**: Hexagonal port for reading batches of inferred transactions from the second buffer. Symmetric to Buffer1Read.
- **Storage**: Hexagonal port for writing batches of pending transactions to a persistent store. The Logger depends exclusively on this trait. Returns `StorageError` (not `BufferError`) with variants `CapacityExceeded` and `Unavailable`.
- **StorageError**: Dedicated error enum for the Storage port. Variants: `CapacityExceeded { capacity: usize }` (store is full), `Unavailable` (store is closed/unreachable). Distinct from `BufferError` to reinforce the conceptual boundary between buffers and storage.
- **LoggerConfig**: Configuration for the Logger. Contains N3_MAX (maximum batch size), speed3 (delay between iterations), and an optional iteration limit.

## Success Criteria

### Measurable Outcomes

- **SC-001**: Logger processes batches with variable sizes correctly, verified by the full range [1, N3_MAX] being exercised over multiple iterations.
- **SC-002**: 100% of inferred transactions read from Buffer2 are converted to pending transactions with `is_reviewed = false` and `actual_fraud = None`.
- **SC-003**: All pending transactions in each batch are persisted to Storage after transformation.
- **SC-004**: Logger stops within one iteration after Buffer2 signals closed and is drained.
- **SC-005**: Both hexagonal ports (Buffer2Read, Storage) are swappable without modifying Logger domain logic.
- **SC-006**: Logger is wired into `main.rs` via `tokio::join!` alongside Producer, Consumer, and Modelizer; runs end-to-end without blocking other components.

## Assumptions

- **speed3** follows the same pattern as Producer's speed1 and Consumer's speed2: a configurable async delay (duration) inserted between processing iterations.
- **Buffer2Read** is a new trait in the domain crate, symmetric to Buffer1Read, returning batches of `InferredTransaction`.
- **Storage** is a new trait in the domain crate for writing batches of `PendingTransaction`. It uses a dedicated `StorageError` enum (`CapacityExceeded`, `Unavailable`) rather than reusing `BufferError`.
- **PendingTransaction** is a new domain type in the domain crate using composition: `PendingTransaction { inferred_transaction: InferredTransaction, is_reviewed: bool, actual_fraud: Option<bool> }`.
- **Error handling**: Buffer2 and Storage errors propagate immediately. No retry logic in the Logger.
- **Logger crate** is a new library crate in the workspace, following the same structure as producer and consumer crates.
- **Iteration logging** uses the `log` facade crate at debug level, consistent with other pipeline components.
- **Pipeline integration** is in scope: Logger is wired into `main.rs` via `tokio::join!` with a ConcurrentBuffer2 (Buffer2Read adapter) and an InMemoryStorage adapter, following the pattern established in feature 004.
