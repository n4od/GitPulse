//! Row types and the payloads used to write them.
//!
//! Read types (`Repo`, `WorkflowRun`, …) are what the UI and the poller see.
//! `*Upsert` types carry only the fields GitHub actually supplies, so a DAO can
//! never accidentally overwrite a locally-owned column such as `monitored`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub id: i64,
    pub github_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub is_private: bool,
    pub default_branch: Option<String>,
    pub avatar_url: Option<String>,
    pub monitored: bool,
    pub last_polled_at: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    pub id: i64,
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
    /// GitHub's own timestamp. Ordering the dashboard by this keeps the list
    /// stable when a run transitions, and makes it the natural retention key.
    pub updated_at: DateTime<Utc>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: i64,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub id: i64,
    pub repo_id: i64,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: i64,
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
