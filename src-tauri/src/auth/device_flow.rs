//! The device-flow polling loop.
//!
//! Kept separate from the HTTP client so the control flow — including the
//! back-off rules and cancellation — can be read and tested on its own.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::AppError;

use super::oauth::{OAuthClient, PollOutcome, TokenResponse};
use super::types::AuthState;

/// GitHub's `slow_down` rule: each time it is returned, the polling interval
/// must grow by five seconds for the rest of the session. It is cumulative, not
/// a temporary nudge.
const SLOW_DOWN_STEP: Duration = Duration::from_secs(5);

/// Polling interval with its back-off state.
///
/// Split out so the growth rule can be tested without sleeping for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollSchedule {
    interval: Duration,
}

impl PollSchedule {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }

    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn slow_down(&mut self) {
        self.interval += SLOW_DOWN_STEP;
    }
}

/// Runs the flow to completion.
///
/// `on_state` is called on every transition so the caller can surface progress;
/// `open_browser` is called once with the verification URI. Both are injected
/// rather than reaching for Tauri so this function stays testable.
pub async fn run<F, B>(
    client: &OAuthClient,
    scopes: &[String],
    cancel: &CancellationToken,
    mut on_state: F,
    open_browser: B,
) -> Result<TokenResponse, AppError>
where
    F: FnMut(AuthState),
    B: FnOnce(&str) -> Result<(), AppError>,
{
    on_state(AuthState::RequestingCode);

    let grant = client.request_device_code(scopes).await?;
    open_browser(&grant.verification_uri)?;

    on_state(AuthState::AwaitingUser {
        user_code: grant.user_code.clone(),
        verification_uri: grant.verification_uri.clone(),
        expires_at: grant.expires_at,
    });

    let mut schedule = PollSchedule::new(grant.interval);

    loop {
        if chrono::Utc::now() >= grant.expires_at {
            return Err(AppError::AuthExpired);
        }

        on_state(AuthState::Polling {
            user_code: grant.user_code.clone(),
            verification_uri: grant.verification_uri.clone(),
            expires_at: grant.expires_at,
        });

        match client.poll_device_code(&grant.device_code).await {
            Ok(PollOutcome::Granted(tokens)) => return Ok(*tokens),
            Ok(PollOutcome::Pending) => {}
            Ok(PollOutcome::SlowDown) => {
                schedule.slow_down();
                tracing::debug!(interval = ?schedule.interval(), "github asked us to slow down");
            }
            Err(AppError::AuthDenied) => return Err(AppError::AuthDenied),
            Err(error) => return Err(error),
        }

        // Sleep *after* each attempt, and make that sleep interruptible: the
        // user must be able to abandon a flow that is waiting on a code they
        // never entered.
        tokio::select! {
            () = cancel.cancelled() => return Err(AppError::AuthCancelled),
            () = tokio::time::sleep(schedule.interval()) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_down_grows_the_interval_by_five_seconds_each_time() {
        let mut schedule = PollSchedule::new(Duration::from_secs(5));
        assert_eq!(schedule.interval(), Duration::from_secs(5));

        schedule.slow_down();
        assert_eq!(schedule.interval(), Duration::from_secs(10));

        // The rule is cumulative, not a one-off nudge.
        schedule.slow_down();
        assert_eq!(schedule.interval(), Duration::from_secs(15));
    }

    #[test]
    fn schedule_starts_from_the_interval_github_supplied() {
        let schedule = PollSchedule::new(Duration::from_secs(12));
        assert_eq!(schedule.interval(), Duration::from_secs(12));
    }
}
