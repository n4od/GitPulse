//! Webhook configuration commands.

use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::webhook::{TransportKind, WebhookConfig, WebhookService};

#[tauri::command]
pub async fn webhook_config(
    service: State<'_, Arc<WebhookService>>,
) -> Result<WebhookConfig, AppError> {
    service.config().await
}

/// Switches transport and restarts the listener. Takes effect immediately.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn webhook_set_transport(
    service: State<'_, Arc<WebhookService>>,
    kind: TransportKind,
    local_port: Option<u16>,
    relay_url: Option<String>,
) -> Result<WebhookConfig, AppError> {
    let service = Arc::clone(service.inner());
    service.set_transport(kind, local_port, relay_url).await?;
    service.config().await
}

/// Returns the signing secret so it can be pasted into GitHub's webhook form.
///
/// This is the one place a secret crosses the IPC boundary, and only because
/// the user has to type it somewhere else. It never appears in an event.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn webhook_signing_secret(service: State<'_, Arc<WebhookService>>) -> Result<String, AppError> {
    service.signing_secret()
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn webhook_rotate_secret(service: State<'_, Arc<WebhookService>>) -> Result<String, AppError> {
    service.rotate_secret()
}

/// Checks that a public URL actually reaches the local listener.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn webhook_test_public_url(
    service: State<'_, Arc<WebhookService>>,
    url: String,
) -> Result<bool, AppError> {
    service.verify_public_url(&url).await
}
