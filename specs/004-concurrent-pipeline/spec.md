# Feature Specification: Concurrent Pipeline

**Feature Branch**: `004-concurrent-pipeline`
**Created**: 2026-02-23
**Status**: Draft
**Input**: User description: "Feature 004 Concurrent Pipeline: refactor main.rs to run
Producer and Consumer concurrently via tokio::join! (Principle VI). Currently
the pipeline is sequential (producer.run() blocks until done, then consumer.run()
starts). Target: both run indefinitely (iterations=None) and interleave via
cooperative async scheduling on current_thread runtime. Two shutdown mechanisms:
(1) CTRL+C via tokio::signal::ctrl_c() closes Buffer1 which propagates
BufferError::Closed to stop each component cleanly; (2) finite iterations
(iterations=Some(n)) for demo/test mode. No new domain types. No changes to
Producer or Consumer crate APIs. All 55 existing tests must keep passing.
Logger wiring is out of scope (deferred to feature 005)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Concurrent Operation (Priority: P1)

As an operator starting the fraud detection pipeline, I want Producer and
Consumer to run at the same time so that transactions flow through the
pipeline continuously -- Producer filling the buffer while Consumer drains
and classifies it -- without either stage waiting for the other to finish.

**Why this priority**: This is the core architectural change. Without it,
the pipeline does not match its intended design. All other stories depend
on concurrent execution being in place.

**Independent Test**: Run the application and observe that log output from
Producer (batch written) and Consumer (batch processed) interleave in time
rather than appearing in two sequential blocks.

**Acceptance Scenarios**:

1. **Given** the app is started with no iteration limit, **When** it runs,
   **Then** Producer and Consumer are both active simultaneously and log
   output from both interleaves in real time.
2. **Given** Producer is writing a batch, **When** Consumer has data
   available, **Then** Consumer processes that data without waiting for
   Producer to finish all its batches.
3. **Given** the pipeline is running, **When** Producer exhausts its work
   (finite iterations configured), **Then** Consumer detects the empty
   buffer and stops cleanly, then the app exits with no error.

---

### User Story 2 - Graceful Shutdown via CTRL+C (Priority: P2)

As an operator running the pipeline continuously (no iteration limit),
I want to stop the application cleanly by pressing CTRL+C so that no
transaction is left in a corrupted or partially-processed state.

**Why this priority**: Without a shutdown mechanism, a continuously-running
pipeline can only be killed by the OS, which may leave the system in an
unknown state. Graceful shutdown is required for safe operation.

**Independent Test**: Start the pipeline with no iteration limit, wait for
several cycles of Producer and Consumer to interleave, then press CTRL+C.
Observe that the app logs a shutdown message and exits with code 0, with no
panic or error output.

**Acceptance Scenarios**:

1. **Given** the pipeline is running with no iteration limit, **When** the
   operator presses CTRL+C, **Then** both Producer and Consumer stop after
   completing their current operation and the app exits cleanly (exit code 0).
2. **Given** CTRL+C is received mid-cycle, **When** a component is in the
   middle of a batch operation, **Then** the component finishes its current
   batch before stopping (no partial batches abandoned mid-flight).
3. **Given** the pipeline receives CTRL+C, **When** components stop,
   **Then** the app exits within 5 seconds (no hang or deadlock).

---

### User Story 3 - Demo / Test Mode with Finite Iterations (Priority: P3)

As a developer demonstrating or testing the pipeline, I want to configure
a fixed number of Producer iterations so that the pipeline runs for a
bounded duration and exits automatically, without requiring CTRL+C.

**Why this priority**: Finite iteration mode is essential for automated
tests and reproducible demos. It must coexist cleanly with the concurrent
execution introduced in US1.

**Independent Test**: Configure Producer with 10 iterations. Run the app.
Verify it completes all 10 iterations with both Producer and Consumer
interleaving their log output, then exits automatically with no error and
no manual intervention required.

**Acceptance Scenarios**:

1. **Given** Producer is configured with a fixed iteration count of 10,
   **When** the app runs, **Then** Producer emits exactly 10 batches,
   Consumer processes all of them, and the app exits cleanly on its own.
2. **Given** the pipeline runs in demo mode, **When** Producer finishes its
   last batch, **Then** Consumer drains the remaining buffer content before
   stopping (no transactions left unprocessed in the buffer).
3. **Given** all 55 existing automated tests, **When** the refactoring is
   applied, **Then** all 55 tests continue to pass without modification.

---

### Edge Cases

- **CTRL+C before Consumer reads any data**: buffer.close() is called;
  Producer's next write_batch returns Err(BufferError::Closed) and stops;
  Consumer's next read returns Err(BufferError::Closed) and stops. No data
  loss risk -- nothing was committed to the buffer yet.
- **Buffer full + shutdown signal**: write_batch returns
  Err(BufferError::Closed) immediately when the buffer is closed,
  regardless of capacity. Producer stops on next poll.
- **Consumer blocked on empty buffer + CTRL+C**: buffer.close() unblocks
  the read call with Err(BufferError::Closed); Consumer stops cleanly.
- **Iteration limit and CTRL+C fire simultaneously**: tokio::select! in
  main resolves to whichever branch completes first. If join! completes
  first (iteration limit), the select! exits normally. If ctrl_c fires
  first, buffer.close() is called; the join! branch resolves shortly after
  as both tasks detect Closed. Either ordering produces a clean exit.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The pipeline MUST start Producer and Consumer as concurrent
  tasks that run simultaneously from application startup.
- **FR-002**: By default (no iteration limit configured), Producer and
  Consumer MUST run indefinitely until an external stop signal is received.
- **FR-003**: The application MUST handle a CTRL+C (interrupt) signal and
  stop all pipeline components cleanly as a result.
- **FR-004**: Shutdown via CTRL+C MUST propagate by closing Buffer1: `main`
  uses `tokio::select!` to race `tokio::signal::ctrl_c()` against
  `tokio::join!(producer.run(), consumer.run())`; on signal receipt, `main`
  calls `buffer.close()` directly, after which both tasks detect
  `BufferError::Closed` and return normally without error.
- **FR-005**: The pipeline MUST support a finite iteration mode: when
  Producer is configured with a fixed iteration count, it stops after that
  count and shutdown propagates naturally to Consumer via buffer closure.
- **FR-006**: Producer and Consumer crate APIs (public types, method
  signatures, config builders) MUST remain unchanged by this feature.
- **FR-007**: No new domain types (traits, structs, enums) MUST be
  introduced by this feature.
- **FR-008**: All 55 existing automated tests MUST pass after the
  refactoring without any modification to test code.
- **FR-009**: The pipeline MUST log an `info`-level message when each
  component stops, indicating the reason (buffer closed or iteration
  limit reached).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: When the app runs with no iteration limit, log output from
  Producer and Consumer interleaves -- neither component runs entirely
  before the other begins. Verified by manual observation of the binary
  log output; no automated test for interleaving order is required.
- **SC-002**: When CTRL+C is pressed on a running pipeline, the application
  exits cleanly (exit code 0) with no panic, no error output, and no hang.
- **SC-003**: All 55 existing automated tests pass after the refactoring
  is applied (zero regressions).
- **SC-004**: When configured with a finite Producer iteration count (e.g.,
  10), the pipeline runs to completion and exits automatically with no
  manual intervention.
- **SC-005**: No Producer batch that was successfully written to Buffer1
  before shutdown is silently dropped: Consumer processes every batch that
  entered the buffer before the close signal.

## Assumptions

- The Tokio single-thread runtime (`current_thread` flavor) is retained;
  no switch to a multi-thread runtime is required or in scope.
- `InMemoryBuffer` (Buffer1 adapter) uses interior mutability and remains
  valid under single-thread concurrent access via cooperative async tasks.
- "Clean stop" means exit code 0 with no panic; batches that were not yet
  written to Buffer1 at the moment of shutdown are acceptable losses.
- Logger component wiring is explicitly out of scope and deferred to
  feature 005.

## Clarifications

### Session 2026-02-23

- Q: Shutdown orchestration pattern -- who calls buffer.close() and when? → A: `tokio::select!` in main races `ctrl_c()` against `join!(producer.run(), consumer.run())`; on CTRL+C, main calls `buffer.close()` directly.
- Q: Maximum acceptable shutdown duration ("reasonable time")? → A: 5 seconds; no forced timeout wrapper in this feature -- hanging after 5 s indicates a bug.
- Q: Automated test required for SC-001 interleaving? → A: Manual observation only; no automated interleaving test needed.
- Q: write_batch behavior when buffer is closed mid-write? → A: Returns Err(BufferError::Closed) immediately; no flush/drain.
- Q: Log level for FR-009 component-stop messages? → A: `info`.
