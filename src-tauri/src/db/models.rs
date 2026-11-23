//! Row types and the payloads used to write them.
//!
//! Read types (`Repo`, `WorkflowRun`, …) are what the UI and the poller see.
//! `*Upsert` types carry only the fields GitHub actually supplies, so a DAO can
//! never accidentally overwrite a locally-owned column such as `monitored`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Repo {
    // Ids arrive as JSON numbers over IPC, not as BigInt: the wire format is
    // serde/JSON, and every id is well inside 2^53.
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub github_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub is_private: bool,
    pub default_branch: Option<String>,
    pub avatar_url: Option<String>,
    pub monitored: bool,
    pub last_polled_at: Option<DateTime<Utc>>,
    /// The most recent poll failure for this repository, cleared on success.
    ///
    /// Distinct from the poller's global error: this one belongs to a single
    /// repository, and it is what a card's own error state reads.
    pub last_error: Option<String>,
    /// Stable code for that failure (`AppError::code`), so the UI can show a
    /// human title instead of parsing a message.
    pub last_error_code: Option<String>,
    /// When it happened. An error with no time cannot be acted on: a minute ago
    /// and a week ago call for different responses.
    pub last_error_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields refreshed from the GitHub API.
///
/// `monitored` and `created_at` are deliberately absent: they are local state
/// and must survive every sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoUpsert {
    pub github_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub is_private: bool,
    pub default_branch: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct WorkflowRun {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub repo_id: i64,
    #[ts(type = "number")]
    pub github_id: i64,
    pub workflow_name: Option<String>,
    pub display_title: Option<String>,
    pub event: Option<String>,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub status: String,
    pub conclusion: Option<String>,
    #[ts(type = "number | null")]
    pub run_number: Option<i64>,
    #[ts(type = "number | null")]
    pub run_attempt: Option<i64>,
    pub html_url: Option<String>,
    pub actor_login: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub run_started_at: Option<DateTime<Utc>>,
    /// GitHub's own timestamp. Ordering the dashboard by this keeps the list
    /// stable when a run transitions, and makes it the natural retention key.
    pub updated_at: DateTime<Utc>,
}

/// Everything a repository card needs, in one payload.
///
/// The card is a projection, not a table: `failing_runs` and `has_active_run`
/// are derived from the runs we hold, and shipping them together means the UI
/// never has to fetch twice to render one card — or to update one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct RepoCard {
    pub repo: Repo,
    /// Newest first, capped. The card shows a sparkline and the dashboard's run
    /// list reads from here, so shipping them together avoids a second fetch.
    pub recent_runs: Vec<WorkflowRun>,
    /// How many of this repository's workflows last ended in failure.
    ///
    /// The latest *completed* run of each workflow, not every failure the cache
    /// holds: a run that failed last month and has succeeded since is not a
    /// reason to paint the repository red. This is the same number the tray, the
    /// taskbar badge and the dashboard filter all read, so they cannot disagree.
    #[ts(type = "number")]
    pub failing_runs: i64,
    /// The run to badge, when something is failing; `None` when nothing is.
    ///
    /// The badge must describe the same thing [`failing_runs`] counts. It used
    /// to describe the newest run of *any* workflow, so a repository could show
    /// "Success" next to "1 failing run" - and disagree with the tray. Picking
    /// the run here, from the failing workflow set, is what makes the badge, the
    /// line under it and the count one answer.
    ///
    /// [`failing_runs`]: RepoCard::failing_runs
    pub failing_run: Option<WorkflowRun>,
    pub has_active_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunUpsert {
    pub repo_id: i64,
    pub github_id: i64,
    pub workflow_name: Option<String>,
    pub display_title: Option<String>,
    pub event: Option<String>,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub status: String,
    pub conclusion: Option<String>,
    pub run_number: Option<i64>,
    pub run_attempt: Option<i64>,
    pub html_url: Option<String>,
    pub actor_login: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub run_started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Job {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub run_id: i64,
    #[ts(type = "number")]
    pub github_id: i64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobUpsert {
    pub run_id: i64,
    pub github_id: i64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Deployment {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub repo_id: i64,
    #[ts(type = "number")]
    pub github_id: i64,
    pub environment: String,
    /// Serialised as `ref` — `ref` is a Rust keyword.
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub sha: Option<String>,
    pub state: Option<String>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentUpsert {
    pub repo_id: i64,
    pub github_id: i64,
    pub environment: String,
    pub git_ref: Option<String>,
    pub sha: Option<String>,
    pub state: Option<String>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct PullRequest {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub repo_id: i64,
    #[ts(type = "number")]
    pub github_id: i64,
    #[ts(type = "number")]
    pub number: i64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub author: Option<String>,
    pub review_state: Option<String>,
    pub checks_state: Option<String>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestUpsert {
    pub repo_id: i64,
    pub github_id: i64,
    pub number: i64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub author: Option<String>,
    pub review_state: Option<String>,
    pub checks_state: Option<String>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}
