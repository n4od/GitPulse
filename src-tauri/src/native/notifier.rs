//! Native notifications, with the restraint a desktop app owes its user.
//!
//! A CI monitor can generate hundreds of events an hour. Three rules keep that
//! from becoming noise:
//!
//! - **Only failures.** Success is the expected outcome; announcing it is spam.
//! - **Once per attempt.** A re-run is a new attempt and deserves its own
//!   notification; the same attempt seen twice does not.
//! - **Never in a burst.** At most one every [`THROTTLE`], so a matrix build
//!   failing twelve jobs produces one notification rather than twelve.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{Timelike, Utc};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::db::RepoCard;

/// Minimum gap between two notifications.
pub const THROTTLE: Duration = Duration::from_secs(10);

/// Conclusions worth interrupting someone for.
const NOTIFY_ON: &[&str] = &["failure", "timed_out", "startup_failure"];

pub struct Notifier {
    app: AppHandle,
    /// `(github run id, attempt)` pairs already announced.
    seen: Mutex<HashSet<(i64, i64)>>,
    last_sent: Mutex<Option<Instant>>,
    muted_until: Mutex<Option<Instant>>,
    /// `(start_hour, end_hour)` in local time, when the window may wrap midnight.
    quiet_hours: Mutex<Option<(u8, u8)>>,
    enabled: Mutex<bool>,
}

impl Notifier {
    #[must_use]
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            seen: Mutex::new(HashSet::new()),
            last_sent: Mutex::new(None),
            muted_until: Mutex::new(None),
            quiet_hours: Mutex::new(None),
            enabled: Mutex::new(true),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut slot) = self.enabled.lock() {
            *slot = enabled;
        }
    }

    /// Suppresses everything for `duration`. Used by the tray's mute item.
    pub fn mute_for(&self, duration: Duration) {
        if let Ok(mut slot) = self.muted_until.lock() {
            *slot = Some(Instant::now() + duration);
        }
    }

    pub fn set_quiet_hours(&self, window: Option<(u8, u8)>) {
        if let Ok(mut slot) = self.quiet_hours.lock() {
            *slot = window;
        }
    }

    #[must_use]
    pub fn is_muted(&self) -> bool {
        self.muted_until
            .lock()
            .ok()
            .and_then(|slot| *slot)
            .is_some_and(|until| until > Instant::now())
    }

    /// Decides whether `card`'s newest run warrants a notification, and shows
    /// one if so.
    pub fn maybe_notify(&self, card: &RepoCard, account: Option<&str>) {
        if !self.enabled.lock().is_ok_and(|slot| *slot) {
            return;
        }

        let Some(run) = card.recent_runs.first() else {
            return;
        };

        let conclusion = run.conclusion.as_deref().unwrap_or_default();
        if !NOTIFY_ON.contains(&conclusion) {
            return;
        }

        if self.is_muted() || self.in_quiet_hours() {
            return;
        }

        // Claim the (run, attempt) before doing anything slow, so two events
        // for the same run cannot both get through.
        let key = (run.github_id, run.run_attempt.unwrap_or(1));
        {
            let Ok(mut seen) = self.seen.lock() else {
                return;
            };

            if !seen.insert(key) {
                return;
            }

            // Bounded: a long session should not grow this without limit.
            if seen.len() > 500 {
                seen.clear();
                seen.insert(key);
            }
        }

        {
            let Ok(mut last) = self.last_sent.lock() else {
                return;
            };

            if last.is_some_and(|sent| sent.elapsed() < THROTTLE) {
                return;
            }

            *last = Some(Instant::now());
        }

        let workflow = run.workflow_name.as_deref().unwrap_or("workflow");
        let body = match conclusion {
            "timed_out" => format!("{} · {workflow} timed out", card.repo.full_name),
            "startup_failure" => format!("{} · {workflow} failed to start", card.repo.full_name),
            _ => format!("{} · {workflow} failed", card.repo.full_name),
        };

        // The account belongs in the title: the body already carries the
        // repository, and with more than one account signed in "acme/api" alone
        // does not say whose build it was.
        let title = match account {
            Some(login) => format!("Build failed — {login}"),
            None => "Build failed".to_owned(),
        };

        if let Err(error) = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
        {
            tracing::warn!(%error, "could not show a notification");
        }
    }

    fn in_quiet_hours(&self) -> bool {
        let Ok(window) = self.quiet_hours.lock() else {
            return false;
        };

        let Some((start, end)) = *window else {
            return false;
        };

        is_within(Utc::now().hour(), start, end)
    }
}

/// True when `hour` falls in a window that may wrap midnight.
#[must_use]
pub fn is_within(hour: u32, start: u8, end: u8) -> bool {
    let (start, end) = (u32::from(start), u32::from(end));

    if start == end {
        return false;
    }

    if start < end {
        (start..end).contains(&hour)
    } else {
        hour >= start || hour < end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_daytime_window() {
        assert!(is_within(9, 9, 17));
        assert!(is_within(16, 9, 17));
        assert!(!is_within(17, 9, 17), "the end hour is exclusive");
        assert!(!is_within(8, 9, 17));
    }

    #[test]
    fn a_window_that_wraps_midnight() {
        assert!(is_within(23, 22, 7));
        assert!(is_within(2, 22, 7));
        assert!(!is_within(12, 22, 7));
        assert!(!is_within(7, 22, 7), "the end hour is exclusive");
    }

    #[test]
    fn an_empty_window_is_never_quiet() {
        assert!(!is_within(0, 8, 8));
        assert!(!is_within(8, 8, 8));
    }

    #[test]
    fn only_failures_are_worth_interrupting_for() {
        // A guard against someone adding "success" to the list later.
        assert_eq!(NOTIFY_ON, &["failure", "timed_out", "startup_failure"]);
        assert!(!NOTIFY_ON.contains(&"success"));
        assert!(!NOTIFY_ON.contains(&"cancelled"));
    }

    #[test]
    fn the_throttle_is_long_enough_to_collapse_a_matrix_build() {
        assert!(THROTTLE >= Duration::from_secs(5));
    }
}
