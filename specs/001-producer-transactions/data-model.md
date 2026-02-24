# Data Model: Producer -- Transaction Generation

**Feature Branch**: `001-producer-transactions`
**Created**: 2026-02-22

## Entities

### Transaction

Core data unit flowing through the pipeline. Defined in `domain` crate.

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| `id` | `uuid::Uuid` | v4 format, unique per transaction | Generated via `Builder::from_random_bytes` |
| `amount` | `f64` | 0.01..=10_000.00, 2 decimal places | Generated as integer cents / 100.0 |
| `last_name` | `String` | Non-empty | Randomly selected from hardcoded list |

**Derives**: `Debug`, `Clone`, `PartialEq`

**Validation**: No runtime validation struct -- constraints enforced at generation time
by `Producer`. The `Transaction` struct is a plain data carrier (no invariants to
protect via private fields).

**Evolution**: Future features will add fields (`predicted_fraud`, `model_name`,
`model_version`, `prediction_confirmed`). Fields will be added as `Option<T>` or via
a separate `InferredTransaction` wrapper type -- TBD in future feature specs.

### BufferError

Error type for buffer operations. Defined in `domain` crate.

| Variant | Fields | When |
|---------|--------|------|
| `Full` | `capacity: usize` | Buffer at capacity, cannot accept batch |
| `Closed` | none | Buffer has been shut down |

**Derives**: `Debug`, `Clone`, `PartialEq`
**Implements**: `thiserror::Error`, `std::fmt::Display` (via `#[error(...)]`)

### ProducerConfig

Configuration for the Producer component. Defined in `producer` crate.

| Field | Type | Constraints | Default |
|-------|------|-------------|---------|
| `n1_max` | `usize` | >= 1, required | none (mandatory) |
| `poll_interval1` | `Duration` | >= 0 | 100ms |
| `iterations` | `Option<usize>` | `None` = infinite, `Some(n)` = n iterations | `None` |
| `seed` | `Option<u64>` | `None` = random, `Some(s)` = deterministic | `None` |

**Construction**: Builder pattern via `ProducerConfig::builder(n1_max)`.
- `n1_max` is a mandatory builder parameter (passed to `builder()`)
- All other fields have defaults
- `build()` validates `n1_max >= 1` and returns `Result<ProducerConfig, ProducerError>`

### ProducerError

Error type for Producer operations. Defined in `producer` crate.

| Variant | Fields | When |
|---------|--------|------|
| `InvalidConfig` | `reason: String` | `n1_max == 0` or other invalid config |
| `Buffer` | `source: BufferError` | Buffer write failed (wraps `BufferError` via `#[from]`) |

**Derives**: `Debug`
**Implements**: `thiserror::Error`, `std::fmt::Display` (via `#[error(...)]`)

## Traits (Hexagonal Ports)

### Buffer1

Write port for Producer output. Defined in `domain` crate.

```text
Buffer1
  write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError>
```

- `&self` receiver (not `&mut self`) -- implementations use interior mutability
- Async (AFIT) -- `#[expect(async_fn_in_trait)]` on trait definition
- Single method for this feature; future features may add `read_batch` or `close`

## Relationships

```text
Producer --uses--> Buffer1 (trait, via generic parameter)
Producer --creates--> Transaction (generates batches)
Producer --configured-by--> ProducerConfig (builder pattern)
Producer --emits--> ProducerError (on failure)
Buffer1 --returns--> BufferError (on write failure)
ProducerError --wraps--> BufferError (via #[from])
InMemoryBuffer --implements--> Buffer1 (adapter in fraud_detection crate)
```

## Crate Ownership

| Entity | Crate | Rationale |
|--------|-------|-----------|
| `Transaction` | `domain` | Shared across all pipeline components (FR-003) |
| `BufferError` | `domain` | Part of the Buffer1 port contract |
| `Buffer1` | `domain` | Hexagonal port, shared definition |
| `ProducerConfig` | `producer` | Producer-specific configuration |
| `ProducerConfigBuilder` | `producer` | Builder for ProducerConfig |
| `ProducerError` | `producer` | Producer-specific errors |
| `Producer` | `producer` | Domain logic |
| `InMemoryBuffer` | `fraud_detection` | Adapter (binary crate owns adapters) |
