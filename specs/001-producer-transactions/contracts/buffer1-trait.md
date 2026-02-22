# Contract: Buffer1 Trait

**Crate**: `domain`
**Type**: Hexagonal port (trait)

## Signature

```rust
#[expect(async_fn_in_trait, reason = "no dyn dispatch needed; internal workspace only")]
pub trait Buffer1 {
    async fn write_batch(&self, batch: Vec<Transaction>) -> Result<(), BufferError>;
}
```

## Semantics

### `write_batch`

**Preconditions**:
- `batch` may be empty (no-op, returns `Ok(())`)
- `batch` elements are valid `Transaction` instances

**Postconditions on `Ok(())`**:
- All transactions in `batch` are stored in the buffer
- Transactions are retrievable in insertion order

**Error conditions**:
- `BufferError::Full { capacity }` -- buffer cannot accept the batch (at capacity)
- `BufferError::Closed` -- buffer has been shut down, no further writes accepted

**Backpressure policy**:
- The trait does NOT define retry behavior
- Callers (Producer) decide how to handle `Full` and `Closed` errors
- `Full` is recoverable (caller may retry after consumer drains)
- `Closed` is terminal (caller should stop producing)

## Adapter Requirements

Any implementation of `Buffer1` MUST:
1. Accept `&self` (use interior mutability if needed)
2. Be async-compatible (return a future)
3. Return `BufferError::Full` when capacity is exceeded (if bounded)
4. Return `BufferError::Closed` when the buffer is shut down

## Known Adapters

| Adapter | Crate | Backing Store | Bounded |
|---------|-------|---------------|---------|
| `InMemoryBuffer` | `fraud_detection` | `RefCell<Vec<Transaction>>` | No (unbounded for PoC) |
