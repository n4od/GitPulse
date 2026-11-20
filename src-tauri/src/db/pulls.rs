//! Open pull requests.
//!
//! Unlike runs and deployments, pull requests are a *snapshot*: the API returns
//! the currently-open set, not an append-only history. Every sync stamps
//! `last_seen_at`, and [`sweep_unseen`] removes rows that the latest sync did
//! not report — otherwise a pull request merged on GitHub would keep showing as
//! open here forever.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};

use crate::error::AppError;

use super::models::{PullRequest, PullRequestUpsert};

const COLUMNS: &str = "id, repo_id, github_id, number, title, state, is_draft, author, \
                       review_state, checks_state, html_url, updated_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<PullRequest> {
    Ok(PullRequest {
        id: row.get(0)?,
        repo_id: row.get(1)?,
        github_id: row.get(2)?,
        number: row.get(3)?,
        title: row.get(4)?,
        state: row.get(5)?,
        is_draft: row.get(6)?,
        author: row.get(7)?,
        review_state: row.get(8)?,
        checks_state: row.get(9)?,
        html_url: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub fn upsert(connection: &Connection, pull: &PullRequestUpsert) -> Result<PullRequest, AppError> {
    connection
        .query_row(
            &format!(
                "INSERT INTO pull_requests (repo_id, github_id, number, title, state, is_draft,
                                            author, review_state, checks_state, html_url,
                                            updated_at, last_seen_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(github_id) DO UPDATE SET
                     number       = excluded.number,
                     title        = excluded.title,
                     state        = excluded.state,
                     is_draft     = excluded.is_draft,
                     author       = excluded.author,
                     review_state = excluded.review_state,
                     checks_state = excluded.checks_state,
                     html_url     = excluded.html_url,
                     updated_at   = excluded.updated_at,
                     last_seen_at = excluded.last_seen_at
                 RETURNING {COLUMNS}"
            ),
            params![
                pull.repo_id,
                pull.github_id,
                pull.number,
                pull.title,
                pull.state,
                pull.is_draft,
                pull.author,
                pull.review_state,
                pull.checks_state,
                pull.html_url,
                pull.updated_at,
                Utc::now(),
            ],
            from_row,
        )
        .map_err(AppError::db)
}

pub fn list_by_repo(connection: &Connection, repo_id: i64) -> Result<Vec<PullRequest>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {COLUMNS} FROM pull_requests
             WHERE repo_id = ?1 AND state = 'open'
             ORDER BY updated_at DESC, id DESC"
        ))
        .map_err(AppError::db)?;

    let rows = statement
        .query_map(params![repo_id], from_row)
        .map_err(AppError::db)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AppError::db)
}

/// Deletes open pull requests not seen since `since`.
///
/// Call with the timestamp taken *before* the sync that upserted the fresh
/// rows; anything older than that was not in the response.
pub fn sweep_unseen(
    connection: &Connection,
    repo_id: i64,
    since: DateTime<Utc>,
) -> Result<usize, AppError> {
    connection
        .execute(
            "DELETE FROM pull_requests
             WHERE repo_id = ?1 AND state = 'open' AND last_seen_at < ?2",
            params![repo_id, since],
        )
        .map_err(AppError::db)
}
