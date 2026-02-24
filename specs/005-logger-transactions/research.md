# Research: Logger Batch Persistence

**Feature**: 005-logger-transactions | **Date**: 2026-02-23

## R1: Buffer2Read Trait Design

**Decision**: New `Buffer2Read` AFIT trait in `domain`, symmetric to `Buffer1Read` but returning `Vec<InferredTransaction>`.

**Rationale**: Constitution Principle I mandates trait-defined ports. `Buffer1Read` is the established pattern for read-side ports. `Buffer2Read` mirrors it for the second buffer.

**Alternatives considered**:
- Reuse `Buffer1Read` with generics: rejected -- violates pedagogical clarity (Principle IV). Each buffer stage has a distinct item type (`Transaction` vs `InferredTransaction`); separate traits make the pipeline topology explicit.
- Single `BufferRead<T>` generic trait: rejected -- unnecessary abstraction for a didactic project; would require trait object gymnastics or additional generic bounds.

**Signature**:
```rust
pub trait Buffer2Read {
    async fn read_batch(&self, max: usize) -> Result<Vec<InferredTransaction>, BufferError>;
}
```

## R2: Storage Trait and StorageError

**Decision**: New `Storage` AFIT trait + `StorageError` enum in `domain`. Variants: `CapacityExceeded { capacity: usize }`, `Unavailable`.

**Rationale**: Spec clarification session explicitly chose a new error type to reinforce that Storage is conceptually different from a Buffer. `BufferError::Full` maps to data in transit; `StorageError::CapacityExceeded` maps to persistence at rest.

**Alternatives considered**:
- Reuse `BufferError` for Storage: rejected per spec clarification -- Storage is not a buffer.
- Single variant `StorageError::Failed(String)`: rejected -- structured variants enable match-based handling.

**Signature**:
```rust
pub trait Storage {
    async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError>;
}
```

## R3: PendingTransaction Composition

**Decision**: Composition struct wrapping `InferredTransaction` + `is_reviewed: bool` + `actual_fraud: Option<bool>`.

**Rationale**: Follows the nesting chain `Transaction -> InferredTransaction -> PendingTransaction` established in the spec. Consistent with `InferredTransaction { transaction: Transaction, ... }`. `is_reviewed` tracks whether a human has examined the record; `actual_fraud: Option<bool>` encodes the ground-truth label (None = not yet reviewed, Some(true/false) = confirmed outcome). `Option<bool>` maps naturally to a nullable BOOLEAN column in SQL databases.

**Alternatives considered**:
- Flat struct copying all fields: rejected -- violates DRY, harder to maintain, loses the composition pattern established for `InferredTransaction`.
- Generic enrichment wrapper `Enriched<T, Extra>`: over-engineering for a didactic project.
- Two booleans (`is_reviewed`, `is_fraud`): rejected -- `is_fraud` is semantically undefined when `is_reviewed = false`.

**Type**:
```rust
pub struct PendingTransaction {
    pub inferred_transaction: InferredTransaction,
    pub is_reviewed: bool,
    pub actual_fraud: Option<bool>,
}
```

## R4: ConcurrentBuffer2 Adapter

**Decision**: New `ConcurrentBuffer2` in `fraud_detection/src/adapters/`, implementing both `Buffer2` (write) and `Buffer2Read` (read). Same yield-on-empty pattern as `ConcurrentBuffer`.

**Rationale**: The existing `InMemoryBuffer2` only implements `Buffer2` (write). For the concurrent pipeline, Logger needs a read-side that yields cooperatively when empty (just like Consumer reads from `ConcurrentBuffer`). A dedicated `ConcurrentBuffer2` mirrors the `ConcurrentBuffer` pattern for `InferredTransaction`.

**Alternatives considered**:
- Add `Buffer2Read` to `InMemoryBuffer2`: rejected -- `InMemoryBuffer2` has capacity semantics and no yield-on-empty logic. Mixing concerns.
- Generic `ConcurrentBuffer<T>`: over-engineering; different item types (`Transaction` vs `InferredTransaction`) make a generic less clear pedagogically.

## R5: InMemoryStorage Adapter

**Decision**: `InMemoryStorage` adapter in `fraud_detection/src/adapters/`, implementing `Storage` with a `RefCell<Vec<PendingTransaction>>` and optional capacity limit.

**Rationale**: Simplest possible persistence adapter for proof-of-concept. Mirrors the simplicity of `InMemoryBuffer2` but for the `Storage` trait.

**Alternatives considered**:
- File-based storage: out of scope per constitution ("all adapters are in-process" for v1).
- Unbounded Vec: acceptable but capacity limit enables testing `StorageError::CapacityExceeded`.

## R6: LoggerConfig Builder Pattern

**Decision**: `LoggerConfig` + `LoggerConfigBuilder` following the exact `ProducerConfig`/`ConsumerConfig` builder pattern. Fields: `n3_max: usize`, `speed3: Duration`, `iterations: Option<u64>`, `seed: Option<u64>`.

**Rationale**: Consistency with established crate patterns. Rejects `n3_max == 0` in `build()`.

**Alternatives considered**:
- Direct struct construction: rejected -- violates M-INIT-BUILDER ms-rust guideline.

## R7: LoggerError Enum

**Decision**: `LoggerError` with variants: `InvalidConfig { reason }`, `Read(BufferError)`, `Write(StorageError)`.

**Rationale**: Symmetric to `ConsumerError`. No `#[from]` on `Read` and `Write` because they wrap different types (`BufferError` vs `StorageError`), but since they ARE different types, `#[from]` is actually fine here (unlike Consumer which wraps `BufferError` for both Read and Write). However, for explicit control and clarity, manual `.map_err()` is preferred.

**Decision refined**: Use `#[from]` for `Write(StorageError)` since it is the only variant wrapping `StorageError`. Use manual `.map_err(LoggerError::Read)` for `BufferError` since other variants do not wrap it -- actually, `Read` is the only one wrapping `BufferError`, so `#[from]` works for both. Final decision: use `#[from]` for both `Read(BufferError)` and `Write(StorageError)` since each source type maps to exactly one variant.

## R8: Pipeline Integration

**Decision**: Logger joins the existing `tokio::join!` in `main.rs`. `ConcurrentBuffer2` replaces `InMemoryBuffer2` as the Buffer2 adapter. Consumer writes to it; Logger reads from it. On CTRL+C, close both `buffer1` and `buffer2`. Logger stops when `buffer2` is closed+drained.

**Rationale**: Follows the pattern established in feature 004. Logger is the final pipeline stage -- it reads from buffer2 just as Consumer reads from buffer1.

**Pipeline flow**:
```
Producer -> buffer1 (ConcurrentBuffer) -> Consumer+Modelizer -> buffer2 (ConcurrentBuffer2) -> Logger -> storage (InMemoryStorage)
```

**Shutdown sequence**:
1. CTRL+C or Producer finishes -> close buffer1
2. Consumer drains buffer1, finishes -> close buffer2
3. Logger drains buffer2, finishes -> all join! arms resolved
