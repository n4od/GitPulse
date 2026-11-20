//! Authentication commands.

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::auth::service::AuthService;
use crate::auth::AuthState;
use crate::error::AppError;

#[tauri::command]
#[must_use]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn auth_get_state(auth: State<'_, Arc<AuthService>>) -> AuthState {
    auth.state()
}

/// Begins the device flow. Returns as soon as the flow is running; the UI
/// follows `auth://state` for progress.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn auth_start_device_flow(
    app: AppHandle,
    auth: State<'_, Arc<AuthService>>,
) -> Result<(), AppError> {
    auth.inner().start(app)
}

#[tauri::command]
#[must_use]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn auth_cancel_device_flow(auth: State<'_, Arc<AuthService>>) -> bool {
    auth.cancel();
    true
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn auth_sign_out(app: AppHandle, auth: State<'_, Arc<AuthService>>) -> Result<(), AppError> {
    auth.sign_out(&app)
}

/// Re-checks the session and refreshes the token if it is close to expiry.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn auth_refresh_session(
    app: AppHandle,
    auth: State<'_, Arc<AuthService>>,
) -> Result<AuthState, AppError> {
    Ok(auth.refresh_session(&app).await)
}
