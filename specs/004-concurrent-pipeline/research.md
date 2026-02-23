# Research: Concurrent Pipeline

**Branch**: `004-concurrent-pipeline` | **Date**: 2026-02-23

## Decision 1 -- tokio signal feature

**Decision**: Add `"signal"` to the workspace tokio feature list in `Cargo.toml`.

**Rationale**: `tokio::signal::ctrl_c()` requires the `signal` cargo feature. Current
workspace config has only `"rt", "macros", "time"`. The `signal` feature is not
automatically included.

**Alternatives considered**: Use OS-level signal handling via `signal-hook` crate --
rejected: `tokio::signal` is already in scope and idiomatic for async Tokio code.

---

## Decision 2 -- buffer adapter for concurrent mode

**Decision**: Create a new `ConcurrentBuffer` adapter in
`crates/fraud_detection/src/adapters/concurrent_buffer.rs`. Keep `InMemoryBuffer`
unchanged.

**Rationale**: `InMemoryBuffer.read_batch` returns `BufferError::Closed` when the
buffer is empty (sequential mode semantics). In concurrent mode, empty means "wait,
more data may arrive." Changing `InMemoryBuffer` would break the existing tests
`in_memory_buffer_stores_batch` and `read_batch_drains_from_front`, violating FR-008.
A new adapter avoids this while keeping all 55 existing tests green.

**`ConcurrentBuffer` behavior**:
- `close()`: sets `closed = true` in inner state.
- `write_batch`: returns `Err(BufferError::Closed)` if closed, otherwise appends.
- `read_batch`: if data available, returns batch; if empty + not closed, calls
  `tokio::task::yield_now().await` and retries (cooperative busy-wait); if empty +
  closed, returns `Err(BufferError::Closed)`.

**Safety on `current_thread` runtime**: `tokio::join!` polls all futures on a single
OS thread; `RefCell` interior mutability is safe; no `Sync` required (Principle VI).
The borrow guard is always dropped before the `yield_now().await` point.

**Alternatives considered**:
- Modify `InMemoryBuffer`: rejected (breaks FR-008, 2 existing tests).
- Use `tokio::sync::Notify` or channel: rejected (adds `sync` feature, adds
  complexity -- yield_now is sufficient for single-thread runtime).

---

## Decision 3 -- finite-mode shutdown

**Decision**: Wrap `producer.run()` in an inline async block in `main.rs` that calls
`buffer1.close()` after the run completes.

**Rationale**: FR-005 states shutdown propagates "naturally via buffer closure" when
Producer finishes its finite iterations. Producer does not call `close()` itself
(no API change allowed). By wrapping in main, we close the buffer as soon as
Producer returns. Consumer then drains remaining data and stops on the first
`read_batch` call that finds an empty+closed buffer.

**Alternatives considered**:
- Add `close()` to `Buffer1` trait: rejected (requires updating all trait impls
  including test mocks, violating FR-008).
- Change Producer to close buffer on stop: rejected (FR-006 -- no API changes).

---

## Decision 4 -- CTRL+C shutdown via `tokio::select!`

**Decision**: Use the pattern specified in FR-004:
```rust
tokio::select! {
    _ = tokio::signal::ctrl_c() => { buffer1.close(); }
    (p, c) = tokio::join!(producer_task, consumer_task) => { p?; c?; }
}
```
When ctrl_c fires, the join! future is dropped (tasks cancelled at current await
point). `buffer1.close()` is called for consistency but the tasks are already
cancelled. `main` returns `Ok(())` immediately.

**Rationale**: Simplest implementation matching FR-004's select!/join! prescription.
SC-002 (clean exit, code 0, no panic, no hang) is satisfied. In-flight data at
cancellation is an acceptable loss per spec Assumptions ("batches not yet written to
Buffer1 are acceptable losses").

**Alternatives considered**:
- Loop + `&mut pipeline_fut` to wait for tasks after close: rejected (adds
  complexity; spec does not mandate draining on CTRL+C, only for finite mode).

---

## Decision 5 -- FR-009 iteration-limit log messages

**Decision**: Add `log::info!` calls in `Producer::run()` and `Consumer::run()` for
the "iteration limit reached" stop case. This is an internal implementation change;
public APIs remain unchanged (FR-006 compliant).

**Rationale**: Producer and Consumer already log the "buffer closed" stop reason.
FR-009 requires logging for BOTH reasons. The iteration-limit branch currently
returns `Ok(())` silently.

---

## Decision 6 -- `tokio::task::yield_now` availability

**Decision**: Use `tokio::task::yield_now()` in `ConcurrentBuffer::read_batch`.
No additional tokio feature is needed beyond `rt` (already present).

**Rationale**: `tokio::task::yield_now` is gated behind the `rt` feature, which is
already in the workspace tokio config. No new dependency needed.
