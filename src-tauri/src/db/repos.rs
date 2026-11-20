//! Repository records.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::AppError;

use super::models::{Repo, RepoUpsert};

const COLUMNS: &str = "id, github_id, owner, name, full_name, is_private, default_branch, \
                       avatar_url, monitored, last_polled_at, created_at, updated_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Repo> {
    Ok(Repo {
        id: row.get(0)?,
        github_id: row.get(1)?,
        owner: row.get(2)?,
        name: row.get(3)?,
        full_name: row.get(4)?,
        is_private: row.get(5)?,
        default_branch: row.get(6)?,
        avatar_url: row.get(7)?,
        monitored: row.get(8)?,
        last_polled_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

/// Inserts or refreshes a repository, returning its local row id.
///
/// `monitored` and `created_at` are intentionally absent from the update list:
/// the first is a user preference that a sync must never reset, the second is
/// the moment we first saw the repo.
pub fn upsert(connection: &Connection, repo: &RepoUpsert) -> Result<i64, AppError> {
    let now = Utc::now();

    connection
        .query_row(
            "INSERT INTO repos (github_id, owner, name, full_name, is_private, default_branch,
                                avatar_url, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(github_id) DO UPDATE SET
                 owner          = excluded.owner,
                 name           = excluded.name,
                 full_name      = excluded.full_name,
                 is_private     = excluded.is_private,
                 default_branch = excluded.default_branch,
                 avatar_url     = excluded.avatar_url,
                 updated_at     = excluded.updated_at
             RETURNING id",
            params![
                repo.github_id,
                repo.owner,
                repo.name,
                repo.full_name,
                repo.is_private,
                repo.default_branch,
                repo.avatar_url,
                now,
            ],
            |row| row.get(0),
        )
        .map_err(AppError::db)
}

pub fn by_id(connection: &Connection, id: i64) -> Result<Option<Repo>, AppError> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM repos WHERE id = ?1"),
            params![id],
            from_row,
        )
        .optional()
        .map_err(AppError::db)
}

pub fn by_full_name(connection: &Connection, full_name: &str) -> Result<Option<Repo>, AppError> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM repos WHERE full_name = ?1"),
            params![full_name],
            from_row,
        )
        .optional()
        .map_err(AppError::db)
}

/// All repositories, monitored first, then alphabetically.
pub fn list(connection: &Connection) -> Result<Vec<Repo>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {COLUMNS} FROM repos ORDER BY monitored DESC, full_name ASC"
        ))
        .map_err(AppError::db)?;

    let rows = statement.query_map([], from_row).map_err(AppError::db)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AppError::db)
}

/// Repositories the poller should refresh.
pub fn list_monitored(connection: &Connection) -> Result<Vec<Repo>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {COLUMNS} FROM repos WHERE monitored = 1 ORDER BY full_name ASC"
        ))
        .map_err(AppError::db)?;

    let rows = statement.query_map([], from_row).map_err(AppError::db)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(AppError::db)
}

pub fn set_monitored(connection: &Connection, id: i64, monitored: bool) -> Result<bool, AppError> {
    let changed = connection
        .execute(
            "UPDATE repos SET monitored = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, monitored, Utc::now()],
        )
        .map_err(AppError::db)?;

    Ok(changed > 0)
}

pub fn touch_polled(connection: &Connection, id: i64) -> Result<(), AppError> {
    connection
        .execute(
            "UPDATE repos SET last_polled_at = ?2 WHERE id = ?1",
            params![id, Utc::now()],
        )
        .map_err(AppError::db)?;

    Ok(())
}

/// Conditional-request token for the runs endpoint, so a `304` costs no quota.
pub fn runs_etag(connection: &Connection, id: i64) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT runs_etag FROM repos WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map(Option::flatten)
        .map_err(AppError::db)
}

pub fn set_runs_etag(connection: &Connection, id: i64, etag: Option<&str>) -> Result<(), AppError> {
    connection
        .execute(
            "UPDATE repos SET runs_etag = ?2 WHERE id = ?1",
            params![id, etag],
        )
        .map_err(AppError::db)?;

    Ok(())
}

/// Removes a repository; runs, jobs, deployments and pull requests go with it
/// through `ON DELETE CASCADE`.
pub fn delete(connection: &Connection, id: i64) -> Result<bool, AppError> {
    let changed = connection
        .execute("DELETE FROM repos WHERE id = ?1", params![id])
        .map_err(AppError::db)?;

    Ok(changed > 0)
}

pub fn count(connection: &Connection) -> Result<i64, AppError> {
    connection
        .query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))
        .map_err(AppError::db)
}
