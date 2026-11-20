//! Run, job and log commands.

use std::sync::Arc;

use tauri::State;

use crate::db::{self, Db, Job, WorkflowRun};
use crate::error::AppError;
use crate::github::{GithubClient, LogDownload};
use crate::poller::Poller;

/// Default page size for the run list.
const DEFAULT_PAGE: i64 = 50;
const MAX_PAGE: i64 = 200;

fn validate_id(value: i64, what: &str) -> Result<i64, AppError> {
    if value <= 0 {
        return Err(AppError::Config(format!("{what} must be positive")));
    }

    Ok(value)
}

/// A page of runs for a repository, newest first.
///
/// Offset paging rather than a keyset cursor: the data is local and bounded by
/// retention, so there is no large table to seek through.
#[tauri::command]
pub async fn runs_list(
    db: State<'_, Db>,
    repo_id: i64,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<WorkflowRun>, AppError> {
    let repo_id = validate_id(repo_id, "repository ids")?;
    let offset = offset.unwrap_or(0).max(0);
    let limit = limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);

    let db = db.inner().clone();
    db.with_connection(move |connection| db::runs::list_by_repo(connection, repo_id, limit, offset))
        .await
}

/// Jobs for a run, refreshed from GitHub.
///
/// Fetched on demand rather than with every poll: jobs are only interesting
/// when somebody is looking at a run.
#[tauri::command]
pub async fn runs_jobs(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    run_id: i64,
) -> Result<Vec<Job>, AppError> {
    let run_id = validate_id(run_id, "run ids")?;
    let db = db.inner().clone();
    let client = Arc::clone(client.inner());

    let context = db
        .with_connection(move |connection| db::runs::context(connection, run_id))
        .await?
        .ok_or(AppError::NotFound)?;

    let fetched = client
        .jobs(&context.full_name, context.github_run_id)
        .await?;

    let now = chrono::Utc::now();
    let upserts: Vec<db::JobUpsert> = fetched
        .iter()
        .map(|job| job.to_upsert(run_id, now))
        .collect();

    db.with_connection(move |connection| db::jobs::replace(connection, run_id, &upserts))
        .await
}

/// One job's log, capped at 5 MB and fetched on demand.
///
/// Job logs, not the run archive: the run endpoint returns a ZIP.
#[tauri::command]
pub async fn job_log(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    job_id: i64,
) -> Result<LogDownload, AppError> {
    let job_id = validate_id(job_id, "job ids")?;
    let db = db.inner().clone();
    let client = Arc::clone(client.inner());

    let context = db
        .with_connection(move |connection| db::jobs::context(connection, job_id))
        .await?
        .ok_or(AppError::NotFound)?;

    let url = client
        .job_log_url(&context.full_name, context.github_job_id)
        .ok_or_else(|| AppError::Internal("the job's repository name is not usable".to_owned()))?;

    client.download_log(&url).await
}

#[derive(Debug, Clone, Copy)]
enum RunAction {
    RerunFailed,
    Rerun,
    Cancel,
}

/// Runs an action against GitHub and asks the poller to refresh its repository
/// so the change shows up promptly instead of at the next scheduled poll.
async fn apply_run_action(
    db: Db,
    client: Arc<GithubClient>,
    poller: &Arc<Poller>,
    run_id: i64,
    action: RunAction,
) -> Result<i64, AppError> {
    let context = db
        .with_connection(move |connection| db::runs::context(connection, run_id))
        .await?
        .ok_or(AppError::NotFound)?;

    match action {
        RunAction::RerunFailed => {
            client
                .rerun_failed(&context.full_name, context.github_run_id)
                .await?;
        }
        RunAction::Rerun => {
            client
                .rerun(&context.full_name, context.github_run_id)
                .await?;
        }
        RunAction::Cancel => {
            client
                .cancel_run(&context.full_name, context.github_run_id)
                .await?;
        }
    }

    tracing::info!(
        repo = %context.full_name,
        run = context.github_run_id,
        "run action dispatched"
    );

    poller.request_refresh(context.repo_id);
    Ok(context.repo_id)
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn runs_rerun_failed(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    poller: State<'_, Arc<Poller>>,
    run_id: i64,
) -> Result<bool, AppError> {
    let run_id = validate_id(run_id, "run ids")?;
    apply_run_action(
        db.inner().clone(),
        Arc::clone(client.inner()),
        poller.inner(),
        run_id,
        RunAction::RerunFailed,
    )
    .await?;

    Ok(true)
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn runs_rerun(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    poller: State<'_, Arc<Poller>>,
    run_id: i64,
) -> Result<bool, AppError> {
    let run_id = validate_id(run_id, "run ids")?;
    apply_run_action(
        db.inner().clone(),
        Arc::clone(client.inner()),
        poller.inner(),
        run_id,
        RunAction::Rerun,
    )
    .await?;

    Ok(true)
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn runs_cancel(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    poller: State<'_, Arc<Poller>>,
    run_id: i64,
) -> Result<bool, AppError> {
    let run_id = validate_id(run_id, "run ids")?;
    apply_run_action(
        db.inner().clone(),
        Arc::clone(client.inner()),
        poller.inner(),
        run_id,
        RunAction::Cancel,
    )
    .await?;

    Ok(true)
}
