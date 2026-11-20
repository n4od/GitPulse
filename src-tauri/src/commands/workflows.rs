//! Workflow listing and dispatch.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::db::{self, Db};
use crate::error::AppError;
use crate::github::GithubClient;
use crate::poller::Poller;

/// A workflow definition, as the dispatch dialog needs it.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct WorkflowSummary {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub path: String,
    /// `active`, `disabled_manually`, … — only active workflows can be dispatched.
    pub state: String,
}

/// Workflows defined in a repository.
///
/// GitHub cannot tell us which ones declare `workflow_dispatch` without
/// fetching each file, so the dialog offers them all and surfaces GitHub's
/// error if the chosen one cannot be triggered.
#[tauri::command]
pub async fn workflows_list(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    repo_id: i64,
) -> Result<Vec<WorkflowSummary>, AppError> {
    if repo_id <= 0 {
        return Err(AppError::Config(
            "repository ids must be positive".to_owned(),
        ));
    }

    let db = db.inner().clone();
    let client = Arc::clone(client.inner());

    let repository = db
        .with_connection(move |connection| db::repos::by_id(connection, repo_id))
        .await?
        .ok_or(AppError::NotFound)?;

    let workflows = client.workflows(&repository.full_name).await?;

    Ok(workflows
        .into_iter()
        .map(|workflow| WorkflowSummary {
            id: workflow.id,
            name: workflow.name,
            path: workflow.path,
            state: workflow.state,
        })
        .collect())
}

/// Triggers a `workflow_dispatch` run.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn workflows_dispatch(
    db: State<'_, Db>,
    client: State<'_, Arc<GithubClient>>,
    poller: State<'_, Arc<Poller>>,
    repo_id: i64,
    workflow_id: i64,
    git_ref: String,
    inputs: Option<BTreeMap<String, String>>,
) -> Result<bool, AppError> {
    if repo_id <= 0 || workflow_id <= 0 {
        return Err(AppError::Config(
            "repository and workflow ids must be positive".to_owned(),
        ));
    }

    let db = db.inner().clone();
    let client = Arc::clone(client.inner());

    let repository = db
        .with_connection(move |connection| db::repos::by_id(connection, repo_id))
        .await?
        .ok_or(AppError::NotFound)?;

    let inputs = inputs
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| (key, serde_json::Value::String(value)))
        .collect::<serde_json::Map<_, _>>();

    client
        .dispatch(&repository.full_name, workflow_id, &git_ref, &inputs)
        .await?;

    tracing::info!(
        repo = %repository.full_name,
        workflow = workflow_id,
        %git_ref,
        "workflow dispatch requested"
    );

    poller.request_refresh(repository.id);
    Ok(true)
}
