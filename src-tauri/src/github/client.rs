//! Shared HTTP plumbing for talking to GitHub.
//!
//! M2 only needs the OAuth and `/user` endpoints; M4 extends this module with
//! the REST surface, the `ETag` cache and the rate-limit budget. The client
//! builder and the endpoint set live here so both use one configuration.

use std::time::Duration;

use crate::error::AppError;

pub const API_VERSION: &str = "2022-11-28";

/// Sent on every request. GitHub rejects requests without a user agent.
pub const USER_AGENT: &str = concat!("Pulse/", env!("CARGO_PKG_VERSION"));

/// Origin set. Overridable so integration tests can point at a local mock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    /// `https://github.com` — device verification pages.
    pub web: String,
    /// `https://api.github.com` — the REST API.
    pub api: String,
    /// `https://github.com` — `/login/device/code` and `/login/oauth/access_token`.
    pub oauth: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            web: "https://github.com".to_owned(),
            api: "https://api.github.com".to_owned(),
            oauth: "https://github.com".to_owned(),
        }
    }
}

impl Endpoints {
    #[must_use]
    pub fn github() -> Self {
        Self::default()
    }

    /// Points all three origins at one base URL. Test-only.
    #[must_use]
    pub fn single_origin(origin: &str) -> Self {
        let origin = origin.trim_end_matches('/').to_owned();
        Self {
            web: origin.clone(),
            api: origin.clone(),
            oauth: origin,
        }
    }

    #[must_use]
    pub fn device_code_url(&self) -> String {
        format!("{}/login/device/code", self.oauth)
    }

    #[must_use]
    pub fn access_token_url(&self) -> String {
        format!("{}/login/oauth/access_token", self.oauth)
    }

    #[must_use]
    pub fn viewer_url(&self) -> String {
        format!("{}/user", self.api)
    }
}

/// Builds the HTTP client for ordinary API calls.
///
/// Redirects are disabled deliberately. GitHub hands out signed CDN URLs for
/// log archives; following a redirect with `reqwest`'s default policy risks
/// replaying the `Authorization` header to a third-party host. Every redirect
/// is therefore resolved by hand, and callers that need the body re-issue the
/// request without credentials.
pub fn build_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(10))
        .gzip(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| AppError::Internal(format!("cannot build the HTTP client: {error}")))
}

/// Builds a client for long-lived streams such as the relay's SSE connection.
///
/// [`build_client`] sets a twenty-second **total** timeout, which is right for
/// an API call and fatal for an event stream: the stream would be aborted every
/// twenty seconds and the relay would reconnect forever. A connect timeout is
/// kept so a dead host still fails fast.
pub fn build_streaming_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .gzip(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| AppError::Internal(format!("cannot build the streaming client: {error}")))
}

/// Maps a transport failure to an `AppError`.
///
/// Connect/timeout failures are retryable; a body that fails to decode is not,
/// because retrying will produce the same bytes.
pub fn transport_error(error: reqwest::Error) -> AppError {
    let retryable = error.is_timeout() || error.is_connect() || error.is_request();
    AppError::network(error, retryable)
}

/// Maps a non-success status to an `AppError`, consuming the response body for
/// diagnostics where it is safe to do so.
pub async fn status_error(response: reqwest::Response) -> AppError {
    let status = response.status();

    match status.as_u16() {
        401 => AppError::TokenRevoked,
        403 => AppError::Forbidden { scope: None },
        404 => AppError::NotFound,
        429 => AppError::SecondaryRateLimited {
            retry_after: Duration::from_secs(60),
        },
        _ => {
            let body = response.text().await.unwrap_or_default();
            let body = crate::redact::redact(body.trim());
            let detail = if body.is_empty() {
                String::new()
            } else {
                format!(": {}", body.chars().take(200).collect::<String>())
            };

            AppError::Internal(format!("GitHub returned {status}{detail}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_derive_the_oauth_urls() {
        let endpoints = Endpoints::github();
        assert_eq!(
            endpoints.device_code_url(),
            "https://github.com/login/device/code"
        );
        assert_eq!(
            endpoints.access_token_url(),
            "https://github.com/login/oauth/access_token"
        );
        assert_eq!(endpoints.viewer_url(), "https://api.github.com/user");
    }

    #[test]
    fn single_origin_tolerates_a_trailing_slash() {
        let endpoints = Endpoints::single_origin("http://127.0.0.1:8080/");
        assert_eq!(endpoints.web, "http://127.0.0.1:8080");
        assert_eq!(
            endpoints.device_code_url(),
            "http://127.0.0.1:8080/login/device/code"
        );
    }
}
