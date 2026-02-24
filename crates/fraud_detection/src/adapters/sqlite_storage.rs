// Rust guideline compliant 2026-02-16

//! SQLite adapter for the `Storage` port (demo).
//!
//! Persists `PendingTransaction` rows to a SQLite file via `sqlx`.
//! Proves that the hexagonal `Storage` port is truly swappable without
//! touching domain or pipeline crates.
//!
//! # Dependency note
//!
//! `sqlx` is a hard dependency (no feature flag). This is intentional for
//! a proof-of-concept binary where build-complexity trade-offs favour
//! simplicity over optional compilation. Adding a Cargo feature would
//! require conditional compilation across main entry points with no real
//! benefit at demo scale.
//!
//! # `INSERT OR REPLACE` semantics
//!
//! Duplicate transaction UUIDs are silently overwritten. This is acceptable
//! for a demo adapter where idempotency is preferred over strict append-only
//! semantics. A production adapter should use plain `INSERT` and propagate
//! the constraint-violation error.

use domain::{PendingTransaction, Storage, StorageError};

/// `Storage` adapter backed by a SQLite database file via `sqlx`.
///
/// Connects to (or creates) a SQLite file and ensures the
/// `pending_transactions` table exists. Duplicate UUIDs are silently
/// overwritten (INSERT OR REPLACE -- see module-level note).
#[derive(Debug, Clone)]
pub struct SqliteStorage {
    pool: sqlx::SqlitePool,
}

impl SqliteStorage {
    /// Open or create a SQLite database and initialize the schema.
    ///
    /// Passes `create_if_missing(true)` so the database file is created on
    /// first run without manual setup. The `pending_transactions` table is
    /// created via `CREATE TABLE IF NOT EXISTS`, making repeated calls safe.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` when the connection or schema creation fails.
    #[must_use]
    pub async fn new(db_url: &str) -> Result<Self, sqlx::Error> {
        // create_if_missing: sqlx 0.8 defaults to false for file databases;
        // enable explicitly so the demo works out of the box on first run.
        let opts = db_url
            .parse::<sqlx::sqlite::SqliteConnectOptions>()?
            .create_if_missing(true);
        let pool = sqlx::SqlitePool::connect_with(opts).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS pending_transactions (
                id              TEXT    PRIMARY KEY,
                amount          REAL    NOT NULL,
                last_name       TEXT    NOT NULL,
                predicted_fraud INTEGER NOT NULL,
                model_name      TEXT    NOT NULL,
                model_version   TEXT    NOT NULL,
                is_reviewed     INTEGER NOT NULL DEFAULT 0,
                actual_fraud    INTEGER           -- NULL / 0 / 1
            )",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
}

impl Storage for SqliteStorage {
    /// Persist each item in `batch` to the SQLite `pending_transactions` table.
    ///
    /// Uses `INSERT OR REPLACE` -- duplicate UUIDs are silently overwritten
    /// (see module-level note). `actual_fraud` maps `Option<bool>` to a
    /// nullable SQLite INTEGER: `None` = NULL, `Some(false)` = 0, `Some(true)` = 1.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Unavailable` on any `sqlx` error (connection
    /// failure, disk full, constraint violation, etc.). The underlying error
    /// is logged at `error` level before mapping.
    async fn write_batch(&self, batch: Vec<PendingTransaction>) -> Result<(), StorageError> {
        for pt in batch {
            let tx = &pt.inferred_transaction.transaction;
            let it = &pt.inferred_transaction;
            // Map Option<bool> -> Option<i64> for the nullable INTEGER column:
            // None = NULL, Some(false) = 0, Some(true) = 1.
            let actual_fraud: Option<i64> = pt.actual_fraud.map(i64::from);
            sqlx::query(
                "INSERT OR REPLACE INTO pending_transactions
                 (id, amount, last_name, predicted_fraud, model_name,
                  model_version, is_reviewed, actual_fraud)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(tx.id.to_string())
            .bind(tx.amount)
            .bind(&tx.last_name)
            .bind(i64::from(it.predicted_fraud))
            .bind(&it.model_name)
            .bind(&it.model_version)
            .bind(i64::from(pt.is_reviewed))
            .bind(actual_fraud)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                log::error!("sqlite.write_batch: {e}");
                StorageError::Unavailable
            })?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::SqliteStorage;
    use domain::{InferredTransaction, PendingTransaction, Storage as _, Transaction};
    use uuid::Uuid;

    // Each test calls make_storage() which opens a fresh SqlitePool backed by
    // an in-memory SQLite database.  Because every call constructs a new pool
    // (and therefore a new in-memory DB), tests are fully isolated with no
    // on-disk side-effects.
    async fn make_storage() -> SqliteStorage {
        SqliteStorage::new("sqlite::memory:")
            .await
            .expect("in-memory SQLite should open")
    }

    fn make_pending(id: Uuid, actual_fraud: Option<bool>) -> PendingTransaction {
        PendingTransaction {
            inferred_transaction: InferredTransaction {
                transaction: Transaction {
                    id,
                    amount: 1.00_f64,
                    last_name: "Test".to_owned(),
                },
                predicted_fraud: false,
                model_name: "DEMO".to_owned(),
                model_version: "4".to_owned(),
            },
            is_reviewed: false,
            actual_fraud,
        }
    }

    // SS-T01: write_batch persists the correct number of rows.
    #[tokio::test]
    async fn write_batch_stores_all_items() {
        let storage = make_storage().await;
        storage
            .write_batch(vec![
                make_pending(Uuid::new_v4(), None),
                make_pending(Uuid::new_v4(), None),
            ])
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pending_transactions")
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(count, 2);
    }

    // SS-T02: actual_fraud column is NULL when Option<bool> is None.
    #[tokio::test]
    async fn actual_fraud_null_when_none() {
        let storage = make_storage().await;
        let id = Uuid::new_v4();
        storage.write_batch(vec![make_pending(id, None)]).await.unwrap();
        let val: Option<i64> =
            sqlx::query_scalar("SELECT actual_fraud FROM pending_transactions WHERE id = ?")
                .bind(id.to_string())
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert!(val.is_none(), "expected NULL, got {val:?}");
    }

    // SS-T03: actual_fraud column is 1 when Option<bool> is Some(true).
    #[tokio::test]
    async fn actual_fraud_1_when_some_true() {
        let storage = make_storage().await;
        let id = Uuid::new_v4();
        storage.write_batch(vec![make_pending(id, Some(true))]).await.unwrap();
        let val: Option<i64> =
            sqlx::query_scalar("SELECT actual_fraud FROM pending_transactions WHERE id = ?")
                .bind(id.to_string())
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(val, Some(1), "expected Some(1), got {val:?}");
    }

    // SS-T04: actual_fraud column is 0 when Option<bool> is Some(false).
    #[tokio::test]
    async fn actual_fraud_0_when_some_false() {
        let storage = make_storage().await;
        let id = Uuid::new_v4();
        storage.write_batch(vec![make_pending(id, Some(false))]).await.unwrap();
        let val: Option<i64> =
            sqlx::query_scalar("SELECT actual_fraud FROM pending_transactions WHERE id = ?")
                .bind(id.to_string())
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(val, Some(0), "expected Some(0), got {val:?}");
    }

    // SS-T05: second write with duplicate UUID overwrites the first row (INSERT OR REPLACE).
    #[tokio::test]
    async fn duplicate_id_is_overwritten() {
        let storage = make_storage().await;
        let id = Uuid::new_v4();
        // First write: actual_fraud = NULL.
        storage.write_batch(vec![make_pending(id, None)]).await.unwrap();
        // Second write: same UUID, actual_fraud = Some(true).
        storage.write_batch(vec![make_pending(id, Some(true))]).await.unwrap();
        // Exactly one row must exist (REPLACE removed the first).
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pending_transactions")
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(count, 1, "expected 1 row after REPLACE, got {count}");
        // The surviving row must carry the second write's value.
        let val: Option<i64> =
            sqlx::query_scalar("SELECT actual_fraud FROM pending_transactions WHERE id = ?")
                .bind(id.to_string())
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(val, Some(1));
    }

    // SS-T06: empty batch returns Ok and leaves the table untouched.
    #[tokio::test]
    async fn empty_batch_is_ok() {
        let storage = make_storage().await;
        storage.write_batch(vec![]).await.unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pending_transactions")
                .fetch_one(&storage.pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
    }
}
