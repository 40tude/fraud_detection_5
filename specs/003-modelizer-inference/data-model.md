# Data Model: Modelizer Inference

**Feature**: 003-modelizer-inference | **Date**: 2026-02-23

## Existing Types (unchanged)

### Transaction (domain)
| Field | Type | Notes |
|-------|------|-------|
| `id` | `uuid::Uuid` | Random UUID v4 |
| `amount` | `f64` | `[0.01, 10_000.00]` |
| `last_name` | `String` | Account holder |

### InferredTransaction (domain)
| Field | Type | Notes |
|-------|------|-------|
| `transaction` | `Transaction` | Original (composition) |
| `predicted_fraud` | `bool` | Model classification result |
| `model_name` | `String` | e.g. `"DEMO"` |
| `model_version` | `String` | e.g. `"4"` |

### ModelVersion (domain)
| Variant | Meaning |
|---------|---------|
| `N` | Latest version (default) |
| `NMinus1` | Previous version |

### ModelizerError (domain)
| Variant | Fields | Used by |
|---------|--------|---------|
| `InferenceFailed` | `reason: String` | `Modelizer::infer`, `Model::classify` |
| `SwitchFailed` | `reason: String` | `Modelizer::switch_version`, `Model::switch_version` |

## New Types

### Model trait (domain) -- NEW

Hexagonal port for per-transaction classification. Defined in `crates/domain/src/lib.rs`.

```text
trait Model {
    async fn classify(&self, tx: &Transaction) -> Result<bool, ModelizerError>
    fn name(&self) -> &str
    fn active_version(&self) -> &str
    async fn switch_version(&self, version: ModelVersion) -> Result<(), ModelizerError>
}
```

| Method | Async | Fallible | Notes |
|--------|-------|----------|-------|
| `classify` | yes | yes (`InferenceFailed`) | Per-transaction fraud decision |
| `name` | no | no | Self-describing, returns static or cached name |
| `active_version` | no | no | Current concrete version string |
| `switch_version` | yes | yes (`SwitchFailed`) | Changes active version; takes effect next classify |

**AFIT**: `#[expect(async_fn_in_trait)]` on trait def.

### Modelizer struct (modelizer crate) -- NEW

Generic struct implementing `domain::Modelizer` by delegating to a `domain::Model`.

```text
struct Modelizer<M: Model> {
    model: M,
}
```

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new` | `fn new(model: M) -> Self` | Constructor; `#[must_use]` |
| `infer` | `impl domain::Modelizer` | Read name+version once, iterate batch calling `model.classify`, build `InferredTransaction` per tx |
| `switch_version` | `impl domain::Modelizer` | Delegate to `model.switch_version` |

No config. No RNG. No internal state beyond the Model reference.

### DemoModel struct (fraud_detection crate) -- NEW

Concrete adapter implementing `domain::Model`. Replaces `DemoModelizer`.

```text
struct DemoModel {
    current_version: RefCell<ModelVersion>,
    rng: RefCell<StdRng>,
}
```

| Field | Type | Notes |
|-------|------|-------|
| `current_version` | `RefCell<ModelVersion>` | Interior mutability (trait takes `&self`) |
| `rng` | `RefCell<StdRng>` | Seeded or OS-random; drives fraud probability |

| Method | Behavior |
|--------|----------|
| `new(seed: Option<u64>)` | Create with optional seed; defaults to `ModelVersion::N` |
| `classify` | Roll RNG against fraud rate: 4% for N (version 4), 3% for NMinus1 (version 3) |
| `name` | Returns `"DEMO"` |
| `active_version` | Returns `"4"` (N) or `"3"` (NMinus1) |
| `switch_version` | Updates `current_version` RefCell |

## Relationships

```text
Consumer --calls--> domain::Modelizer trait
                        ^
                        |  implements
                        |
              modelizer::Modelizer<M>
                        |
                        |  delegates to
                        v
                  domain::Model trait
                        ^
                        |  implements
                        |
        fraud_detection::DemoModel adapter
```

## Validation Rules

- `classify` must return `Ok(true)` or `Ok(false)` -- never errors for DEMO (simple RNG roll).
- `switch_version` with already-active version is a no-op (no error).
- Empty batch to `infer` returns empty vec (no error).
- Version switch takes effect on next `infer`, not current (FR-009).
- Over 10k transactions, fraud rate must converge within 1 percentage point of target (SC-002, SC-003).
