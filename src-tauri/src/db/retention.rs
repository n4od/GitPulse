//! Retention: keep the cache bounded without losing anything the user needs.
//!
//! Runs older than [`DEFAULT_RUN_RETENTION_DAYS`] and webhook deliveries older
//! than [`DEFAULT_DELIVERY_RETENTION_DAYS`] are deleted. `VACUUM` runs at most
//! once every [`VACUUM_INTERVAL_DAYS`] because it rewrites the whole file and
//! blocks the connection while it does.

use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;

use crate::error::AppError;

use super::{deliveries, meta, runs};

pub const DEFAULT_RUN_RETENTION_DAYS: i64 = 90;
pub const DEFAULT_DELIVERY_RETENTION_DAYS: i64 = 30;
const VACUUM_INTERVAL_DAYS: i64 = 30;
const LAST_VACUUM_KEY: &str = "last_vacuum_at";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RetentionReport {
    pub runs_deleted: usize,
    pub deliveries_deleted: usize,
    pub vacuumed: bool,
}

/// Runs the maintenance sweep with the shipped defaults.
pub fn prune_with_defaults(connection: &Connection) -> Result<RetentionReport, AppError> {
    prune(
        connection,
        DEFAULT_RUN_RETENTION_DAYS,
        DEFAULT_DELIVERY_RETENTION_DAYS,
    )
}

pub fn prune(
    connection: &Connection,
    run_retention_days: i64,
    delivery_retention_days: i64,
) -> Result<RetentionReport, AppError> {
    let now = Utc::now();

    let runs_deleted =
        runs::delete_updated_before(connection, now - Duration::days(run_retention_days))?;
    let deliveries_deleted = deliveries::delete_received_before(
        connection,
        now - Duration::days(delivery_retention_days),
    )?;

    let vacuumed = maybe_vacuum(connection, now)?;

    Ok(RetentionReport {
        runs_deleted,
        deliveries_deleted,
        vacuumed,
    })
}

/// Vacuums at most once per [`VACUUM_INTERVAL_DAYS`].
///
/// Must not be called inside an open transaction — `SQLite` refuses to run
/// `VACUUM` there. Every caller in this crate does so outside one.
fn maybe_vacuum(connection: &Connection, now: DateTime<Utc>) -> Result<bool, AppError> {
    let last = meta::get(connection, LAST_VACUUM_KEY)?
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|parsed| parsed.with_timezone(&Utc));

    if let Some(last) = last {
        if now.signed_duration_since(last) < Duration::days(VACUUM_INTERVAL_DAYS) {
            return Ok(false);
        }
    }

    connection.execute_batch("VACUUM").map_err(AppError::db)?;
    meta::set(connection, LAST_VACUUM_KEY, &now.to_rfc3339())?;

    Ok(true)
}
