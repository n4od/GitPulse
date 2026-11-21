//! Account-scoped cache maintenance.
//!
//! The local cache is not "the user's data" in the abstract — it is a copy of
//! what *one account* could see. Repositories are the sharp edge: two accounts
//! can hold different sets of repositories, and a row from the first account is
//! not merely stale under the second, it is wrong.
//!
//! So the cache records which account owns it (`ACCOUNT_KEY`) and is discarded
//! whenever that account changes.

use rusqlite::Connection;

use crate::error::AppError;

use super::{meta, repos};

/// `meta` key holding the login that owns the cached data.
pub const ACCOUNT_KEY: &str = "account_login";

/// Deletes every row that belongs to the signed-in account.
///
/// Repositories go too, unlike [`super::deliveries::clear_cache`], which keeps
/// them because within one account they are the user's choices. Here the
/// account itself may be changing, so keeping them would show the next account
/// repositories it may not even be able to see.
///
/// Settings live outside this database and are untouched. Returns the number of
/// rows removed across all tables.
pub fn purge_account_data(connection: &Connection) -> Result<usize, AppError> {
    let transaction = connection.unchecked_transaction().map_err(AppError::db)?;

    let mut removed = 0usize;
    // Children before parents, so this does not depend on cascade behaviour.
    for statement in [
        "DELETE FROM jobs",
        "DELETE FROM workflow_runs",
        "DELETE FROM deployments",
        "DELETE FROM pull_requests",
        "DELETE FROM webhook_deliveries",
        "DELETE FROM repos",
    ] {
        removed += transaction.execute(statement, []).map_err(AppError::db)?;
    }

    transaction.commit().map_err(AppError::db)?;
    meta::remove(connection, ACCOUNT_KEY)?;

    Ok(removed)
}

/// Records which account owns the cache, purging first when it cannot be trusted.
///
/// The cache is discarded in two cases:
///
/// * the recorded owner is a different account, and
/// * there is **no** recorded owner but rows exist. Those rows were written
///   before this bookkeeping did, so they cannot be attributed to whoever is
///   signing in now — and showing them is the bug this exists to prevent. This
///   is the upgrade path: the first launch after the fix clears whatever a
///   previous account left behind.
///
/// An empty, unowned cache is not a case at all: there is nothing to discard, so
/// the owner is simply recorded.
///
/// Returns `true` when the previous data was purged.
pub fn claim_for_account(connection: &Connection, login: &str) -> Result<bool, AppError> {
    let owner = meta::get(connection, ACCOUNT_KEY)?;
    let same_account = owner
        .as_deref()
        .is_some_and(|stored| stored.eq_ignore_ascii_case(login));

    let unowned = owner.is_none() && repos::count(connection)? > 0;
    let purged = !same_account && (owner.is_some() || unowned);

    if purged {
        purge_account_data(connection)?;
    }

    meta::set(connection, ACCOUNT_KEY, login)?;

    Ok(purged)
}
