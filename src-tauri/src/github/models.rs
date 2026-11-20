//! Wire models for the GitHub REST API, and the mapping into our cache rows.
//!
//! These are intentionally narrower than GitHub's schema: only the fields the
//! UI renders are declared, and everything that optional in the API is
//! `Option` here. A missing field must never fail a whole poll.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::db::models::{DeploymentUpsert, JobUpsert, PullRequestUpsert, RepoUpsert, RunUpsert};

#[derive(Debug, Clone, Deserialize)]
pub struct ApiActor {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiOwner {
    pub login: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: ApiOwner,
    #[serde(default)]
    pub private: bool,
    pub default_branch: Option<String>,
}

impl From<&ApiRepo> for RepoUpsert {
    fn from(repo: &ApiRepo) -> Self {
        Self {
            github_id: repo.id,
            owner: repo.owner.login.clone(),
            name: repo.name.clone(),
            full_name: repo.full_name.clone(),
            is_private: repo.private,
            default_branch: repo.default_branch.clone(),
            avatar_url: repo.owner.avatar_url.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRun {
    pub id: i64,
    /// The workflow name, e.g. "CI".
    pub name: Option<String>,
    pub display_title: Option<String>,
    pub event: Option<String>,
    pub head_branch: Option<String>,
    pub head_sha: Option<String>,
    pub status: String,
    pub conclusion: Option<String>,
    pub run_number: Option<i64>,
    pub run_attempt: Option<i64>,
    pub html_url: Option<String>,
    pub actor: Option<ApiActor>,
    pub created_at: Option<DateTime<Utc>>,
    pub run_started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl ApiRun {
    #[must_use]
    pub fn to_upsert(&self, repo_id: i64) -> RunUpsert {
        RunUpsert {
            repo_id,
            github_id: self.id,
            workflow_name: self.name.clone(),
            display_title: self.display_title.clone(),
            event: self.event.clone(),
            branch: self.head_branch.clone(),
            head_sha: self.head_sha.clone(),
            status: self.status.clone(),
            conclusion: self.conclusion.clone(),
            run_number: self.run_number,
            run_attempt: self.run_attempt,
            html_url: self.html_url.clone(),
            actor_login: self.actor.as_ref().map(|actor| actor.login.clone()),
            created_at: self.created_at,
            run_started_at: self.run_started_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRunsPage {
    pub total_count: Option<i64>,
    pub workflow_runs: Vec<ApiRun>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiJob {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub html_url: Option<String>,
}

impl ApiJob {
    #[must_use]
    pub fn to_upsert(&self, run_id: i64, updated_at: DateTime<Utc>) -> JobUpsert {
        JobUpsert {
            run_id,
            github_id: self.id,
            name: self.name.clone(),
            status: self.status.clone(),
            conclusion: self.conclusion.clone(),
            started_at: self.started_at,
            completed_at: self.completed_at,
            html_url: self.html_url.clone(),
            updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiJobsPage {
    pub jobs: Vec<ApiJob>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiWorkflow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiWorkflowsPage {
    pub workflows: Vec<ApiWorkflow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiDeployment {
    pub id: i64,
    pub environment: String,
    /// `ref` is a Rust keyword.
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub sha: Option<String>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl ApiDeployment {
    /// `state` is not on the deployment object itself — it lives on the
    /// deployment's statuses, which are fetched separately.
    #[must_use]
    pub fn to_upsert(&self, repo_id: i64, state: Option<String>) -> DeploymentUpsert {
        DeploymentUpsert {
            repo_id,
            github_id: self.id,
            environment: self.environment.clone(),
            git_ref: self.git_ref.clone(),
            sha: self.sha.clone(),
            state,
            html_url: self.html_url.clone(),
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiDeploymentStatus {
    pub state: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiPull {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub state: String,
    #[serde(default)]
    pub draft: bool,
    pub user: Option<ApiActor>,
    pub html_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl ApiPull {
    #[must_use]
    pub fn to_upsert(
        &self,
        repo_id: i64,
        review_state: Option<String>,
        checks_state: Option<String>,
    ) -> PullRequestUpsert {
        PullRequestUpsert {
            repo_id,
            github_id: self.id,
            number: self.number,
            title: self.title.clone(),
            state: self.state.clone(),
            is_draft: self.draft,
            author: self.user.as_ref().map(|user| user.login.clone()),
            review_state,
            checks_state,
            html_url: self.html_url.clone(),
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRateLimit {
    pub resources: ApiRateResources,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRateResources {
    pub core: ApiRateCore,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRateCore {
    pub limit: i64,
    pub remaining: i64,
    pub reset: i64,
    pub used: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_maps_into_a_cache_upsert() {
        let run: ApiRun = serde_json::from_value(serde_json::json!({
            "id": 42,
            "name": "CI",
            "display_title": "Fix the thing",
            "event": "push",
            "head_branch": "main",
            "head_sha": "a1b2c3d",
            "status": "completed",
            "conclusion": "failure",
            "run_number": 7,
            "run_attempt": 2,
            "html_url": "https://github.com/acme/api/actions/runs/42",
            "actor": { "login": "octocat" },
            "created_at": "2026-11-20T10:00:00Z",
            "run_started_at": "2026-11-20T10:00:05Z",
            "updated_at": "2026-11-20T10:04:00Z"
        }))
        .expect("valid run");

        let upsert = run.to_upsert(1);

        assert_eq!(upsert.repo_id, 1);
        assert_eq!(upsert.github_id, 42);
        assert_eq!(upsert.workflow_name.as_deref(), Some("CI"));
        assert_eq!(upsert.branch.as_deref(), Some("main"));
        assert_eq!(upsert.run_attempt, Some(2));
        assert_eq!(upsert.actor_login.as_deref(), Some("octocat"));
        assert_eq!(upsert.conclusion.as_deref(), Some("failure"));
    }

    #[test]
    fn a_run_tolerates_missing_optional_fields() {
        let run: ApiRun = serde_json::from_value(serde_json::json!({
            "id": 1,
            "status": "queued",
            "updated_at": "2026-11-20T10:00:00Z"
        }))
        .expect("minimal run is still valid");

        let upsert = run.to_upsert(3);
        assert_eq!(upsert.status, "queued");
        assert!(upsert.conclusion.is_none());
        assert!(upsert.workflow_name.is_none());
        assert!(upsert.actor_login.is_none());
    }

    #[test]
    fn a_repo_maps_its_owner_into_the_upsert() {
        let repo: ApiRepo = serde_json::from_value(serde_json::json!({
            "id": 9,
            "name": "api",
            "full_name": "acme/api",
            "owner": { "login": "acme", "avatar_url": "https://avatars.githubusercontent.com/u/1" },
            "private": true,
            "default_branch": "main"
        }))
        .expect("valid repo");

        let upsert: RepoUpsert = (&repo).into();
        assert_eq!(upsert.owner, "acme");
        assert_eq!(upsert.full_name, "acme/api");
        assert!(upsert.is_private);
        assert!(upsert.avatar_url.is_some());
    }

    #[test]
    fn a_deployment_reads_the_ref_keyword_field() {
        let deployment: ApiDeployment = serde_json::from_value(serde_json::json!({
            "id": 5,
            "environment": "production",
            "ref": "main",
            "sha": "a1b2c3d",
            "updated_at": "2026-11-20T10:00:00Z"
        }))
        .expect("valid deployment");

        assert_eq!(deployment.git_ref.as_deref(), Some("main"));

        let upsert = deployment.to_upsert(2, Some("success".to_owned()));
        assert_eq!(upsert.environment, "production");
        assert_eq!(upsert.state.as_deref(), Some("success"));
    }
}
