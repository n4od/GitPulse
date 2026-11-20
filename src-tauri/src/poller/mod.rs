//! The poller: keeps the local cache in step with GitHub.
//!
//! One loop, one request at a time. That sounds slow, and it is the point: the
//! whole sweep of 250 repositories at a 257-second interval needs roughly one
//! request per second, while each request takes about 200 ms. Concurrency would
//! add a scheduler, a semaphore and a set of in-flight bookkeeping to save time
//! we do not need, and would make "never fetch the same repository twice"
//! something to prove rather than something structurally true.
//!
//! Emission is diff-only: [`crate::db::diff::diff_runs`] returns the runs whose
//! observable state actually changed, and only those are published.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::db::{self, Db, Repo, RunUpsert, WorkflowRun};
use crate::error::AppError;
use crate::github::budget::DEFAULT_HOURLY_CEILING;
use crate::github::{GithubClient, RateBudget};

pub mod schedule;

#[cfg(test)]
mod tests;

pub use schedule::{interval_for, is_due, target_interval, MAX_INTERVAL, MIN_INTERVAL};

pub const POLLER_STATUS_EVENT: &str = "poller://status";
pub const RUNS_UPDATED_EVENT: &str = "run://updated";

#[derive(Debug, Clone)]
pub struct PollConfig {
    /// How often the loop wakes to look for due repositories. Waking is cheap:
    /// it is one indexed query.
    pub tick: Duration,
    pub min_interval: Duration,
    pub max_interval: Duration,
    /// Runs requested per repository per poll. The dashboard only needs the head.
    pub fetch_limit: u32,
    pub hourly_ceiling: f64,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            tick: Duration::from_secs(30),
            min_interval: MIN_INTERVAL,
            max_interval: MAX_INTERVAL,
            fetch_limit: 20,
            hourly_ceiling: DEFAULT_HOURLY_CEILING,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunsChangedEvent {
    pub repo_id: i64,
    pub runs: Vec<WorkflowRun>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollerStatus {
    pub monitored: usize,
    pub target_interval_secs: u64,
    pub budget_available: u32,
    pub last_error: Option<String>,
}

/// Where the poller publishes. Injected so the loop can be tested without a
/// window; the Tauri adapter lives in the composition root.
pub trait PollEvents: Send + Sync + 'static {
    fn runs_changed(&self, event: RunsChangedEvent);
    fn status(&self, status: PollerStatus);
}

pub struct Poller {
    db: Db,
    github: Arc<GithubClient>,
    events: Arc<dyn PollEvents>,
    config: PollConfig,
    /// Repositories the user asked to refresh. Drained once per tick, and
    /// deduplicated so mashing a button queues one poll, not ten.
    urgent: Mutex<Vec<i64>>,
    wake: Notify,
    cancel: CancellationToken,
    last_error: Mutex<Option<String>>,
}

impl Poller {
    #[must_use]
    pub fn new(
        db: Db,
        github: Arc<GithubClient>,
        events: Arc<dyn PollEvents>,
        config: PollConfig,
    ) -> Self {
        Self {
            db,
            github,
            events,
            config,
            urgent: Mutex::new(Vec::new()),
            wake: Notify::new(),
            cancel: CancellationToken::new(),
            last_error: Mutex::new(None),
        }
    }

    /// Asks for a repository to be polled on the next tick, ahead of the
    /// schedule. Idempotent: a second request for the same repository while the
    /// first is pending changes nothing.
    pub fn request_refresh(&self, repo_id: i64) {
        if let Ok(mut queue) = self.urgent.lock() {
            if !queue.contains(&repo_id) {
                queue.push(repo_id);
            }
        }

        self.wake.notify_one();
    }

    pub fn stop(&self) {
        self.cancel.cancel();
        // Release the loop if it is waiting on the tick rather than on due
        // repositories.
        self.wake.notify_one();
    }

    /// Runs forever until [`Self::stop`] is called.
    pub async fn run(self: Arc<Self>) {
        tracing::info!(
            tick_secs = self.config.tick.as_secs(),
            min_interval_secs = self.config.min_interval.as_secs(),
            "poller started"
        );

        loop {
            if let Err(error) = self.tick().await {
                tracing::warn!(%error, "poller tick failed");
                self.record_error(&error);
            }

            tokio::select! {
                () = self.cancel.cancelled() => break,
                () = self.wake.notified() => {}
                () = tokio::time::sleep(self.config.tick) => {}
            }
        }

        tracing::info!("poller stopped");
    }

    pub async fn tick(&self) -> Result<usize, AppError> {
        self.tick_at(Utc::now()).await
    }

    /// One scheduling pass. `now` is injected so tests can advance the clock
    /// without sleeping.
    pub async fn tick_at(&self, now: DateTime<Utc>) -> Result<usize, AppError> {
        let repositories = self.db.with_connection(db::repos::list_monitored).await?;
        let monitored = repositories.len();

        if monitored == 0 {
            self.publish_status(monitored, MIN_INTERVAL.as_secs());
            return Ok(0);
        }

        let target = target_interval(monitored, self.config.hourly_ceiling);
        let active = self.active_repositories().await?;
        let urgent = self.drain_urgent();

        let mut ordered: Vec<(Repo, bool)> = repositories
            .into_iter()
            .map(|repo| {
                let has_active_run = active.contains(&repo.id);
                (repo, has_active_run)
            })
            .collect();

        // Manual requests first, then repositories with something in flight,
        // then alphabetically so the sweep is deterministic.
        ordered.sort_by_key(|(repo, has_active_run)| {
            (
                !urgent.contains(&repo.id),
                !*has_active_run,
                repo.full_name.clone(),
            )
        });

        let mut polled = 0_usize;

        for (repo, has_active_run) in ordered {
            let wanted = urgent.contains(&repo.id)
                || is_due(
                    repo.last_polled_at,
                    interval_for(has_active_run, target, self.config.min_interval),
                    now,
                );

            if !wanted {
                continue;
            }

            // Another request may already have spent the token this poll would
            // need; deferring is cheaper than blocking the whole sweep.
            if self.budget().available() == 0 {
                tracing::debug!("local budget exhausted; deferring the rest of the sweep");
                break;
            }

            match self.poll_repo(&repo).await {
                Ok(()) => polled += 1,
                Err(error) if is_stopping(&error) => {
                    if matches!(error, AppError::AuthRequired) {
                        tracing::debug!("not signed in; skipping this sweep");
                    } else {
                        tracing::warn!(%error, "pausing the sweep");
                    }
                    self.record_error(&error);
                    break;
                }
                Err(error) => {
                    tracing::warn!(repo = %repo.full_name, %error, "poll failed");
                    self.record_error(&error);
                }
            }
        }

        self.publish_status(monitored, target.as_secs());

        Ok(polled)
    }

    async fn poll_repo(&self, repo: &Repo) -> Result<(), AppError> {
        let runs = self
            .github
            .runs(&repo.full_name, self.config.fetch_limit)
            .await?;

        let repo_id = repo.id;
        let upserts: Vec<RunUpsert> = runs.iter().map(|run| run.to_upsert(repo_id)).collect();

        // Writing and diffing in one closure keeps it a single transaction.
        let diff = self
            .db
            .with_connection(move |connection| db::diff::diff_runs(connection, repo_id, &upserts))
            .await?;

        self.db
            .with_connection(move |connection| db::repos::touch_polled(connection, repo_id))
            .await?;

        if !diff.is_empty() {
            self.events.runs_changed(RunsChangedEvent {
                repo_id,
                runs: diff.changed,
            });
        }

        Ok(())
    }

    async fn active_repositories(&self) -> Result<HashSet<i64>, AppError> {
        self.db
            .with_connection(db::runs::repos_with_active_runs)
            .await
            .map(|ids| ids.into_iter().collect())
    }

    fn drain_urgent(&self) -> HashSet<i64> {
        self.urgent
            .lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default()
    }

    fn budget(&self) -> &Arc<RateBudget> {
        self.github.budget()
    }

    fn record_error(&self, error: &AppError) {
        if let Ok(mut slot) = self.last_error.lock() {
            *slot = Some(error.to_string());
        }
    }

    fn publish_status(&self, monitored: usize, target_interval_secs: u64) {
        self.events.status(PollerStatus {
            monitored,
            target_interval_secs,
            budget_available: self.budget().available(),
            last_error: self.last_error.lock().ok().and_then(|slot| slot.clone()),
        });
    }
}

/// Errors that mean "stop this sweep", as opposed to "this one repository
/// failed". Hammering a client that just told us to stop is how a rate limit
/// becomes an outage.
fn is_stopping(error: &AppError) -> bool {
    matches!(
        error,
        AppError::AuthRequired
            | AppError::RateLimited { .. }
            | AppError::SecondaryRateLimited { .. }
            | AppError::TokenRevoked
    )
}
