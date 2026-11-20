//! Diagnostics commands.

use std::sync::Arc;

use tauri::State;

use crate::db::Db;
use crate::error::AppError;
use crate::github::GithubClient;
use crate::poller::Poller;
use crate::state::AppState;

use super::metrics::{self, SystemMetrics};

#[tauri::command]
pub async fn system_metrics(
    db: State<'_, Db>,
    state: State<'_, AppState>,
    client: State<'_, Arc<GithubClient>>,
) -> Result<SystemMetrics, AppError> {
    let database = db.inner().clone();
    let budget_available = client.budget().available();

    metrics::collect(&database, state.started_at(), budget_available).await
}

/// Drops every cached run, job, deployment and pull request.
///
/// The repositories stay: they are the user's choices, not cached data. A
/// refresh repopulates everything else within a minute or two.
#[tauri::command]
pub async fn system_clear_cache(
    db: State<'_, Db>,
    poller: State<'_, Arc<Poller>>,
) -> Result<bool, AppError> {
    let database = db.inner().clone();

    database
        .with_connection(crate::db::deliveries::clear_cache)
        .await?;

    // Ask for everything back immediately rather than at the next interval.
    if let Ok(repos) = database
        .with_connection(crate::db::repos::list_monitored)
        .await
    {
        for repo in repos {
            poller.request_refresh(repo.id);
        }
    }

    tracing::info!("local cache cleared");
    Ok(true)
}
