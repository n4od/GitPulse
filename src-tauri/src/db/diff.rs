//! Write-amplification control for polling.
//!
//! A poll fetches the same runs over and over; most of them have not changed.
//! Writing every row on every poll would churn the WAL and, worse, emit a UI
//! event per row per poll. [`diff_runs`] compares a fingerprint of the fields
//! we actually render and returns only the runs that moved.

use rusqlite::Connection;

use crate::error::AppError;

use super::models::{RunUpsert, WorkflowRun};
use super::runs::{self, RunFingerprint};

#[derive(Debug, Default)]
pub struct RunDiff {
    /// Runs that were inserted or whose visible state changed, newest state.
    pub changed: Vec<WorkflowRun>,
    /// Runs that were already up to date and were not written.
    pub unchanged: usize,
}

impl RunDiff {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty()
    }

    #[must_use]
    pub fn considered(&self) -> usize {
        self.changed.len() + self.unchanged
    }
}

/// Upserts only the runs whose fingerprint differs from what we already hold.
///
/// The whole batch is one transaction so a partially applied poll cannot leave
/// the cache in a state that disagrees with itself.
pub fn diff_runs(
    connection: &Connection,
    repo_id: i64,
    incoming: &[RunUpsert],
) -> Result<RunDiff, AppError> {
    let known = runs::fingerprints(connection, repo_id)?;

    let transaction = connection.unchecked_transaction().map_err(AppError::db)?;
    let mut diff = RunDiff::default();

    for run in incoming {
        let fingerprint = RunFingerprint::of(run);

        match known.get(&run.github_id) {
            Some(existing) if *existing == fingerprint => diff.unchanged += 1,
            _ => diff.changed.push(runs::upsert(&transaction, run)?),
        }
    }

    transaction.commit().map_err(AppError::db)?;

    Ok(diff)
}
