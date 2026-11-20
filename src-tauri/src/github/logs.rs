//! Workflow log download.
//!
//! GitHub answers the logs endpoint with a redirect to a **pre-signed** CDN
//! URL. Two things matter:
//!
//! 1. The redirect must be resolved by hand, because the CDN is a different
//!    host and the `Authorization` header must not travel with it.
//! 2. The body must be streamed against a cap. A log archive can be tens of
//!    megabytes; buffering it whole to find out is how a memory budget dies.

use serde::Serialize;

use crate::error::AppError;

use super::api::GithubClient;
use super::client::{status_error, transport_error, API_VERSION, USER_AGENT};

/// Cap for a single log download, before truncation.
pub const LOG_CAP_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct LogDownload {
    pub text: String,
    /// True when the body was longer than the cap and got cut.
    pub truncated: bool,
}

impl GithubClient {
    /// Archive of every log in a run. A ZIP, which is why the UI reads job logs
    /// instead — those come back as text.
    #[must_use]
    pub fn run_log_url(&self, full_name: &str, run_id: i64) -> Option<String> {
        let full_name = super::endpoints::validate_full_name(full_name).ok()?;
        Some(self.api_url(&format!("/repos/{full_name}/actions/runs/{run_id}/logs")))
    }

    /// One job's log, as plain text.
    #[must_use]
    pub fn job_log_url(&self, full_name: &str, job_id: i64) -> Option<String> {
        let full_name = super::endpoints::validate_full_name(full_name).ok()?;
        Some(self.api_url(&format!("/repos/{full_name}/actions/jobs/{job_id}/logs")))
    }

    pub async fn download_log(&self, url: &str) -> Result<LogDownload, AppError> {
        self.download_log_capped(url, LOG_CAP_BYTES).await
    }

    /// Same as [`Self::download_log`] with an explicit cap, so the truncation
    /// path can be tested without a 5 MB fixture.
    pub async fn download_log_capped(
        &self,
        url: &str,
        cap: usize,
    ) -> Result<LogDownload, AppError> {
        self.budget().acquire().await?;
        let token = self.token_provider().access_token().await?;

        let response = self
            .http()
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header("X-GitHub-Api-Version", API_VERSION)
            .bearer_auth(secrecy::ExposeSecret::expose_secret(&token))
            .send()
            .await
            .map_err(transport_error)?;

        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    AppError::Internal("the log redirect carried no Location header".to_owned())
                })?;

            let target = resolve(url, location)?;

            // Deliberately no credential: the CDN URL is pre-signed and belongs
            // to a different origin. Sending the token here would leak it.
            let cdn = self
                .http()
                .get(&target)
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .send()
                .await
                .map_err(transport_error)?;

            if !cdn.status().is_success() {
                return Err(status_error(cdn).await);
            }

            return read_capped(cdn, cap).await;
        }

        if !response.status().is_success() {
            return Err(status_error(response).await);
        }

        read_capped(response, cap).await
    }
}

fn resolve(base: &str, location: &str) -> Result<String, AppError> {
    if location.starts_with("https://") || location.starts_with("http://") {
        return Ok(location.to_owned());
    }

    let base = url::Url::parse(base)
        .map_err(|error| AppError::Config(format!("invalid log URL: {error}")))?;

    base.join(location)
        .map(|url| url.to_string())
        .map_err(|error| AppError::Config(format!("invalid redirect target: {error}")))
}

async fn read_capped(mut response: reqwest::Response, cap: usize) -> Result<LogDownload, AppError> {
    let mut buffer: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut truncated = false;

    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        let remaining = cap.saturating_sub(buffer.len());

        if chunk.len() > remaining {
            buffer.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }

        buffer.extend_from_slice(&chunk);
    }

    Ok(LogDownload {
        // Logs are usually UTF-8 but GitHub has served odd bytes before;
        // lossy conversion keeps the rest readable instead of failing.
        text: String::from_utf8_lossy(&buffer).into_owned(),
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_redirects_are_kept_as_is() {
        let target = resolve(
            "https://api.github.com/repos/acme/api/actions/runs/1/logs",
            "https://objects.githubusercontent.com/signed?token=abc",
        )
        .expect("absolute");

        assert_eq!(
            target,
            "https://objects.githubusercontent.com/signed?token=abc"
        );
    }

    #[test]
    fn relative_redirects_are_resolved_against_the_origin() {
        let target = resolve("https://api.github.com/a/b/c", "/elsewhere/log").expect("relative");
        assert_eq!(target, "https://api.github.com/elsewhere/log");
    }
}
