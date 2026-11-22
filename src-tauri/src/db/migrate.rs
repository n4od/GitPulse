//! Forward-only schema migrations, applied inside a transaction.
//!
//! Each migration is a `.sql` file embedded at compile time. Applied versions
//! are recorded in `schema_migrations`, so re-running is a no-op — which is
//! what makes `Db::open` safe to call on every launch.
//!
//! Migrations are never edited after release: a shipped version has already
//! run on someone's machine. Corrections go in a new file.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppError;

/// Ordered list of `(version, sql)`. Append only.
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../../migrations/0001_init.sql")),
    (
        "0002_webhook_deliveries",
        include_str!("../../migrations/0002_webhook_deliveries.sql"),
    ),
    (
        "0003_repo_last_error",
        include_str!("../../migrations/0003_repo_last_error.sql"),
    ),
    (
        "0004_repo_error_detail",
        include_str!("../../migrations/0004_repo_error_detail.sql"),
    ),
    (
        "0005_drop_webhook_deliveries",
        include_str!("../../migrations/0005_drop_webhook_deliveries.sql"),
    ),
    (
        "0006_drop_webhook_settings",
        include_str!("../../migrations/0006_drop_webhook_settings.sql"),
    ),
];

const CREATE_LEDGER: &str = "
CREATE TABLE IF NOT EXISTS schema_migrations (
    version    TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL
);
";

/// Applies every migration that has not been applied yet.
pub fn apply(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(CREATE_LEDGER)
        .map_err(AppError::db)?;

    for (version, sql) in MIGRATIONS {
        if is_applied(connection, version)? {
            continue;
        }

        // `unchecked_transaction` works on `&Connection`, which keeps every
        // caller free to hold a shared borrow.
        let transaction = connection.unchecked_transaction().map_err(AppError::db)?;
        transaction.execute_batch(sql).map_err(AppError::db)?;
        transaction
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![version, Utc::now()],
            )
            .map_err(AppError::db)?;
        transaction.commit().map_err(AppError::db)?;

        tracing::info!(version, "applied database migration");
    }

    Ok(())
}

fn is_applied(connection: &Connection, version: &str) -> Result<bool, AppError> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![version],
            |row| row.get(0),
        )
        .map_err(AppError::db)?;

    Ok(count > 0)
}

/// Highest applied version, for diagnostics.
pub fn current_version(connection: &Connection) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::db)
}
