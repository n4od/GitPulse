//! Key/value bookkeeping: poller cursors, settings blob, maintenance clocks.
//!
//! Secrets never belong here — tokens live in the OS keychain.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppError;

pub fn get(connection: &Connection, key: &str) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::db)
}

pub fn set(connection: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO meta (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, Utc::now()],
        )
        .map_err(AppError::db)?;

    Ok(())
}

pub fn remove(connection: &Connection, key: &str) -> Result<(), AppError> {
    connection
        .execute("DELETE FROM meta WHERE key = ?1", params![key])
        .map_err(AppError::db)?;

    Ok(())
}
