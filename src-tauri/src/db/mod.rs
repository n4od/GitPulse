//! Local `SQLite` cache.
//!
//! # One file per account
//!
//! The cache holds what *one account* could see, so it is stored as one database
//! per account — `gitpulse-<login>.db`. Isolation is then structural rather than
//! a filter on every read: another account's rows are not in the database to
//! leak. `meta.account_login` is kept as a cross-check, because a file copied or
//! renamed under someone else's name would otherwise be trusted.
//!
//! The handle is deliberately indirect. Callers hold a [`Db`] clone — the poller,
//! the tray, every command - and the connection behind it is
//! swapped when the account changes, so none of them has to be rebuilt.
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
//! async runtime that owns the poller.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use crate::accounts;
use crate::error::AppError;

pub mod cache;
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
pub use jobs::JobContext;
pub use models::{
    Deployment, DeploymentUpsert, Job, JobUpsert, PullRequest, PullRequestUpsert, Repo, RepoCard,
    RepoUpsert, RunUpsert, WorkflowRun,
};
pub use retention::RetentionReport;
pub use runs::RunContext;

/// The database written before there was one per account.
const LEGACY_FILE: &str = "gitpulse.db";

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
    /// Where an account's database lives. Unused while the database in memory is
    /// the one in use.
    directory: PathBuf,
    /// The database in use, swapped when the account changes.
    ///
    /// There is always one. It is an in-memory database until an account is
    /// known, so a read while signed out returns nothing rather than an error —
    /// which is what the screen behind it shows anyway. Nothing is lost by that,
    /// because nothing is written before there is an account to write it for.
    current: Mutex<Open>,
}

struct Open {
    connection: Connection,
    /// `None` when this is the in-memory database.
    file: Option<PathBuf>,
}

impl Db {
    /// A handle to the databases in `directory`.
    ///
    /// Nothing is read from disk yet: the database is named after the account,
    /// and the account is not known until the stored session has been read. Use
    /// [`Db::open_account`] once it is.
    pub fn new(directory: impl Into<PathBuf>) -> Result<Self, AppError> {
        Ok(Self {
            inner: Arc::new(Inner {
                directory: directory.into(),
                current: Mutex::new(Open {
                    connection: in_memory()?,
                    file: None,
                }),
            }),
        })
    }

    /// An in-memory database, for tests and for a session that never reaches the
    /// disk.
    pub fn open_in_memory() -> Result<Self, AppError> {
        Self::new(PathBuf::new())
    }

    /// Opens `login`'s database and makes it the one this handle reads.
    ///
    /// Opening it is also what claims it: the file is named for the account, and
    /// [`cache::claim_for_account`] cross-checks the name recorded inside it.
    /// A file whose contents claim a different account has those contents
    /// discarded rather than served under the wrong name — the same rule `0.5.5`
    /// applies to an unowned cache.
    ///
    /// Returns early when that account is already open, so callers that run on
    /// every validation cost nothing.
    pub fn open_account(&self, login: &str) -> Result<(), AppError> {
        let normalized = accounts::normalize(login);

        if normalized.is_empty() {
            return Err(AppError::Config(
                "cannot open a database for an account with no name".to_owned(),
            ));
        }

        let file = self
            .inner
            .directory
            .join(format!("gitpulse-{normalized}.db"));

        {
            let current = lock(&self.inner.current)?;
            if current.file.as_deref() == Some(file.as_path()) {
                return Ok(());
            }
        }

        file_legacy_database(&self.inner.directory)?;

        let connection = open_file(&file, &normalized)?;

        let mut current = lock(&self.inner.current)?;
        *current = Open {
            connection,
            file: Some(file),
        };

        tracing::info!(account = %normalized, "opened the account's database");
        Ok(())
    }

    /// Closes the account's database, leaving the file where it is.
    pub fn close(&self) -> Result<(), AppError> {
        self.read_from_memory()
    }

    /// Closes the account's database and deletes its file.
    ///
    /// This is what signing out does. The cached repositories were fetched with
    /// the token that just went away, so they are not this account's to keep —
    /// and with one file per account the file *is* the account, so removing it is
    /// the whole of the operation.
    pub fn forget_current(&self) -> Result<(), AppError> {
        let file = lock(&self.inner.current)?.file.clone();

        let Some(file) = file else {
            return Ok(());
        };

        // Drop the connection before deleting: Windows will not remove a file
        // that is still open.
        self.read_from_memory()?;
        remove_file_and_siblings(&file)?;

        tracing::info!(file = %file.display(), "deleted the account's cache on sign-out");
        Ok(())
    }

    /// The file backing the current database, or `None` while it is in memory.
    #[must_use]
    pub fn path(&self) -> Option<PathBuf> {
        lock(&self.inner.current)
            .ok()
            .and_then(|open| open.file.clone())
    }

    fn read_from_memory(&self) -> Result<(), AppError> {
        let connection = in_memory()?;
        let mut current = lock(&self.inner.current)?;
        *current = Open {
            connection,
            file: None,
        };
        Ok(())
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
            let current = lock(&inner.current)?;
            operation(&current.connection)
        })
        .await
        .map_err(|error| AppError::Internal(format!("database worker failed: {error}")))?
    }

    /// Runs a closure on the calling thread.
    ///
    /// For the two paths that cannot await: startup (before the async runtime
    /// owns the connection) and sign-out (a user action that must finish before
    /// the signed-out state is published). Each is one short query, and neither
    /// races the poller.
    pub fn with_connection_blocking<T, F>(&self, operation: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError>,
    {
        let current = lock(&self.inner.current)?;
        operation(&current.connection)
    }
}

/// Opens one account's file: directory, pragmas, migrations, and the claim.
fn open_file(file: &Path, login: &str) -> Result<Connection, AppError> {
    if let Some(parent) = file.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                AppError::Config(format!(
                    "cannot create data directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
    }

    let connection = Connection::open(file).map_err(AppError::db)?;
    let connection = prepare(connection)?;

    if cache::claim_for_account(&connection, login)? {
        tracing::warn!(
            account = %login,
            "the database held another account's data; it has been discarded"
        );
    }

    Ok(connection)
}

/// Files the database written before there was one per account.
///
/// `gitpulse.db` recorded its owner in `meta.account_login` from `0.5.5`
/// onwards, so it can be renamed under that account's name instead of being
/// thrown away — a rename, not a migration, because the rows are already the
/// right account's. A file with no recorded owner predates that bookkeeping and
/// cannot be attributed to anyone, so it is discarded: the rule `0.5.5` already
/// applies to an unowned cache.
fn file_legacy_database(directory: &Path) -> Result<(), AppError> {
    let legacy = directory.join(LEGACY_FILE);

    if !legacy.is_file() {
        return Ok(());
    }

    let owner = read_owner(&legacy)?;

    let Some(owner) = owner else {
        remove_file_and_siblings(&legacy)?;
        tracing::info!("discarded a pre-accounts cache that recorded no owner");
        return Ok(());
    };

    let target = directory.join(format!("gitpulse-{}.db", accounts::normalize(&owner)));

    if target.exists() {
        // An earlier run already filed this account's data, so the leftover copy
        // has nothing to add.
        remove_file_and_siblings(&legacy)?;
        tracing::info!(account = %owner, "discarded a duplicate pre-accounts database");
        return Ok(());
    }

    std::fs::rename(&legacy, &target).map_err(|error| {
        AppError::Config(format!(
            "cannot file the pre-accounts database under {owner}: {error}"
        ))
    })?;

    tracing::info!(
        account = %owner,
        file = %target.display(),
        "filed the pre-accounts database under its owner"
    );

    Ok(())
}

/// The account a database records, read through a connection that is dropped
/// again immediately.
fn read_owner(path: &Path) -> Result<Option<String>, AppError> {
    let connection = Connection::open(path).map_err(AppError::db)?;
    let owner = cache::account_owner(&connection)?;

    // Move anything still in the write-ahead log into the file itself, so that
    // renaming the file does not leave its most recent writes behind.
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(AppError::db)?;

    Ok(owner)
}

fn prepare(connection: Connection) -> Result<Connection, AppError> {
    connection.execute_batch(PRAGMAS).map_err(AppError::db)?;
    migrate::apply(&connection)?;
    Ok(connection)
}

fn in_memory() -> Result<Connection, AppError> {
    let connection = Connection::open_in_memory().map_err(AppError::db)?;
    prepare(connection)
}

/// Deletes a database along with the write-ahead log and shared-memory file
/// beside it, which is where the most recent writes live until a checkpoint.
fn remove_file_and_siblings(file: &Path) -> Result<(), AppError> {
    for candidate in [
        file.to_path_buf(),
        sibling(file, "-wal"),
        sibling(file, "-shm"),
    ] {
        match std::fs::remove_file(&candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AppError::Config(format!(
                    "cannot delete {}: {error}",
                    candidate.display()
                )))
            }
        }
    }

    Ok(())
}

fn sibling(file: &Path, suffix: &str) -> PathBuf {
    let mut name = file.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn lock(current: &Mutex<Open>) -> Result<MutexGuard<'_, Open>, AppError> {
    current.lock().map_err(|_| {
        AppError::Internal("the database connection was poisoned by an earlier panic".to_owned())
    })
}

/// Drops everything the poller fetched, keeping the repositories themselves.
///
/// "Clear cache" is about the *activity* — runs, jobs, deployments, pull
/// requests — because that is what goes stale. The repositories are the user's
/// choices, so they stay, and the next sweep refetches the rest.
///
/// This used to live in the deliveries module, which it never really belonged to:
/// it cleared the cache and merely also deleted webhook deliveries.
pub fn clear_cached_activity(connection: &Connection) -> Result<usize, AppError> {
    let transaction = connection.unchecked_transaction().map_err(AppError::db)?;

    let mut removed = 0usize;
    // Children before parents, so this does not depend on cascade behaviour.
    for statement in [
        "DELETE FROM jobs",
        "DELETE FROM workflow_runs",
        "DELETE FROM deployments",
        "DELETE FROM pull_requests",
    ] {
        removed += transaction.execute(statement, []).map_err(AppError::db)?;
    }

    transaction.commit().map_err(AppError::db)?;
    Ok(removed)
}
