//! GitHub OAuth 2.0 device flow and token exchange.
//!
//! The device flow exists so no client secret has to ship in a desktop binary:
//! the app asks for a device code, the user approves it in their browser, and
//! the app polls until GitHub hands over a token.
//!
//! The same endpoints serve an OAuth App and a GitHub App. What comes back
//! differs: only a GitHub App with expiring tokens enabled returns
//! `refresh_token` and `expires_in`. Nothing here assumes either shape, so both
//! configurations work without a code fork.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::AppError;
use crate::github::client::{status_error, transport_error, Endpoints, API_VERSION, USER_AGENT};

/// The grant type GitHub expects when polling for a device token.
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// GitHub's documented default polling interval, used when it omits one.
const DEFAULT_INTERVAL_SECS: u64 = 5;

/// How much sooner than the real expiry we treat a token as stale. Refreshing at
/// the boundary would race the request we are about to make with it.
pub const EXPIRY_SKEW_SECS: i64 = 600;

#[derive(Debug, Clone)]
pub struct DeviceCodeGrant {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: DateTime<Utc>,
    pub interval: Duration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

impl TokenResponse {
    /// True when the token can be renewed without user interaction.
    #[must_use]
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }
}

#[derive(Debug, Clone)]
pub enum PollOutcome {
    /// The user has not approved yet. Keep polling.
    Pending,
    /// We polled too fast; GitHub asks us to back off.
    SlowDown,
    Granted(Box<TokenResponse>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Viewer {
    pub login: String,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthClient {
    http: reqwest::Client,
    endpoints: Endpoints,
    /// `None` when the app has not been configured with a GitHub client id.
    /// Operations that need it return an actionable `Config` error rather than
    /// failing at startup, because the app is usable for browsing settings
    /// without being signed in.
    client_id: Option<String>,
}

impl OAuthClient {
    #[must_use]
    pub fn new(http: reqwest::Client, endpoints: Endpoints, client_id: Option<String>) -> Self {
        Self {
            http,
            endpoints,
            client_id,
        }
    }

    #[must_use]
    pub fn endpoints(&self) -> &Endpoints {
        &self.endpoints
    }

    fn client_id(&self) -> Result<&str, AppError> {
        self.client_id.as_deref().ok_or_else(|| {
            AppError::Config(
                "no GitHub client id is configured. Register an OAuth App or GitHub App with \
                 device flow enabled, then set GITPULSE_GITHUB_CLIENT_ID."
                    .to_owned(),
            )
        })
    }

    /// Step 1: ask GitHub for a device code and the code the user must type.
    pub async fn request_device_code(
        &self,
        scopes: &[String],
    ) -> Result<DeviceCodeGrant, AppError> {
        let mut form = vec![("client_id", self.client_id()?.to_owned())];
        if !scopes.is_empty() {
            form.push(("scope", scopes.join(" ")));
        }

        let response = self
            .http
            .post(self.endpoints.device_code_url())
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&form)
            .send()
            .await
            .map_err(transport_error)?;

        // GitHub answers an unrecognised client id with a plain 404, not an
        // OAuth error body. Left to the generic handler that becomes
        // "resource not found", which sends the user looking in the wrong
        // place entirely.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::Config(
                "GitHub does not recognise this client id. Check that \
                 GITPULSE_GITHUB_CLIENT_ID matches the app's Client ID, and that device flow \
                 is enabled on that app."
                    .to_owned(),
            ));
        }

        if !response.status().is_success() {
            return Err(status_error(response).await);
        }

        let body: DeviceCodeResponse = response.json().await.map_err(transport_error)?;

        Ok(DeviceCodeGrant {
            device_code: body.device_code,
            user_code: body.user_code,
            verification_uri: body.verification_uri,
            expires_at: Utc::now() + chrono::Duration::seconds(body.expires_in.max(0)),
            interval: Duration::from_secs(body.interval.max(1)),
        })
    }

    /// Step 2: poll for the token. Expected to be called repeatedly.
    pub async fn poll_device_code(&self, device_code: &str) -> Result<PollOutcome, AppError> {
        let form = [
            ("client_id", self.client_id()?.to_owned()),
            ("device_code", device_code.to_owned()),
            ("grant_type", DEVICE_GRANT_TYPE.to_owned()),
        ];

        let raw = self.post_token_form(&form).await?;

        match raw.error.as_deref() {
            // Not an error: the user simply has not finished yet.
            Some("authorization_pending") => Ok(PollOutcome::Pending),
            Some("slow_down") => Ok(PollOutcome::SlowDown),
            Some("access_denied") => Err(AppError::AuthDenied),
            Some("expired_token") => Err(AppError::AuthExpired),
            Some("incorrect_device_code") => Err(AppError::Config(
                "GitHub rejected the device code. The sign-in has to be restarted.".to_owned(),
            )),
            Some("device_flow_disabled") => Err(AppError::Config(
                "device flow is not enabled for this GitHub app. Turn it on in the app's \
                 settings."
                    .to_owned(),
            )),
            Some("incorrect_client_credentials") => Err(AppError::Config(
                "GitHub does not recognise the configured client id".to_owned(),
            )),
            Some(other) => Err(AppError::Internal(format!(
                "GitHub returned an unexpected OAuth error: {other}"
            ))),
            None => Ok(PollOutcome::Granted(Box::new(raw.into_tokens()?))),
        }
    }

    /// Exchanges a refresh token for a new access token.
    ///
    /// GitHub rotates the refresh token: the one passed here is invalidated,
    /// and the caller must persist the new one before using the new access
    /// token.
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenResponse, AppError> {
        let form = [
            ("client_id", self.client_id()?.to_owned()),
            ("grant_type", "refresh_token".to_owned()),
            ("refresh_token", refresh_token.to_owned()),
        ];

        let raw = self.post_token_form(&form).await?;

        match raw.error.as_deref() {
            Some("bad_refresh_token") => Err(AppError::TokenRevoked),
            Some(other) => Err(AppError::Internal(format!(
                "GitHub rejected the refresh request: {other}"
            ))),
            None => raw.into_tokens(),
        }
    }

    /// `GET /user` — who the token belongs to, and whether it still works.
    pub async fn fetch_viewer(&self, access_token: &str) -> Result<Viewer, AppError> {
        let response = self
            .http
            .get(self.endpoints.viewer_url())
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header("X-GitHub-Api-Version", API_VERSION)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(transport_error)?;

        if !response.status().is_success() {
            return Err(status_error(response).await);
        }

        let body: ViewerResponse = response.json().await.map_err(transport_error)?;

        Ok(Viewer {
            login: body.login,
            avatar_url: body.avatar_url,
            display_name: body.name,
        })
    }

    async fn post_token_form(&self, form: &[(&str, String)]) -> Result<TokenRaw, AppError> {
        let response = self
            .http
            .post(self.endpoints.access_token_url())
            .header(reqwest::header::ACCEPT, "application/json")
            .form(form)
            .send()
            .await
            .map_err(transport_error)?;

        if !response.status().is_success() {
            return Err(status_error(response).await);
        }

        response.json().await.map_err(transport_error)
    }
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: i64,
    #[serde(default = "default_interval")]
    interval: u64,
}

const fn default_interval() -> u64 {
    DEFAULT_INTERVAL_SECS
}

/// One shape covers both the success and error cases: GitHub returns HTTP 200
/// with an `error` field for expected polling outcomes, so the status code
/// carries no signal here.
#[derive(Debug, Deserialize)]
struct TokenRaw {
    access_token: Option<String>,
    scope: Option<String>,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    refresh_token_expires_in: Option<i64>,
    error: Option<String>,
}

impl TokenRaw {
    fn into_tokens(self) -> Result<TokenResponse, AppError> {
        let access_token = self.access_token.ok_or_else(|| {
            AppError::Internal("GitHub returned no access token and no error".to_owned())
        })?;

        let now = Utc::now();

        Ok(TokenResponse {
            access_token,
            refresh_token: self.refresh_token,
            expires_at: self
                .expires_in
                .map(|seconds| now + chrono::Duration::seconds(seconds)),
            refresh_expires_at: self
                .refresh_token_expires_in
                .map(|seconds| now + chrono::Duration::seconds(seconds)),
            scopes: self
                .scope
                .unwrap_or_default()
                .split([',', ' '])
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct ViewerResponse {
    login: String,
    avatar_url: Option<String>,
    name: Option<String>,
}
