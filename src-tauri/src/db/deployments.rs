//! Deployments and their latest status.

use rusqlite::{params, Connection, Row};

use crate::error::AppError;

use super::models::{Deployment, DeploymentUpsert};

const COLUMNS: &str = "id, repo_id, github_id, environment, git_ref, sha, state, html_url, \
                       updated_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Deployment> {
    Ok(Deployment {
        id: row.get(0)?,
        repo_id: row.get(1)?,
        github_id: row.get(2)?,
        environment: row.get(3)?,
        git_ref: row.get(4)?,
        sha: row.get(5)?,
        state: row.get(6)?,
        html_url: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn upsert(
    connection: &Connection,
    deployment: &DeploymentUpsert,
) -> Result<Deployment, AppError> {
    connection
        .query_row(
            &format!(
                "INSERT INTO deployments (repo_id, github_id, environment, git_ref, sha, state,
                                          html_url, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(github_id) DO UPDATE SET
                     environment = excluded.environment,
                     git_ref     = excluded.git_ref,
                     sha         = excluded.sha,
                     state       = excluded.state,
                     html_url    = excluded.html_url,
                     updated_at  = excluded.updated_at
                 RETURNING {COLUMNS}"
            ),
            params![
                deployment.repo_id,
                deployment.github_id,
                deployment.environment,
                deployment.git_ref,
                deployment.sha,
                deployment.state,
                deployment.html_url,
                deployment.updated_at,
            ],
            from_row,
        )
        .map_err(AppError::db)
}

pub fn list_by_repo(
    connection: &Connection,
    repo_id: i64,
    limit: i64,
) -> Result<Vec<Deployment>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {COLUMNS} FROM deployments
             WHERE repo_id = ?1
             ORDER BY updated_at DESC, id DESC
             LIMIT ?2"
        ))
        .map_err(AppError::db)?;

    let rows = statement
        .query_map(params![repo_id, limit], from_row)
        .map_err(AppError::db)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AppError::db)
}
