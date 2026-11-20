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

/// How many consecutive transport failures to ride out before giving up.
///
/// A device code lives for fifteen minutes; a network blip in that window must
/// not throw away the user's progress. Five consecutive failures at the polling
/// interval is long enough to survive a Wi-Fi handover and short enough to fail
/// honestly when the connection is really gone.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

/// Tracks the retry budget for transient polling failures.
///
/// Split out so the policy can be tested without a flaky network.
#[derive(Debug, Default)]
pub struct PollRetries {
    consecutive: u32,
}

impl PollRetries {
    /// Records a failure and answers whether to keep polling.
    pub fn record_failure(&mut self) -> bool {
        self.consecutive += 1;
        self.consecutive <= MAX_CONSECUTIVE_FAILURES
    }

    pub fn reset(&mut self) {
        self.consecutive = 0;
    }

    #[must_use]
    pub fn consecutive(&self) -> u32 {
        self.consecutive
    }
}

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
    let mut retries = PollRetries::default();

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
            Ok(PollOutcome::Pending) => retries.reset(),
            Ok(PollOutcome::SlowDown) => {
                schedule.slow_down();
                retries.reset();
                tracing::debug!(interval = ?schedule.interval(), "github asked us to slow down");
            }
            Err(error) if error.retryable() && retries.record_failure() => {
                // A drop in the network is not a decision by the user. Keep
                // polling: the code is still valid and the user may already
                // have approved it.
                tracing::warn!(
                    %error,
                    consecutive = retries.consecutive(),
                    "transient error while polling; the device code is still valid"
                );
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

    #[test]
    fn a_few_transport_failures_do_not_abandon_a_valid_code() {
        let mut retries = PollRetries::default();

        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            assert!(retries.record_failure(), "should keep polling");
        }

        assert_eq!(retries.consecutive(), MAX_CONSECUTIVE_FAILURES);
    }

    #[test]
    fn enough_consecutive_failures_gives_up() {
        let mut retries = PollRetries::default();

        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            retries.record_failure();
        }

        assert!(!retries.record_failure(), "the budget is spent");
    }

    #[test]
    fn a_success_resets_the_budget() {
        let mut retries = PollRetries::default();

        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            retries.record_failure();
        }

        retries.reset();

        assert_eq!(retries.consecutive(), 0);
        assert!(retries.record_failure(), "a fresh budget after a success");
    }
}
