# Data Model: Consumer Batch Processing

**Feature**: 002-consumer-transactions | **Date**: 2026-02-22

## Domain Entities (domain crate)

### InferredTransaction

Transaction enriched with Modelizer inference results.

| Field | Type | Description |
|-------|------|-------------|
| `transaction` | `Transaction` | Original transaction (composition) |
| `predicted_fraud` | `bool` | `true` if Modelizer flagged as fraudulent |
| `model_name` | `String` | Name of model used (e.g. "DINN") |
| `model_version` | `String` | Version string (e.g. "v1", "v2") |

Derives: `Debug`, `Clone`, `PartialEq`

Convenience accessor `id` delegates to `self.transaction.id`.

### ModelVersion

Selectable model version for Modelizer switch commands.

| Variant | Description |
|---------|-------------|
| `N` | Latest model version (default) |
| `NMinus1` | Previous model version |

Derives: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

### ModelizerError

Errors from the Modelizer hexagonal port.

| Variant | Fields | Description |
|---------|--------|-------------|
| `InferenceFailed` | `reason: String` | Inference could not be completed |
| `SwitchFailed` | `reason: String` | Version switch could not be applied |

Derives: `Debug`, `thiserror::Error`

### AlarmError

Errors from the Alarm hexagonal port.

| Variant | Fields | Description |
|---------|--------|-------------|
| `DeliveryFailed` | `reason: String` | Alarm could not be delivered |

Derives: `Debug`, `thiserror::Error`

### BufferError (existing -- no changes)

Already defines `Full { capacity }` and `Closed`. Reused for Buffer1Read and Buffer2.

## Port Traits (domain crate)

### Buffer1Read

Read side of the first inter-component buffer.

```text
trait Buffer1Read {
    async fn read_batch(&self, max: usize) -> Result<Vec<Transaction>, BufferError>
}
```

- Awaits until data available (adapter responsibility)
- Returns 1..=max transactions when data present
- Returns `Err(BufferError::Closed)` when closed and drained
- Never returns `BufferError::Full`

### Buffer2

Write side of the second inter-component buffer.

```text
trait Buffer2 {
    async fn write_batch(&self, batch: Vec<InferredTransaction>) -> Result<(), BufferError>
}
```

- Same error semantics as `Buffer1::write_batch`
- Accepts `InferredTransaction` (not `Transaction`)

### Modelizer

Inference and version-switching port.

```text
trait Modelizer {
    async fn infer(&self, batch: Vec<Transaction>) -> Result<Vec<InferredTransaction>, ModelizerError>
    async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError>
}
```

- `infer` returns one `InferredTransaction` per input `Transaction` (same order, same count)
- `switch_version` takes effect on next `infer` call (FR-010)

### Alarm

Per-transaction fraud alert port.

```text
trait Alarm {
    async fn trigger(&self, transaction: &InferredTransaction) -> Result<(), AlarmError>
}
```

- Called once per fraudulent transaction (best-effort)
- Failures collected, not fatal

## Consumer Entities (consumer crate)

### ConsumerConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `n2_max` | `usize` | (required) | Max batch size, range `[1, N2_MAX]` |
| `speed2` | `Duration` | `100ms` | Delay between processing iterations |
| `iterations` | `Option<u64>` | `None` | Optional iteration limit |
| `seed` | `Option<u64>` | `None` | Optional RNG seed for deterministic batch sizes |

Builder pattern via `ConsumerConfig::builder(n2_max)` (mirrors `ProducerConfig`).

### ConsumerError

| Variant | Source | Description |
|---------|--------|-------------|
| `InvalidConfig` | `reason: String` | Config validation failure |
| `Read` | `BufferError` | Buffer1 read failure |
| `Inference` | `ModelizerError` | Modelizer inference failure |
| `Write` | `BufferError` | Buffer2 write failure |

Derives: `Debug`, `thiserror::Error`

No `#[from]` on `Read`/`Write` (same source type); manual `.map_err()`.

### Consumer

| Field | Type | Description |
|-------|------|-------------|
| `config` | `ConsumerConfig` | Runtime configuration |
| `rng` | `RefCell<StdRng>` | Interior-mutable RNG for batch size variation |

Methods:
- `new(config) -> Self`
- `consume_once<B1, M, A, B2>(&self, ...) -> Result<Vec<AlarmError>, ConsumerError>`
- `run<B1, M, A, B2>(&self, ...) -> Result<(), ConsumerError>`
- `switch_model_version<M>(&self, modelizer: &M, version: ModelVersion) -> Result<(), ConsumerError>`

## State Transitions

```text
Transaction (Buffer1) --> [Consumer reads] --> [Modelizer infers] --> InferredTransaction
  --> [Alarm per fraudulent] --> [Buffer2 write] --> InferredTransaction (Buffer2)
```

Model version: `N (default)` <--switch--> `NMinus1` (via Modelizer port, takes effect next batch)
