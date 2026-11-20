//! Local request budget.
//!
//! GitHub's authenticated primary limit is 5,000 requests per hour. Polling 250
//! repositories on a fixed interval blows straight through that, so the client
//! is paced locally at 70% of the ceiling and the reserve is left for the user
//! pressing "refresh".
//!
//! This is a token bucket, not a fixed interval: bursts are legitimate (a
//! dashboard opening polls a dozen repositories at once) and a fixed interval
//! would make the UI feel stuck.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

use crate::error::AppError;

/// 70% of GitHub's 5,000/hour authenticated ceiling.
pub const DEFAULT_HOURLY_CEILING: f64 = 3_500.0;

/// How many requests may go out back-to-back before pacing kicks in.
pub const DEFAULT_BURST: f64 = 30.0;

/// Waits longer than this are surfaced to the caller instead of being slept
/// inline. An hour-long sleep inside a request would be indistinguishable from
/// a hang, and would ignore cancellation.
const MAX_INLINE_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BudgetSnapshot {
    /// Tokens available right now, rounded down.
    pub available: u32,
    /// Set while a secondary-limit pause is in force.
    pub paused_until: Option<DateTime<Utc>>,
}

struct State {
    tokens: f64,
    refilled_at: Instant,
    paused_until: Option<Instant>,
    /// Diagnostics: how many requests the local bucket has handed out.
    granted: u64,
}

pub struct RateBudget {
    capacity: f64,
    refill_per_second: f64,
    state: Mutex<State>,
}

impl RateBudget {
    #[must_use]
    pub fn new(per_hour: f64, burst: f64) -> Self {
        let capacity = burst.max(1.0);

        Self {
            capacity,
            refill_per_second: (per_hour.max(1.0) / 3600.0),
            state: Mutex::new(State {
                tokens: capacity,
                refilled_at: Instant::now(),
                paused_until: None,
                granted: 0,
            }),
        }
    }

    #[must_use]
    pub fn for_github() -> Self {
        Self::new(DEFAULT_HOURLY_CEILING, DEFAULT_BURST)
    }

    /// Blocks until a request may be sent.
    ///
    /// Returns [`AppError::RateLimited`] rather than blocking when the wait
    /// would exceed [`MAX_INLINE_WAIT`], so a caller can schedule a retry
    /// instead of appearing to hang.
    pub async fn acquire(&self) -> Result<(), AppError> {
        loop {
            match self.try_acquire() {
                Ok(()) => return Ok(()),
                Err(wait) if wait <= MAX_INLINE_WAIT => tokio::time::sleep(wait).await,
                Err(wait) => {
                    let reset = chrono::Duration::from_std(wait).unwrap_or_default();
                    return Err(AppError::RateLimited {
                        reset_at: Utc::now() + reset,
                    });
                }
            }
        }
    }

    /// Stops every request until `when`. Used when GitHub reports a secondary
    /// limit: the whole client must back off, not just the request that hit it.
    pub fn pause_until(&self, when: DateTime<Utc>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        let duration = (when - Utc::now()).to_std().unwrap_or_default();
        let target = Instant::now() + duration;

        if state.paused_until.is_none_or(|current| current < target) {
            state.paused_until = Some(target);
        }
    }

    #[must_use]
    // `tokens` is clamped to a non-negative value below `u32::MAX` before the
    // cast, so neither truncation nor sign loss can happen here.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn snapshot(&self) -> BudgetSnapshot {
        let Ok(state) = self.state.lock() else {
            return BudgetSnapshot::default();
        };

        let paused_until = state
            .paused_until
            .filter(|target| *target > Instant::now())
            .map(|target| {
                Utc::now() + chrono::Duration::from_std(target - Instant::now()).unwrap_or_default()
            });

        let available = state.tokens.floor().clamp(0.0, f64::from(u32::MAX));

        BudgetSnapshot {
            available: available as u32,
            paused_until,
        }
    }

    /// Requests granted locally since construction. Diagnostics only.
    #[must_use]
    pub fn granted(&self) -> u64 {
        self.state.lock().map_or(0, |state| state.granted)
    }

    /// Gives a token back.
    ///
    /// A `304 Not Modified` is free against GitHub's real limit, but the client
    /// had to spend a token before it knew the response would be a `304`. This
    /// returns it, so conditional polling genuinely costs nothing.
    pub fn refund(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        state.tokens = (state.tokens + 1.0).min(self.capacity);
        state.granted = state.granted.saturating_sub(1);
    }

    /// `Ok(())` if a token was taken, `Err(wait)` otherwise.
    fn try_acquire(&self) -> Result<(), Duration> {
        let Ok(mut state) = self.state.lock() else {
            // A poisoned budget must not wedge the app; fail open and let the
            // server enforce the real limit.
            return Ok(());
        };

        let now = Instant::now();
        state.tokens = refill(
            state.tokens,
            now.saturating_duration_since(state.refilled_at),
            self.refill_per_second,
            self.capacity,
        );
        state.refilled_at = now;

        if let Some(paused_until) = state.paused_until {
            if paused_until > now {
                return Err(paused_until - now);
            }
            state.paused_until = None;
        }

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            state.granted += 1;
            return Ok(());
        }

        // Time until the bucket holds one whole token.
        let deficit = 1.0 - state.tokens;
        let seconds = deficit / self.refill_per_second;
        Err(Duration::from_secs_f64(seconds.max(0.001)))
    }
}

/// Pure refill maths, split out so it can be tested without a clock.
fn refill(tokens: f64, elapsed: Duration, per_second: f64, capacity: f64) -> f64 {
    (tokens + elapsed.as_secs_f64() * per_second).min(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refill_saturates_at_capacity() {
        let tokens = refill(0.0, Duration::from_secs(3600), 1.0, 30.0);
        assert!(
            (tokens - 30.0).abs() < 1e-9,
            "expected saturation, got {tokens}"
        );
    }

    #[test]
    fn refill_accumulates_fractionally() {
        let tokens = refill(0.0, Duration::from_millis(500), 1.0, 30.0);
        assert!((tokens - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn a_burst_is_allowed_before_pacing() {
        let budget = RateBudget::new(3_600.0, 5.0);
        for _ in 0..5 {
            assert!(budget.try_acquire().is_ok(), "the burst should be free");
        }
        assert!(
            budget.try_acquire().is_err(),
            "the sixth request must wait for the bucket"
        );
        assert_eq!(budget.granted(), 5);
    }

    #[test]
    fn the_deficit_reports_a_realistic_wait() {
        // One token per second: after spending the burst, the next wait is ~1s.
        let budget = RateBudget::new(3_600.0, 1.0);
        assert!(budget.try_acquire().is_ok());

        let wait = budget.try_acquire().expect_err("must wait");
        assert!(
            wait >= Duration::from_millis(900) && wait <= Duration::from_millis(1_100),
            "unexpected wait: {wait:?}"
        );
    }

    #[tokio::test]
    async fn a_pause_blocks_even_a_full_bucket() {
        let budget = RateBudget::new(3_600.0, 30.0);
        budget.pause_until(Utc::now() + chrono::Duration::seconds(2));

        let wait = budget.try_acquire().expect_err("paused");
        assert!(wait > Duration::from_secs(1), "wait was {wait:?}");
        assert!(budget.snapshot().paused_until.is_some());
    }

    #[tokio::test]
    async fn acquire_waits_inline_for_short_pauses() {
        let budget = RateBudget::new(3_600.0, 1.0);
        budget.pause_until(Utc::now() + chrono::Duration::milliseconds(50));

        let started = Instant::now();
        budget
            .acquire()
            .await
            .expect("short pauses are slept inline");
        assert!(started.elapsed() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn acquire_surfaces_a_long_wait_instead_of_hanging() {
        let budget = RateBudget::new(3_600.0, 1.0);
        // An empty bucket at one token per second far exceeds the inline limit.
        budget.pause_until(Utc::now() + chrono::Duration::hours(1));

        let error = budget.acquire().await.expect_err("an hour is not inline");
        assert_eq!(error.code(), "rate_limited");
    }
}
