//! Typed REST endpoints.
//!
//! Every method validates its inputs *before* building a URL. Repository names
//! and git refs end up in a path, so an unvalidated `..` or `/` would let a
//! caller address a different endpoint than intended.

use serde_json::Value;

use crate::error::AppError;

use super::api::GithubClient;
use super::models::{
    ApiDeployment, ApiDeploymentStatus, ApiJob, ApiJobsPage, ApiPull, ApiRateCore, ApiRateLimit,
    ApiRepo, ApiRun, ApiRunsPage, ApiWorkflow, ApiWorkflowsPage,
};

/// GitHub username/organisation and repository name segments.
///
/// Rejects anything that is not a plain segment, plus the two path-traversal
/// spellings that survive a character-class check.
pub fn validate_full_name(full_name: &str) -> Result<&str, AppError> {
    let mut parts = full_name.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();

    let segment_ok = |segment: &str| {
        !segment.is_empty()
            && segment.len() <= 100
            && segment != "."
            && segment != ".."
            && segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };

    if parts.next().is_some() || !segment_ok(owner) || !segment_ok(name) {
        return Err(AppError::Config(format!(
            "invalid repository name: {full_name:?}"
        )));
    }

    Ok(full_name)
}

/// A git ref goes into a JSON body for dispatch, but it is also echoed into
/// logs and error messages, so it gets the same treatment.
pub fn validate_git_ref(git_ref: &str) -> Result<&str, AppError> {
    let ok = !git_ref.is_empty()
        && git_ref.len() <= 255
        && !git_ref.starts_with('/')
        && !git_ref.ends_with('/')
        && !git_ref.contains("..")
        && git_ref
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'));

    if ok {
        Ok(git_ref)
    } else {
        Err(AppError::Config(format!("invalid git ref: {git_ref:?}")))
    }
}

impl GithubClient {
    /// Repositories the user can see, most recently pushed first.
    pub async fn repos(&self, page: u32, per_page: u32) -> Result<Vec<ApiRepo>, AppError> {
        self.get_json(&format!(
            "/user/repos?affiliation=owner,collaborator,organization_member&sort=pushed&per_page={per_page}&page={page}"
        ))
        .await
    }

    pub async fn repo(&self, full_name: &str) -> Result<ApiRepo, AppError> {
        let full_name = validate_full_name(full_name)?;
        self.get_json(&format!("/repos/{full_name}")).await
    }

    /// The most recent runs for a repository. `per_page` bounds the work GitHub
    /// does on our behalf, so keep it small — the poller only needs the head.
    pub async fn runs(&self, full_name: &str, per_page: u32) -> Result<Vec<ApiRun>, AppError> {
        let full_name = validate_full_name(full_name)?;
        let page: ApiRunsPage = self
            .get_json(&format!(
                "/repos/{full_name}/actions/runs?per_page={per_page}"
            ))
            .await?;

        Ok(page.workflow_runs)
    }

    pub async fn run(&self, full_name: &str, run_id: i64) -> Result<ApiRun, AppError> {
        let full_name = validate_full_name(full_name)?;
        self.get_json(&format!("/repos/{full_name}/actions/runs/{run_id}"))
            .await
    }

    pub async fn jobs(&self, full_name: &str, run_id: i64) -> Result<Vec<ApiJob>, AppError> {
        let full_name = validate_full_name(full_name)?;
        let page: ApiJobsPage = self
            .get_json(&format!(
                "/repos/{full_name}/actions/runs/{run_id}/jobs?per_page=100"
            ))
            .await?;

        Ok(page.jobs)
    }

    pub async fn workflows(&self, full_name: &str) -> Result<Vec<ApiWorkflow>, AppError> {
        let full_name = validate_full_name(full_name)?;
        let page: ApiWorkflowsPage = self
            .get_json(&format!(
                "/repos/{full_name}/actions/workflows?per_page=100"
            ))
            .await?;

        Ok(page.workflows)
    }

    /// Triggers a `workflow_dispatch` run. The workflow must declare the
    /// trigger and exist on the default branch, or GitHub returns 404/422.
    pub async fn dispatch(
        &self,
        full_name: &str,
        workflow_id: i64,
        git_ref: &str,
        inputs: &serde_json::Map<String, Value>,
    ) -> Result<(), AppError> {
        let full_name = validate_full_name(full_name)?;
        let git_ref = validate_git_ref(git_ref)?;

        self.post(
            &format!("/repos/{full_name}/actions/workflows/{workflow_id}/dispatches"),
            Some(serde_json::json!({ "ref": git_ref, "inputs": inputs })),
        )
        .await
    }

    pub async fn rerun_failed(&self, full_name: &str, run_id: i64) -> Result<(), AppError> {
        let full_name = validate_full_name(full_name)?;
        self.post(
            &format!("/repos/{full_name}/actions/runs/{run_id}/rerun-failed-jobs"),
            None,
        )
        .await
    }

    pub async fn rerun(&self, full_name: &str, run_id: i64) -> Result<(), AppError> {
        let full_name = validate_full_name(full_name)?;
        self.post(
            &format!("/repos/{full_name}/actions/runs/{run_id}/rerun"),
            None,
        )
        .await
    }

    pub async fn cancel_run(&self, full_name: &str, run_id: i64) -> Result<(), AppError> {
        let full_name = validate_full_name(full_name)?;
        self.post(
            &format!("/repos/{full_name}/actions/runs/{run_id}/cancel"),
            None,
        )
        .await
    }

    pub async fn deployments(
        &self,
        full_name: &str,
        per_page: u32,
    ) -> Result<Vec<ApiDeployment>, AppError> {
        let full_name = validate_full_name(full_name)?;
        self.get_json(&format!(
            "/repos/{full_name}/deployments?per_page={per_page}"
        ))
        .await
    }

    pub async fn deployment_statuses(
        &self,
        full_name: &str,
        deployment_id: i64,
    ) -> Result<Vec<ApiDeploymentStatus>, AppError> {
        let full_name = validate_full_name(full_name)?;
        self.get_json(&format!(
            "/repos/{full_name}/deployments/{deployment_id}/statuses?per_page=10"
        ))
        .await
    }

    pub async fn pulls(&self, full_name: &str) -> Result<Vec<ApiPull>, AppError> {
        let full_name = validate_full_name(full_name)?;
        self.get_json(&format!("/repos/{full_name}/pulls?state=open&per_page=50"))
            .await
    }

    /// The authoritative budget. The local bucket is a guard, this is truth.
    pub async fn rate_limit(&self) -> Result<ApiRateCore, AppError> {
        let page: ApiRateLimit = self.get_json("/rate_limit").await?;
        Ok(page.resources.core)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_repository_names() {
        for name in ["acme/api", "acme/api.service", "a/b", ".github/workflows"] {
            assert!(validate_full_name(name).is_ok(), "{name} should be valid");
        }
    }

    #[test]
    fn rejects_path_traversal_and_degenerate_names() {
        for name in [
            "../../admin",
            "acme/api/extra",
            "acme/",
            "/api",
            "",
            "acme/..",
            "acme/./api",
            "acme/api name",
            "acme/api?x=1",
        ] {
            assert_eq!(
                validate_full_name(name)
                    .expect_err("must be rejected")
                    .code(),
                "config",
                "{name:?} should be rejected"
            );
        }
    }

    #[test]
    fn accepts_normal_git_refs() {
        for reference in ["main", "release/1.2", "feature_thing", "v1.0.0"] {
            assert!(
                validate_git_ref(reference).is_ok(),
                "{reference} should be valid"
            );
        }
    }

    #[test]
    fn rejects_dangerous_git_refs() {
        for reference in ["", "/main", "main/", "feature/../main", "main;rm -rf"] {
            assert_eq!(
                validate_git_ref(reference)
                    .expect_err("must be rejected")
                    .code(),
                "config",
                "{reference:?} should be rejected"
            );
        }
    }
}
