//! Test fixtures for the Rust side.

use chrono::Utc;

use crate::db::{Repo, RepoCard, WorkflowRun};

#[must_use]
pub fn make_repo() -> Repo {
    Repo {
        id: 1,
        github_id: 100,
        owner: "acme".to_owned(),
        name: "api".to_owned(),
        full_name: "acme/api".to_owned(),
        is_private: false,
        default_branch: Some("main".to_owned()),
        avatar_url: None,
        monitored: true,
        last_polled_at: None,
        last_error: None,
        last_error_code: None,
        last_error_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[must_use]
pub fn make_card(repo: Repo, failing_runs: i64, has_active_run: bool) -> RepoCard {
    RepoCard {
        repo,
        recent_runs: vec![WorkflowRun {
            id: 1,
            repo_id: 1,
            github_id: 500,
            workflow_name: Some("CI".to_owned()),
            display_title: None,
            event: Some("push".to_owned()),
            branch: Some("main".to_owned()),
            head_sha: None,
            status: if has_active_run {
                "in_progress"
            } else {
                "completed"
            }
            .to_owned(),
            conclusion: if has_active_run {
                None
            } else {
                Some("success".to_owned())
            },
            run_number: Some(1),
            run_attempt: Some(1),
            html_url: None,
            actor_login: None,
            created_at: None,
            run_started_at: None,
            updated_at: Utc::now(),
        }],
        failing_runs,
        has_active_run,
    }
}
