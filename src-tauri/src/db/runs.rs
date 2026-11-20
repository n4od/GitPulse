//! Workflow runs.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::AppError;

use super::models::{RunUpsert, WorkflowRun};

const COLUMNS: &str = "id, repo_id, github_id, workflow_name, display_title, event, branch, \
                       head_sha, status, conclusion, run_number, run_attempt, html_url, \
                       actor_login, created_at, run_started_at, updated_at";

/// Statuses that mean a run is still going to change on its own.
pub const ACTIVE_STATUSES: &[&str] = &["queued", "waiting", "pending", "requested", "in_progress"];

fn from_row(row: &Row<'_>) -> rusqlite::Result<WorkflowRun> {
    Ok(WorkflowRun {
        id: row.get(0)?,
        repo_id: row.get(1)?,
        github_id: row.get(2)?,
        workflow_name: row.get(3)?,
        display_title: row.get(4)?,
        event: row.get(5)?,
        branch: row.get(6)?,
        head_sha: row.get(7)?,
        status: row.get(8)?,
        conclusion: row.get(9)?,
        run_number: row.get(10)?,
        run_attempt: row.get(11)?,
        html_url: row.get(12)?,
        actor_login: row.get(13)?,
        created_at: row.get(14)?,
        run_started_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

/// What a poll compares against to decide whether a run actually changed.
///
/// `updated_at` is deliberately excluded: GitHub bumps it on transitions we do
/// not render, and treating that as a change would emit a spurious UI update on
/// every poll. Re-running a workflow moves `run_attempt`, which is included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFingerprint {
    pub status: String,
    pub conclusion: Option<String>,
    pub run_attempt: Option<i64>,
}

impl RunFingerprint {
    #[must_use]
    pub fn of(run: &RunUpsert) -> Self {
        Self {
            status: run.status.clone(),
            conclusion: run.conclusion.clone(),
            run_attempt: run.run_attempt,
        }
    }
}

/// Inserts or updates a run and returns the stored row.
///
/// `repo_id` is not updated on conflict: a run belongs to one repository for
/// its whole life, and letting a bad payload move it would corrupt the cache.
pub fn upsert(connection: &Connection, run: &RunUpsert) -> Result<WorkflowRun, AppError> {
    connection
        .query_row(
            &format!(
                "INSERT INTO workflow_runs (repo_id, github_id, workflow_name, display_title,
                                            event, branch, head_sha, status, conclusion,
                                            run_number, run_attempt, html_url, actor_login,
                                            created_at, run_started_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                 ON CONFLICT(github_id) DO UPDATE SET
                     workflow_name  = excluded.workflow_name,
                     display_title  = excluded.display_title,
                     event          = excluded.event,
                     branch         = excluded.branch,
                     head_sha       = excluded.head_sha,
                     status         = excluded.status,
                     conclusion     = excluded.conclusion,
                     run_number     = excluded.run_number,
                     run_attempt    = excluded.run_attempt,
                     html_url       = excluded.html_url,
                     actor_login    = excluded.actor_login,
                     created_at     = excluded.created_at,
                     run_started_at = excluded.run_started_at,
                     updated_at     = excluded.updated_at
                 RETURNING {COLUMNS}"
            ),
            params![
                run.repo_id,
                run.github_id,
                run.workflow_name,
                run.display_title,
                run.event,
                run.branch,
                run.head_sha,
                run.status,
                run.conclusion,
                run.run_number,
                run.run_attempt,
                run.html_url,
                run.actor_login,
                run.created_at,
                run.run_started_at,
                run.updated_at,
            ],
            from_row,
        )
        .map_err(AppError::db)
}

pub fn by_id(connection: &Connection, id: i64) -> Result<Option<WorkflowRun>, AppError> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM workflow_runs WHERE id = ?1"),
            params![id],
            from_row,
        )
        .optional()
        .map_err(AppError::db)
}

/// Newest first. `limit` of `-1` means "no limit" in `SQLite`.
pub fn list_by_repo(
    connection: &Connection,
    repo_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<WorkflowRun>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {COLUMNS} FROM workflow_runs
             WHERE repo_id = ?1
             ORDER BY updated_at DESC, id DESC
             LIMIT ?2 OFFSET ?3"
        ))
        .map_err(AppError::db)?;

    let rows = statement
        .query_map(params![repo_id, limit, offset], from_row)
        .map_err(AppError::db)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AppError::db)
}

pub fn latest_for_repo(
    connection: &Connection,
    repo_id: i64,
) -> Result<Option<WorkflowRun>, AppError> {
    connection
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM workflow_runs
                 WHERE repo_id = ?1
                 ORDER BY updated_at DESC, id DESC
                 LIMIT 1"
            ),
            params![repo_id],
            from_row,
        )
        .optional()
        .map_err(AppError::db)
}

/// Status of every run we already hold for a repository, keyed by GitHub id.
/// Used by [`super::diff`] to skip writes for runs that have not changed.
pub fn fingerprints(
    connection: &Connection,
    repo_id: i64,
) -> Result<std::collections::HashMap<i64, RunFingerprint>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT github_id, status, conclusion, run_attempt
             FROM workflow_runs WHERE repo_id = ?1",
        )
        .map_err(AppError::db)?;

    let rows = statement
        .query_map(params![repo_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                RunFingerprint {
                    status: row.get(1)?,
                    conclusion: row.get(2)?,
                    run_attempt: row.get(3)?,
                },
            ))
        })
        .map_err(AppError::db)?;

    let mut result = std::collections::HashMap::new();
    for row in rows {
        let (github_id, fingerprint) = row.map_err(AppError::db)?;
        result.insert(github_id, fingerprint);
    }

    Ok(result)
}

/// Whether any run for this repository is still in flight. The poller uses this
/// to decide between the fast and the slow interval.
pub fn has_active(connection: &Connection, repo_id: i64) -> Result<bool, AppError> {
    let placeholders = ACTIVE_STATUSES
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "SELECT EXISTS(
             SELECT 1 FROM workflow_runs
             WHERE repo_id = ?1 AND status IN ({placeholders})
         )"
    );

    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(ACTIVE_STATUSES.len() + 1);
    values.push(Box::new(repo_id));
    for status in ACTIVE_STATUSES {
        values.push(Box::new((*status).to_owned()));
    }

    connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values.iter().map(AsRef::as_ref)),
            |row| row.get(0),
        )
        .map_err(AppError::db)
}

/// Every repository with at least one run in flight, in one query.
///
/// The poller asks this once per tick rather than once per repository, which is
/// what keeps a 250-repository sweep to a couple of queries per tick.
pub fn repos_with_active_runs(connection: &Connection) -> Result<Vec<i64>, AppError> {
    let placeholders = ACTIVE_STATUSES
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let sql =
        format!("SELECT DISTINCT repo_id FROM workflow_runs WHERE status IN ({placeholders})");

    let mut statement = connection.prepare(&sql).map_err(AppError::db)?;
    let rows = statement
        .query_map(
            rusqlite::params_from_iter(ACTIVE_STATUSES.iter().copied()),
            |row| row.get(0),
        )
        .map_err(AppError::db)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AppError::db)
}

/// Recent runs for every repository, in a single query.
///
/// The dashboard needs this for every card at once; asking per repository would
/// turn one screen into 250 queries.
pub fn recent_for_all(
    connection: &Connection,
    per_repo: i64,
) -> Result<std::collections::HashMap<i64, Vec<WorkflowRun>>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {COLUMNS} FROM (
                 SELECT *,
                        ROW_NUMBER() OVER (
                            PARTITION BY repo_id ORDER BY updated_at DESC, id DESC
                        ) AS rank
                 FROM workflow_runs
             ) WHERE rank <= ?1
             ORDER BY repo_id, rank"
        ))
        .map_err(AppError::db)?;

    let rows = statement
        .query_map(params![per_repo], from_row)
        .map_err(AppError::db)?;

    let mut recent: std::collections::HashMap<i64, Vec<WorkflowRun>> =
        std::collections::HashMap::new();

    for row in rows {
        let run = row.map_err(AppError::db)?;
        recent.entry(run.repo_id).or_default().push(run);
    }

    Ok(recent)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunAggregate {
    pub failing: i64,
    pub active: bool,
}

/// Failing and active counts for every repository, in one pass.
pub fn aggregates(
    connection: &Connection,
) -> Result<std::collections::HashMap<i64, RunAggregate>, AppError> {
    let placeholders = ACTIVE_STATUSES
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "SELECT repo_id,
                SUM(CASE WHEN conclusion IN ('failure', 'timed_out') THEN 1 ELSE 0 END),
                SUM(CASE WHEN status IN ({placeholders}) THEN 1 ELSE 0 END)
         FROM workflow_runs
         GROUP BY repo_id"
    );

    let mut statement = connection.prepare(&sql).map_err(AppError::db)?;

    let rows = statement
        .query_map(
            rusqlite::params_from_iter(ACTIVE_STATUSES.iter().copied()),
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    RunAggregate {
                        failing: row.get(1)?,
                        active: row.get::<_, i64>(2)? > 0,
                    },
                ))
            },
        )
        .map_err(AppError::db)?;

    let mut result = std::collections::HashMap::new();
    for row in rows {
        let (repo_id, aggregate) = row.map_err(AppError::db)?;
        result.insert(repo_id, aggregate);
    }

    Ok(result)
}

/// How many runs for this repository ended in failure.
pub fn count_failing(connection: &Connection, repo_id: i64) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM workflow_runs
             WHERE repo_id = ?1 AND conclusion IN ('failure', 'timed_out')",
            params![repo_id],
            |row| row.get(0),
        )
        .map_err(AppError::db)
}

pub fn count_by_repo(connection: &Connection, repo_id: i64) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM workflow_runs WHERE repo_id = ?1",
            params![repo_id],
            |row| row.get(0),
        )
        .map_err(AppError::db)
}

/// Retention sweep. Returns the number of rows removed.
pub fn delete_updated_before(
    connection: &Connection,
    cutoff: DateTime<Utc>,
) -> Result<usize, AppError> {
    connection
        .execute(
            "DELETE FROM workflow_runs WHERE updated_at < ?1",
            params![cutoff],
        )
        .map_err(AppError::db)
}
