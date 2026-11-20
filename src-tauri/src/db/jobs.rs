//! Jobs belonging to a workflow run.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::AppError;

use super::models::{Job, JobUpsert};

const COLUMNS: &str = "id, run_id, github_id, name, status, conclusion, started_at, \
                       completed_at, html_url, updated_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: row.get(0)?,
        run_id: row.get(1)?,
        github_id: row.get(2)?,
        name: row.get(3)?,
        status: row.get(4)?,
        conclusion: row.get(5)?,
        started_at: row.get(6)?,
        completed_at: row.get(7)?,
        html_url: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn upsert(connection: &Connection, job: &JobUpsert) -> Result<Job, AppError> {
    connection
        .query_row(
            &format!(
                "INSERT INTO jobs (run_id, github_id, name, status, conclusion, started_at,
                                   completed_at, html_url, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(github_id) DO UPDATE SET
                     name         = excluded.name,
                     status       = excluded.status,
                     conclusion   = excluded.conclusion,
                     started_at   = excluded.started_at,
                     completed_at = excluded.completed_at,
                     html_url     = excluded.html_url,
                     updated_at   = excluded.updated_at
                 RETURNING {COLUMNS}"
            ),
            params![
                job.run_id,
                job.github_id,
                job.name,
                job.status,
                job.conclusion,
                job.started_at,
                job.completed_at,
                job.html_url,
                job.updated_at,
            ],
            from_row,
        )
        .map_err(AppError::db)
}

/// Replaces every stored job for a run.
///
/// GitHub issues new job ids on every re-run, so an upsert alone would leave
/// the previous attempt's jobs in the list next to the new ones.
pub fn replace(
    connection: &Connection,
    run_id: i64,
    jobs: &[JobUpsert],
) -> Result<Vec<Job>, AppError> {
    let transaction = connection.unchecked_transaction().map_err(AppError::db)?;

    transaction
        .execute("DELETE FROM jobs WHERE run_id = ?1", params![run_id])
        .map_err(AppError::db)?;

    let mut stored = Vec::with_capacity(jobs.len());
    for job in jobs {
        stored.push(upsert(&transaction, job)?);
    }

    transaction.commit().map_err(AppError::db)?;

    Ok(stored)
}

pub fn list_by_run(connection: &Connection, run_id: i64) -> Result<Vec<Job>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {COLUMNS} FROM jobs WHERE run_id = ?1 ORDER BY id ASC"
        ))
        .map_err(AppError::db)?;

    let rows = statement
        .query_map(params![run_id], from_row)
        .map_err(AppError::db)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AppError::db)
}

pub fn count_by_run(connection: &Connection, run_id: i64) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .map_err(AppError::db)
}

/// Everything fetching a job's log needs.
pub struct JobContext {
    pub full_name: String,
    pub github_job_id: i64,
    pub run_id: i64,
}

pub fn context(connection: &Connection, job_id: i64) -> Result<Option<JobContext>, AppError> {
    connection
        .query_row(
            "SELECT repos.full_name, jobs.github_id, jobs.run_id
             FROM jobs
             JOIN workflow_runs AS runs ON runs.id = jobs.run_id
             JOIN repos ON repos.id = runs.repo_id
             WHERE jobs.id = ?1",
            params![job_id],
            |row| {
                Ok(JobContext {
                    full_name: row.get(0)?,
                    github_job_id: row.get(1)?,
                    run_id: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(AppError::db)
}
