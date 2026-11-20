//! Local `SQLite` cache.
//!
//! # Why a single connection
//!
//! The plan called for a read pool of four plus one write connection. That was
//! dropped after building the schema: every query in the dashboard's hot path
//! is an indexed lookup over a few thousand rows, and a poll cycle issues tens
//! of statements, not thousands. A pool would add a checkout, a `Semaphore` and
//! a background thread set to save microseconds.
//!
//! What the design does need — and what a pool would have given us — is that no
//! two writers ever race for the file lock. A single `Mutex<Connection>` gives
//! that for free: writes serialise, and WAL means reads never block on them.
//! If profiling ever shows contention, add readers then, with a measurement in
//! hand.
//!
//! Blocking work runs on `spawn_blocking` so a slow query cannot stall the
//! async runtime that owns the poller and the webhook listener.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::AppError;

pub mod deliveries;
pub mod deployments;
pub mod diff;
pub mod jobs;
pub mod meta;
pub mod migrate;
pub mod models;
pub mod pulls;
pub mod repos;
pub mod retention;
pub mod runs;

#[cfg(test)]
mod tests;

pub use diff::RunDiff;
pub use models::{
    Deployment, DeploymentUpsert, Job, JobUpsert, PullRequest, PullRequestUpsert, Repo, RepoCard,
    RepoUpsert, RunUpsert, WorkflowRun,
};
pub use retention::RetentionReport;

/// Applied to every connection immediately after it is opened.
///
/// `foreign_keys` is the important one: it is per-connection and defaults to
/// OFF, so without this every `ON DELETE CASCADE` in the schema is silently
/// inert and deleting a repository would orphan its runs, jobs and pull
/// requests.
const PRAGMAS: &str = "
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;
";

#[derive(Clone)]
pub struct Db {
    inner: Arc<Inner>,
}

struct Inner {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl Db {
    /// Opens (or creates) the database, applies pragmas and migrates.
    ///
    /// Safe to call on every launch: migrations are recorded and skipped.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    AppError::Config(format!(
                        "cannot create data directory {}: {error}",
                        parent.display()
                    ))
                })?;
            }
        }

        let connection = Connection::open(&path).map_err(AppError::db)?;
        let connection = Self::prepare(connection)?;

        Ok(Self {
            inner: Arc::new(Inner {
                connection: Mutex::new(connection),
                path,
            }),
        })
    }

    /// In-memory database, for tests and for a "session only" mode.
    pub fn open_in_memory() -> Result<Self, AppError> {
        let connection = Connection::open_in_memory().map_err(AppError::db)?;
        let connection = Self::prepare(connection)?;

        Ok(Self {
            inner: Arc::new(Inner {
                connection: Mutex::new(connection),
                path: PathBuf::from(":memory:"),
            }),
        })
    }

    fn prepare(connection: Connection) -> Result<Connection, AppError> {
        connection.execute_batch(PRAGMAS).map_err(AppError::db)?;
        migrate::apply(&connection)?;
        Ok(connection)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// Runs a closure against the connection on a blocking thread.
    ///
    /// The closure takes `&Connection`; DAOs that need a transaction create one
    /// internally with `unchecked_transaction`, which needs only a shared
    /// borrow — so callers never have to think about `&mut`.
    pub async fn with_connection<T, F>(&self, operation: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let inner = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            let connection = lock(&inner.connection)?;
            operation(&connection)
        })
        .await
        .map_err(|error| AppError::Internal(format!("database worker failed: {error}")))?
    }
}

fn lock(connection: &Mutex<Connection>) -> Result<MutexGuard<'_, Connection>, AppError> {
    connection.lock().map_err(|_| {
        AppError::Internal("the database connection was poisoned by an earlier panic".to_owned())
    })
}
