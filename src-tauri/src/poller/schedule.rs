//! Scheduling maths, kept pure so it can be tested without a clock.
//!
//! The interval is a *plan*: it says how often each repository should be
//! revisited. The token bucket in [`crate::github::budget`] is the
//! *enforcement*: it is what actually stops us. Two mechanisms, two jobs —
//! the plan keeps the sweep evenly spread, the bucket keeps the promise.

use std::time::Duration;

use chrono::{DateTime, Utc};

/// Never poll a repository more often than this, however few there are.
pub const MIN_INTERVAL: Duration = Duration::from_secs(60);

/// Never let a repository go stale for longer than this.
pub const MAX_INTERVAL: Duration = Duration::from_secs(900);

/// Interval that fits `repos` polls inside `hourly_ceiling`.
///
/// This is the whole budget argument in one line: 250 repositories against a
/// 3,500/hour ceiling is 257 seconds each. Ten repositories would be 11
/// seconds, which is why the result is clamped up to [`MIN_INTERVAL`] — there
/// is no point hammering GitHub when the answer will not have changed.
#[must_use]
pub fn target_interval(repos: usize, hourly_ceiling: f64) -> Duration {
    if repos == 0 {
        return MIN_INTERVAL;
    }

    // A repo count is nowhere near 2^53, so the precision loss is theoretical.
    #[allow(clippy::cast_precision_loss)]
    let requested = (repos as f64) * 3600.0 / hourly_ceiling.max(1.0);

    let seconds = requested
        .ceil()
        .clamp(MIN_INTERVAL.as_secs_f64(), MAX_INTERVAL.as_secs_f64());

    Duration::from_secs_f64(seconds)
}

/// A repository with a run in flight is the only thing worth polling quickly —
/// that is where the state is about to change on its own.
#[must_use]
pub fn interval_for(has_active_run: bool, target: Duration, floor: Duration) -> Duration {
    if has_active_run {
        floor.min(target)
    } else {
        target
    }
}

/// Whether a repository is due for a poll.
///
/// Uses wall-clock time rather than a monotonic instant, which means a resume
/// after sleep needs no special handling: every repository is simply overdue
/// and gets picked up on the next tick.
#[must_use]
pub fn is_due(last_polled: Option<DateTime<Utc>>, interval: Duration, now: DateTime<Utc>) -> bool {
    match last_polled {
        None => true,
        // A clock that moved backwards (timezone change, NTP correction) must
        // not park a repository forever.
        Some(last) => (now - last)
            .to_std()
            .map_or(true, |elapsed| elapsed >= interval),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CEILING: f64 = 3_500.0;

    #[test]
    fn ten_repositories_are_clamped_up_to_the_floor() {
        // 10 * 3600 / 3500 = 10.3s, well under the floor.
        assert_eq!(target_interval(10, CEILING), MIN_INTERVAL);
    }

    #[test]
    fn a_hundred_repositories_land_just_under_two_minutes() {
        // 100 * 3600 / 3500 = 102.9s, rounded up.
        assert_eq!(target_interval(100, CEILING), Duration::from_secs(103));
    }

    #[test]
    fn five_hundred_repositories_land_near_nine_minutes() {
        // 500 * 3600 / 3500 = 514.3s, rounded up.
        assert_eq!(target_interval(500, CEILING), Duration::from_secs(515));
    }

    #[test]
    fn an_absurd_repository_count_is_clamped_down() {
        // 4000 * 3600 / 3500 = 4114s, clamped to the ceiling.
        assert_eq!(target_interval(4_000, CEILING), MAX_INTERVAL);
    }

    #[test]
    fn no_repositories_means_no_waiting_policy() {
        assert_eq!(target_interval(0, CEILING), MIN_INTERVAL);
    }

    #[test]
    #[allow(clippy::cast_precision_loss, reason = "repo counts are far below 2^53")]
    fn the_budget_derived_interval_keeps_the_sweep_inside_the_ceiling() {
        // The guarantee, stated as arithmetic: polls per hour must not exceed
        // the ceiling once the plan is applied.
        for repos in [10_usize, 100, 250, 500] {
            let interval = target_interval(repos, CEILING);
            let polls_per_hour = (repos as f64) * 3600.0 / interval.as_secs_f64();
            assert!(
                polls_per_hour <= CEILING,
                "{repos} repositories at {interval:?} would need {polls_per_hour:.0} polls/hour"
            );
        }
    }

    #[test]
    fn an_active_run_pulls_the_interval_down_to_the_floor() {
        let target = target_interval(250, CEILING);
        assert_eq!(
            interval_for(true, target, MIN_INTERVAL),
            MIN_INTERVAL,
            "a running workflow is the only thing worth polling fast"
        );
        assert_eq!(interval_for(false, target, MIN_INTERVAL), target);
    }

    #[test]
    fn a_never_polled_repository_is_due_immediately() {
        assert!(is_due(None, MIN_INTERVAL, Utc::now()));
    }

    #[test]
    fn a_just_polled_repository_is_not_due() {
        let now = Utc::now();
        assert!(!is_due(
            Some(now - chrono::Duration::seconds(5)),
            MIN_INTERVAL,
            now
        ));
    }

    #[test]
    fn a_repository_polled_before_the_interval_is_due() {
        let now = Utc::now();
        assert!(is_due(
            Some(now - chrono::Duration::seconds(61)),
            MIN_INTERVAL,
            now
        ));
    }

    #[test]
    fn waking_from_sleep_makes_everything_overdue() {
        // The machine slept for two hours: no special-casing needed, wall-clock
        // due times simply pass.
        let now = Utc::now();
        assert!(is_due(
            Some(now - chrono::Duration::hours(2)),
            MAX_INTERVAL,
            now
        ));
    }

    #[test]
    fn a_backwards_clock_does_not_park_a_repository() {
        let now = Utc::now();
        assert!(
            is_due(Some(now + chrono::Duration::hours(1)), MIN_INTERVAL, now),
            "a future timestamp must be treated as overdue, not as never-due"
        );
    }
}
