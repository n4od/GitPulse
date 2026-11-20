//! The authenticated GitHub REST client.
//!
//! Every request goes through one pipeline, in this order:
//!
//! 1. **Budget** — wait for a local token so we never surprise GitHub.
//! 2. **Token** — ask the auth layer, which refreshes if it is close to expiry.
//! 3. **Conditional** — attach `If-None-Match` when we have a cached body.
//! 4. **Send**, then classify:
//!    - `304` returns the cached body and refunds the budget token (free request)
//!    - `401` means the token is dead, which is the auth layer's problem
//!    - `403` + `remaining: 0` is primary exhaustion: pause until the reset
//!    - `429` / `403` + `Retry-After` is a secondary limit: pause everyone
//!    - `5xx` and transport errors retry with full-jitter backoff
//!    - everything else is terminal
//!
//! Redirects are *not* followed here (see [`super::client::build_client`]).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::header::HeaderMap;
use reqwest::{Method, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use tokio::sync::watch;
use ts_rs::TS;

use crate::error::AppError;

use super::budget::RateBudget;
use super::client::{status_error, transport_error, Endpoints, API_VERSION, USER_AGENT};
use super::etag::EtagCache;

/// Total attempts for a retryable failure, including the first.
pub const MAX_ATTEMPTS: u32 = 4;

const BASE_BACKOFF: Duration = Duration::from_millis(500);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(60);

/// Supplies tokens without making this module depend on the auth internals.
#[async_trait]
pub trait TokenProvider: Send + Sync + 'static {
    async fn access_token(&self) -> Result<SecretString, AppError>;
}

/// What the UI's rate-limit meter shows.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct RateLimitSnapshot {
    pub limit: Option<i64>,
    pub remaining: Option<i64>,
    pub used: Option<i64>,
    pub reset_at: Option<DateTime<Utc>>,
    /// Set while a secondary limit is in force.
    pub paused_until: Option<DateTime<Utc>>,
    /// Tokens left in the local bucket, which is stricter than GitHub's.
    pub local_available: u32,
}

pub struct GithubClient {
    http: reqwest::Client,
    endpoints: Endpoints,
    tokens: Arc<dyn TokenProvider>,
    budget: Arc<RateBudget>,
    etags: EtagCache,
    rate: watch::Sender<RateLimitSnapshot>,
}

impl GithubClient {
    #[must_use]
    pub fn new(
        http: reqwest::Client,
        endpoints: Endpoints,
        tokens: Arc<dyn TokenProvider>,
        budget: Arc<RateBudget>,
    ) -> Self {
        let (rate, _) = watch::channel(RateLimitSnapshot::default());

        Self {
            http,
            endpoints,
            tokens,
            budget,
            etags: EtagCache::with_default_capacity(),
            rate,
        }
    }

    #[must_use]
    pub fn budget(&self) -> &Arc<RateBudget> {
        &self.budget
    }

    /// The underlying client, for the log downloader which needs its own
    /// redirect handling.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub(crate) fn token_provider(&self) -> &Arc<dyn TokenProvider> {
        &self.tokens
    }

    /// A receiver that fires whenever the rate-limit picture changes.
    #[must_use]
    pub fn rate_receiver(&self) -> watch::Receiver<RateLimitSnapshot> {
        self.rate.subscribe()
    }

    /// Builds an absolute API URL from a path.
    #[must_use]
    pub fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.endpoints.api, path)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        let bytes = self.execute(Method::GET, &self.api_url(path), None).await?;
        serde_json::from_slice(&bytes).map_err(|error| {
            AppError::Internal(format!("cannot decode the GitHub response: {error}"))
        })
    }

    /// Sends a bodyless `POST` or one with a JSON body, ignoring the response
    /// body. Every action endpoint in this app returns `201`/`204`.
    pub async fn post(&self, path: &str, body: Option<serde_json::Value>) -> Result<(), AppError> {
        self.execute(Method::POST, &self.api_url(path), body.as_ref())
            .await?;
        Ok(())
    }

    async fn execute(
        &self,
        method: Method,
        url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Vec<u8>, AppError> {
        let cache_key = format!("{method} {url}");
        let mut attempt: u32 = 0;

        loop {
            attempt += 1;
            self.budget.acquire().await?;

            let token = self.tokens.access_token().await?;

            let mut request = self
                .http
                .request(method.clone(), url)
                .header(reqwest::header::ACCEPT, "application/vnd.github+json")
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .header("X-GitHub-Api-Version", API_VERSION)
                .bearer_auth(token.expose_secret());

            if method == Method::GET {
                if let Some(etag) = self.etags.etag(&cache_key) {
                    request = request.header(reqwest::header::IF_NONE_MATCH, etag);
                }
            }

            if let Some(body) = body {
                request = request.json(body);
            }

            let response = match request.send().await {
                Ok(response) => response,
                Err(error) => {
                    if attempt < MAX_ATTEMPTS && is_retryable_transport(&error) {
                        tracing::warn!(%error, attempt, "retrying after a transport failure");
                        self.backoff(attempt).await;
                        continue;
                    }
                    return Err(transport_error(error));
                }
            };

            let status = response.status();
            let rate = self.read_rate_headers(response.headers());
            let _ = self.rate.send_replace(rate.snapshot.clone());

            if status == StatusCode::NOT_MODIFIED {
                // Conditional request: free against the real limit, so give the
                // local token back too.
                self.budget.refund();

                return self
                    .etags
                    .get(&cache_key)
                    .map(|cached| (*cached.body).clone())
                    .ok_or_else(|| {
                        AppError::Internal(
                            "GitHub returned 304 Not Modified but nothing was cached".to_owned(),
                        )
                    });
            }

            if status.is_success() {
                let etag = response
                    .headers()
                    .get(reqwest::header::ETAG)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);

                let bytes = response.bytes().await.map_err(transport_error)?.to_vec();

                if method == Method::GET {
                    if let Some(etag) = etag {
                        self.etags
                            .insert(cache_key.clone(), etag, Arc::new(bytes.clone()));
                    }
                }

                return Ok(bytes);
            }

            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::TokenRevoked);
            }

            // Primary limit: pause everything until GitHub's own reset time.
            if status == StatusCode::FORBIDDEN && rate.remaining == Some(0) {
                let reset_at = rate
                    .reset_at
                    .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1));

                self.budget.pause_until(reset_at);
                tracing::warn!(%reset_at, "primary rate limit exhausted");

                return Err(AppError::RateLimited { reset_at });
            }

            // Secondary limit: GitHub wants us to stop for a while.
            if status == StatusCode::TOO_MANY_REQUESTS
                || (status == StatusCode::FORBIDDEN && rate.retry_after.is_some())
            {
                let retry_after = rate.retry_after.unwrap_or(DEFAULT_RETRY_AFTER);

                self.budget.pause_until(
                    Utc::now() + chrono::Duration::from_std(retry_after).unwrap_or_default(),
                );
                tracing::warn!(?retry_after, "secondary rate limit; pausing the client");

                return Err(AppError::SecondaryRateLimited { retry_after });
            }

            if status.is_server_error() && attempt < MAX_ATTEMPTS {
                tracing::warn!(%status, attempt, "retrying after a server error");
                self.backoff(attempt).await;
                continue;
            }

            return Err(status_error(response).await);
        }
    }

    fn read_rate_headers(&self, headers: &HeaderMap) -> ObservedRate {
        let reset_at = header_i64(headers, "x-ratelimit-reset")
            .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single());
        let remaining = header_i64(headers, "x-ratelimit-remaining");

        let budget = self.budget.snapshot();

        ObservedRate {
            remaining,
            reset_at,
            retry_after: header_i64(headers, "retry-after")
                .and_then(|seconds| u64::try_from(seconds).ok())
                .map(Duration::from_secs),
            snapshot: RateLimitSnapshot {
                limit: header_i64(headers, "x-ratelimit-limit"),
                remaining,
                used: header_i64(headers, "x-ratelimit-used"),
                reset_at,
                paused_until: budget.paused_until,
                local_available: budget.available,
            },
        }
    }

    /// Full-jitter exponential backoff: a uniform choice in `[1ms, ceiling]`.
    /// The jitter matters more than the ceiling — without it, every client
    /// retries in lockstep after an outage.
    async fn backoff(&self, attempt: u32) {
        let ceiling = BASE_BACKOFF
            .saturating_mul(1 << attempt.min(6))
            .min(MAX_BACKOFF);
        let millis = u64::try_from(ceiling.as_millis()).unwrap_or(1).max(1);
        let jitter = (nanos_now() % millis).max(1);

        tokio::time::sleep(Duration::from_millis(jitter)).await;
    }
}

struct ObservedRate {
    remaining: Option<i64>,
    reset_at: Option<DateTime<Utc>>,
    retry_after: Option<Duration>,
    snapshot: RateLimitSnapshot,
}

fn header_i64(headers: &HeaderMap, name: &str) -> Option<i64> {
    headers.get(name)?.to_str().ok()?.trim().parse().ok()
}

fn is_retryable_transport(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

fn nanos_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| u64::from(elapsed.subsec_nanos()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_ceiling_doubles_and_caps() {
        let ceiling = |attempt: u32| {
            BASE_BACKOFF
                .saturating_mul(1 << attempt.min(6))
                .min(MAX_BACKOFF)
        };

        assert_eq!(ceiling(1), Duration::from_secs(1));
        assert_eq!(ceiling(2), Duration::from_secs(2));
        assert_eq!(ceiling(3), Duration::from_secs(4));
        // Capped well before the shift would overflow.
        assert_eq!(ceiling(9), MAX_BACKOFF);
    }
}
