//! Repository commands for the dashboard.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::db::{self, Db, RepoCard};
use crate::error::AppError;
use crate::github::GithubClient;
use crate::poller::Poller;

/// A repository GitHub offers that Pulse may or may not already watch.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct AvailableRepo {
    #[ts(type = "number")]
    pub github_id: i64,
    pub full_name: String,
    pub is_private: bool,
    pub avatar_url: Option<String>,
    pub tracked: bool,
}

/// Every card the dashboard renders.
///
/// Search and filtering happen in the webview: 250 cards is well inside what a
/// keystroke can filter, and round-tripping to Rust per character would make
/// search feel worse than the data warrants.
#[tauri::command]
pub async fn repos_list(db: State<'_, Db>) -> Result<Vec<RepoCard>, AppError> {
    let db = db.inner().clone();
    db.with_connection(db::repos::cards).await
}

/// Repositories the signed-in user can access, annotated with whether Pulse
/// already watches them.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn repos_available(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    page: Option<u32>,
) -> Result<Vec<AvailableRepo>, AppError> {
    let db = db.inner().clone();
    let client = Arc::clone(client.inner());

    let repositories = client.repos(page.unwrap_or(1).max(1), 100).await?;

    let tracked: std::collections::HashSet<String> = db
        .with_connection(|connection| {
            Ok(db::repos::list(connection)?
                .into_iter()
                .map(|repo| repo.full_name)
                .collect())
        })
        .await?;

    Ok(repositories
        .into_iter()
        .map(|repo| AvailableRepo {
            github_id: repo.id,
            is_private: repo.private,
            avatar_url: repo.owner.avatar_url.clone(),
            tracked: tracked.contains(&repo.full_name),
            full_name: repo.full_name,
        })
        .collect())
}

/// Starts watching a repository and asks for an immediate first poll.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn repos_track(
    full_name: String,
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    poller: State<'_, Arc<Poller>>,
) -> Result<i64, AppError> {
    let db = db.inner().clone();
    let client = Arc::clone(client.inner());

    // Re-fetch rather than trusting the caller's copy: `full_name` came from
    // the webview, and the row we store should be whatever GitHub says it is.
    let repository = client.repo(&full_name).await?;
    let upsert: db::RepoUpsert = (&repository).into();
    let label = upsert.full_name.clone();

    let id = db
        .with_connection(move |connection| db::repos::upsert(connection, &upsert))
        .await?;

    db.with_connection(move |connection| db::repos::set_monitored(connection, id, true))
        .await?;

    poller.request_refresh(id);
    tracing::info!(repo = %label, id, "tracking repository");

    Ok(id)
}

/// Mutes or unmutes a repository without discarding its cached history.
#[tauri::command]
pub async fn repos_set_monitored(
    repo_id: i64,
    monitored: bool,
    db: State<'_, Db>,
) -> Result<bool, AppError> {
    if repo_id <= 0 {
        return Err(AppError::Config(
            "repository ids must be positive".to_owned(),
        ));
    }

    let db = db.inner().clone();
    db.with_connection(move |connection| db::repos::set_monitored(connection, repo_id, monitored))
        .await
}
