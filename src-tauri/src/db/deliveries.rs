//! Webhook delivery log, used to make ingestion idempotent.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::error::AppError;

/// Records a delivery. Returns `true` when it is new, `false` when it is a
/// replay — GitHub retries, and a relay can resend after a reconnect, so a
/// `false` here means "already processed, ignore it".
pub fn record(
    connection: &Connection,
    delivery_id: &str,
    event: &str,
    repo_full_name: Option<&str>,
) -> Result<bool, AppError> {
    let inserted = connection
        .execute(
            "INSERT OR IGNORE INTO webhook_deliveries (delivery_id, event, repo_full_name, received_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![delivery_id, event, repo_full_name, Utc::now()],
        )
        .map_err(AppError::db)?;

    Ok(inserted == 1)
}

pub fn seen(connection: &Connection, delivery_id: &str) -> Result<bool, AppError> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE delivery_id = ?1",
            params![delivery_id],
            |row| row.get(0),
        )
        .map_err(AppError::db)?;

    Ok(count > 0)
}

pub fn count(connection: &Connection) -> Result<i64, AppError> {
    connection
        .query_row("SELECT COUNT(*) FROM webhook_deliveries", [], |row| {
            row.get(0)
        })
        .map_err(AppError::db)
}

/// When the last delivery arrived, for the settings screen.
pub fn last_received_at(connection: &Connection) -> Result<Option<DateTime<Utc>>, AppError> {
    connection
        .query_row(
            "SELECT MAX(received_at) FROM webhook_deliveries",
            [],
            |row| row.get::<_, Option<DateTime<Utc>>>(0),
        )
        .map_err(AppError::db)
}

pub fn delete_received_before(
    connection: &Connection,
    cutoff: DateTime<Utc>,
) -> Result<usize, AppError> {
    connection
        .execute(
            "DELETE FROM webhook_deliveries WHERE received_at < ?1",
            params![cutoff],
        )
        .map_err(AppError::db)
}
