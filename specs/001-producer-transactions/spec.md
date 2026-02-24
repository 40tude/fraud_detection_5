# Feature Specification: Producer -- Transaction Generation

**Feature Branch**: `001-producer-transactions`
**Created**: 2026-02-21
**Status**: Draft
**Input**: User description: "Producer simulates transactions coming from the outside world. Producer generates and writes batches of transactions every poll_interval1. Batches are of size N1 which vary at each iteration [1, N1_MAX]. In a first version a transaction schema = {UUID, amount, last_name, ...}. The transaction schema will change."

## User Scenarios & Testing *(mandatory)*

### User Story 1 -- Generate a Single Batch of Transactions (Priority: P1)

The Producer generates a batch of random transactions and writes them
to a buffer. The batch size is randomly chosen between 1 and N1_MAX
for each iteration. Each transaction contains at minimum: a unique ID
(UUID), an amount, and a last name.

**Why this priority**: This is the foundational capability. Without
transaction generation, no downstream component can operate. It also
validates the hexagonal architecture boundary between the Producer
domain and the buffer adapter.

**Independent Test**: Can be fully tested by invoking the Producer once
and asserting that a batch of the correct size appears in a test buffer
(in-memory adapter). Each transaction in the batch carries valid UUID,
amount, and last_name fields.

**Acceptance Scenarios**:

1. **Given** a configured Producer with N1_MAX = 10, **When** the
   Producer generates one batch, **Then** the batch contains between 1
   and 10 transactions, each with a valid UUID, an amount between 0.01
   and 10,000.00 (2 decimal places), and a non-empty last_name.
2. **Given** a configured Producer with N1_MAX = 1, **When** the
   Producer generates one batch, **Then** the batch contains exactly 1
   transaction.
3. **Given** a configured Producer, **When** the Producer generates
   multiple batches, **Then** each batch size is independently random
   within [1, N1_MAX].

---

### User Story 2 -- Write Batches to Buffer1 via Port (Priority: P1)

The Producer writes generated transaction batches to Buffer1 through a
trait-defined port. The Producer has no knowledge of the buffer's
concrete implementation.

**Why this priority**: Equally critical as US1 because it validates the
hexagonal architecture contract. The Producer MUST depend only on the
Buffer1 port (trait), never on a concrete adapter.

**Independent Test**: Inject a mock or in-memory buffer adapter that
implements the Buffer1 trait. Generate a batch, write it, then read it
back from the adapter and verify the data matches.

**Acceptance Scenarios**:

1. **Given** a Producer with an in-memory buffer adapter, **When** a
   batch of 5 transactions is generated, **Then** the buffer contains
   exactly those 5 transactions with matching UUIDs.
2. **Given** a Producer with any adapter implementing the Buffer1
   trait, **When** a batch is written, **Then** the Producer has zero
   direct dependencies on the adapter's concrete type.

---

### User Story 3 -- Continuous Production every poll_interval1 (Priority: P2)

The Producer runs in a loop, generating and writing batches
continuously. Between each iteration, the Producer sleeps for a
configurable duration (poll_interval1). Each iteration produces a batch of
random size within [1, N1_MAX].

**Why this priority**: Builds on US1 and US2. Continuous production
enables pipeline throughput evaluation but is not required for
validating the architecture or data model.

**Independent Test**: Start the Producer, let it run for a fixed number
of iterations (e.g., 5), then assert that the buffer received 5
batches, each of valid size, and that the Producer can be stopped
cleanly.

**Acceptance Scenarios**:

1. **Given** a Producer configured with N1_MAX = 20 and 5 iterations,
   **When** the Producer runs, **Then** 5 batches are written to the
   buffer, each containing between 1 and 20 transactions.
2. **Given** a running Producer, **When** the buffer returns
   `BufferError::Closed`, **Then** the Producer exits its loop and
   returns `Ok(())` with no data loss.

---

### Edge Cases

- What happens when N1_MAX = 0? The Producer MUST reject this
  configuration at startup.
- What happens when the buffer is full? The Producer MUST respect the
  buffer trait's backpressure behavior (block, drop, or error --
  defined by the buffer contract, not the Producer).
- What happens when amount is outside [0.01, 10,000.00]? Transactions
  MUST have amounts within this range with exactly 2 decimal places.

## Clarifications

### Session 2026-02-21

- Q: What does "poll_interval1" mean concretely? → A: Configurable sleep duration between batch iterations (e.g., 100ms between batches).
- Q: Should the Producer's random generation be reproducible (seedable RNG)? → A: Yes, seedable RNG -- configurable seed for deterministic tests, random seed in production.
- Q: What range should generated amount values cover? → A: Currency-like floats, 0.01 to 10,000.00, 2 decimal places.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST generate transaction batches of random size
  N1 in [1, N1_MAX] per iteration.
- **FR-002**: Each transaction MUST carry at minimum: `id` (UUID v4),
  `amount` (currency-like float, 0.01 to 10,000.00, 2 decimal places),
  `last_name` (non-empty string).
- **FR-003**: Transaction schema MUST be defined in the shared domain
  crate so downstream components can depend on it.
- **FR-004**: Producer MUST write batches to Buffer1 exclusively
  through a trait-defined port (no concrete buffer dependency).
- **FR-005**: N1_MAX MUST be configurable and MUST be >= 1; invalid
  values MUST be rejected at startup.
- **FR-006**: Producer MUST support continuous operation (loop) with
  configurable iteration count or indefinite mode. Between iterations,
  the Producer MUST sleep for a configurable duration (poll_interval1).
- **FR-007**: Producer MUST handle buffer backpressure as defined by
  the Buffer1 trait contract.
- **FR-008**: Transaction field values (amount, last_name) MUST be
  generated randomly with reasonable distributions.
- **FR-009**: Producer MUST accept an optional RNG seed. When a seed is
  provided, all generated data (batch sizes, field values) MUST be
  deterministically reproducible. When no seed is provided, the
  Producer MUST use a random seed.

### Key Entities

- **Transaction**: Core data unit flowing through the pipeline. Minimum
  fields: `id` (UUID v4), `amount` (currency-like float, 0.01 to
  10,000.00, 2 decimal places), `last_name` (non-empty string). Schema
  will evolve in future features.
- **Buffer1 Port**: Trait defining the write contract between Producer
  and its output buffer. Defines write and backpressure behavior.
- **Producer**: Domain component that generates transaction batches and
  writes them to Buffer1 via its port.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Producer generates valid transaction batches with 100% of
  transactions passing schema validation (UUID format, amount in
  [0.01, 10,000.00] with 2 decimal places, non-empty last_name).
- **SC-002**: Batch sizes are uniformly distributed across [1, N1_MAX]
  over a statistically significant number of iterations (>= 100).
- **SC-003**: Swapping the Buffer1 adapter (e.g., from Vec-based to
  channel-based) requires zero changes to Producer domain code.
- **SC-004**: All Producer behavior is covered by tests written before
  implementation (TDD compliance).
- **SC-005**: A developer unfamiliar with the codebase can understand
  the Producer's architecture and trait boundaries within 10 minutes of
  reading the code.
