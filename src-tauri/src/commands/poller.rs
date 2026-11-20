//! Poller commands.

use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::poller::Poller;

/// Asks the poller to refresh one repository on its next tick.
///
/// The poller deduplicates, so repeated clicks while a poll is pending collapse
/// into one request.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn repos_refresh(repo_id: i64, poller: State<'_, Arc<Poller>>) -> Result<bool, AppError> {
    if repo_id <= 0 {
        return Err(AppError::Config(
            "repository ids must be positive".to_owned(),
        ));
    }

    poller.request_refresh(repo_id);
    Ok(true)
}
