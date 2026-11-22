//! Diagnostic numbers for the settings screen.

use serde::Serialize;
use ts_rs::TS;

use crate::db::{self, Db};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct SystemMetrics {
    /// Size of the cache on disk, including the write-ahead log.
    pub db_size_mb: f64,
    pub monitored_repos: usize,
    pub cached_runs: i64,
    pub deliveries: i64,
    /// Requests the local bucket will still hand out right now.
    pub budget_available: u32,
    #[ts(type = "number")]
    #[ts(type = "number")]
    pub uptime_secs: u64,
}

/// Reads what the settings screen can honestly report.
///
/// Deliberately absent: resident memory. Reading it needs a platform call
/// through `unsafe`, which this crate forbids, and a dependency that exists to
/// fetch one number is not worth the binary it costs.
pub async fn collect(
    db: &Db,
    started_at: chrono::DateTime<chrono::Utc>,
    budget_available: u32,
) -> Result<SystemMetrics, AppError> {
    let monitored_repos = db
        .with_connection(|connection| Ok(db::repos::list_monitored(connection)?.len()))
        .await?;

    let (cached_runs, deliveries) = db
        .with_connection(|connection| {
            let runs: i64 = connection
                .query_row("SELECT COUNT(*) FROM workflow_runs", [], |row| row.get(0))
                .map_err(AppError::db)?;

            Ok((runs, db::deliveries::count(connection)?))
        })
        .await?;

    let db_size_mb = db.path().map_or(0.0, |path| spanned_size_mb(&path));

    let uptime_secs = (chrono::Utc::now() - started_at)
        .to_std()
        .map_or(0, |elapsed| elapsed.as_secs());

    Ok(SystemMetrics {
        db_size_mb,
        monitored_repos,
        cached_runs,
        deliveries,
        budget_available,
        uptime_secs,
    })
}

/// The database plus its `-wal` and `-shm` companions, which can dwarf it.
fn spanned_size_mb(path: &std::path::Path) -> f64 {
    let mut total = 0_u64;

    for suffix in ["", "-wal", "-shm"] {
        let candidate = if suffix.is_empty() {
            path.to_path_buf()
        } else {
            path.with_extension(format!(
                "{}{suffix}",
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("db")
            ))
        };

        if let Ok(metadata) = std::fs::metadata(&candidate) {
            total += metadata.len();
        }
    }

    // A megabyte is small enough here that the precision loss is irrelevant.
    #[allow(clippy::cast_precision_loss)]
    let megabytes = total as f64 / (1024.0 * 1024.0);

    megabytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_database_reports_zero() {
        assert!(spanned_size_mb(std::path::Path::new("does-not-exist.db")) < f64::EPSILON);
    }
}
