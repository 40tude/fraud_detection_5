# Data Model: Logger Batch Persistence

**Feature**: 005-logger-transactions | **Date**: 2026-02-23

## Domain Types (crate: `domain`)

### PendingTransaction (NEW)

A transaction awaiting full verification. Wraps an `InferredTransaction` with a confirmation flag.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `inferred_transaction` | `InferredTransaction` | Composition: original inferred transaction | (from input) |
| `prediction_confirmed` | `bool` | Whether fraud prediction has been fully verified | `false` |

**Derives**: `Debug`, `Clone`, `PartialEq`
**Convenience method**: `id() -> uuid::Uuid` (delegates through `inferred_transaction.id()`)

**Composition chain**:
```
PendingTransaction
  +-- inferred_transaction: InferredTransaction
  |     +-- transaction: Transaction
  |     |     +-- id: Uuid
  |     |     +-- amount: f64
  |     |     +-- last_name: String
  |     +-- predicted_fraud: bool
  |     +-- model_name: String
  |     +-- model_version: ModelVersion
  +-- prediction_confirmed: bool
```

### StorageError (NEW)

| Variant | Fields | Description |
|---------|--------|-------------|
| `CapacityExceeded` | `capacity: usize` | Storage is at maximum capacity |
| `Unavailable` | (unit) | Storage backend is unreachable or closed |

**Derives**: `Debug`, `Clone`, `PartialEq`
**Trait impl**: `thiserror::Error`

## Port Traits (crate: `domain`)

### Buffer2Read (NEW)

Read-side hexagonal port for Buffer2. Symmetric to `Buffer1Read`.

| Method | Signature | Description |
|--------|-----------|-------------|
| `read_batch` | `async fn read_batch(&self, max: usize) -> Result<Vec<InferredTransaction>, BufferError>` | Read up to `max` inferred transactions. Returns `Err(Closed)` when closed and drained. |

**AFIT suppression**: `#[expect(async_fn_in_trait, reason = "no dyn dispatch needed; internal workspace only")]`

### Storage (NEW)

Write-side hexagonal port for persistent storage.

| Method | Signature | Description |
|--------|-----------|-------------|
| `write_batch` | `async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError>` | Persist a batch of pending transactions. |

**AFIT suppression**: same as `Buffer2Read`.

## Logger Crate Types (crate: `logger`)

### LoggerError

| Variant | Wraps | Source |
|---------|-------|--------|
| `InvalidConfig` | `{ reason: String }` | Config validation failure |
| `Read` | `BufferError` | Buffer2Read failure (`#[from]`) |
| `Write` | `StorageError` | Storage failure (`#[from]`) |

### LoggerConfig

| Field | Type | Default | Constraint |
|-------|------|---------|------------|
| `n3_max` | `usize` | (required) | >= 1 |
| `speed3` | `Duration` | 100 ms | any |
| `iterations` | `Option<u64>` | `None` | `None` = infinite |
| `seed` | `Option<u64>` | `None` | `None` = OS-seeded |

**Builder**: `LoggerConfig::builder(n3_max) -> LoggerConfigBuilder`

### Logger

| Field | Type | Description |
|-------|------|-------------|
| `config` | `LoggerConfig` | Runtime configuration |
| `rng` | `RefCell<StdRng>` | Interior-mutable RNG for batch sizing |

**Methods**:
- `new(config) -> Self` -- seed from config or OS
- `log_once(&self, buf2: &B2R, storage: &S) -> Result<(), LoggerError>` -- read, transform, persist one batch
- `run(&self, buf2: &B2R, storage: &S) -> Result<(), LoggerError>` -- continuous loop until closed or iteration limit

## Adapter Types (crate: `fraud_detection`)

### ConcurrentBuffer2 (NEW)

Implements `Buffer2` (write) + `Buffer2Read` (read). Same yield-on-empty pattern as `ConcurrentBuffer`.

| Field | Type | Description |
|-------|------|-------------|
| `inner` | `RefCell<ConcurrentBuffer2Inner>` | Interior-mutable state |

**Inner**:
- `data: Vec<InferredTransaction>`
- `closed: bool`

**Methods**: `new() -> Self`, `close(&self)`

### InMemoryStorage (NEW)

Implements `Storage`. Backed by `RefCell<Vec<PendingTransaction>>` with optional capacity.

| Field | Type | Description |
|-------|------|-------------|
| `inner` | `RefCell<Vec<PendingTransaction>>` | Stored pending transactions |
| `capacity` | `usize` | Maximum item count |

**Methods**: `new(capacity) -> Self`

## State Transitions

```
InferredTransaction --(Logger.log_once)--> PendingTransaction { prediction_confirmed: false }
```

No other state transitions in scope. `prediction_confirmed` update is deferred to a future feature.
