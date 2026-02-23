# Data Model: Concurrent Pipeline

**Branch**: `004-concurrent-pipeline` | **Date**: 2026-02-23

## Overview

No new domain types are introduced (FR-007). The only new entity is the
`ConcurrentBuffer` adapter in the binary crate.

---

## ConcurrentBuffer (adapter -- binary crate only)

**Location**: `crates/fraud_detection/src/adapters/concurrent_buffer.rs`

**Purpose**: Buffer1 + Buffer1Read adapter for the concurrent pipeline.
Differs from `InMemoryBuffer` in that an empty buffer blocks (yields) rather
than signaling Closed. Explicit `close()` call signals end-of-data.

### Inner State

| Field    | Type            | Description                                    |
|----------|-----------------|------------------------------------------------|
| `data`   | `Vec<Transaction>` | Buffered transactions (FIFO)                |
| `closed` | `bool`          | When `true`, no further writes are accepted    |

Wrapped in `RefCell<ConcurrentBufferInner>` for interior mutability.

### State Transitions

```
         write_batch(batch)                   read_batch (data available)
OPEN ─────────────────────────► OPEN    OPEN ──────────────────────────► OPEN
  │                                       │
  │ close()                               │ read_batch (empty, open) ──► yield, retry
  ▼                                       │
CLOSED                                    │ read_batch (empty, closed) ─► Err(Closed)
  │
  │ write_batch ─► Err(Closed)
```

### Trait Implementations

| Trait       | Method         | Behavior                                       |
|-------------|----------------|------------------------------------------------|
| `Buffer1`   | `write_batch`  | Append if open; `Err(Closed)` if closed        |
| `Buffer1Read` | `read_batch` | Return data; yield+retry if empty+open; `Err(Closed)` if empty+closed |

### Invariants

- The `RefCell` borrow is always dropped before `yield_now().await` to prevent
  borrow panic under re-entrant polling.
- `close()` is idempotent (setting `true` twice is harmless).
- Reads drain from the front via `Vec::drain(..count)` (same as `InMemoryBuffer`).

---

## Unchanged Types

All domain types (`Transaction`, `InferredTransaction`, `ModelVersion`, `BufferError`,
port traits) are unchanged. `InMemoryBuffer` is unchanged.

---

## main.rs Wiring Change (not a new type)

The orchestration in `main.rs` changes from sequential to:

```
ConcurrentBuffer ─── Buffer1 ──► Producer
                 ─── Buffer1Read ► Consumer ──► Modelizer ──► Buffer2 ──► Logger (deferred)
                                              └──► Alarm
```

Concurrent execution: `tokio::join!(producer_task, consumer_task)` inside
`tokio::select!` with `ctrl_c()`.
