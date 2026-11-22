//! The update commands: the app's own version, and the update actions.

use tauri::AppHandle;

use crate::error::AppError;
use crate::update::{self, UpdateInfo};

/// The version this build reports.
///
/// Read from the packaged app rather than from `CARGO_PKG_VERSION`, so the
/// number Settings shows is the same one the updater compares against. Those two
/// are compiled from different files - `tauri.conf.json` and `Cargo.toml` - and
/// reading the one the updater actually used is what stops the UI from claiming
/// a version the updater disagrees with.
// Tauri injects command arguments by value - a borrowed handle is not a valid
// command parameter - so this takes ownership and then only borrows it.
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value, not by reference"
)]
#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// `None` means the running version is the newest published one.
#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<Option<UpdateInfo>, AppError> {
    update::check(&app).await
}

/// The result of the last check, without making a request.
///
/// The startup check runs before the window subscribes to its event, so the
/// frontend reads this immediately after subscribing, to catch a result that
/// already landed.
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value, not by reference"
)]
#[tauri::command]
pub fn update_pending(app: AppHandle) -> Option<UpdateInfo> {
    update::pending(&app)
}

/// Downloads and installs the pending update, then restarts. On Windows the
/// installer takes the process down before this returns.
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), AppError> {
    update::install(&app).await
}
