//! System-level commands: build metadata and controlled URL opening.

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub version: &'static str,
    pub tauri_version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub started_at: DateTime<Utc>,
}

#[tauri::command]
#[must_use]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn system_info(state: State<'_, AppState>) -> SystemInfo {
    SystemInfo {
        version: env!("CARGO_PKG_VERSION"),
        tauri_version: tauri::VERSION,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        started_at: state.started_at(),
    }
}

/// Opens a GitHub URL in the user's default browser.
///
/// The frontend is not trusted to decide what may be launched: only `https`
/// URLs on `github.com` (or a subdomain) are accepted, which prevents the
/// webview from being used as a generic protocol launcher.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn system_open_url(app: AppHandle, url: String) -> Result<(), AppError> {
    let parsed = url::Url::parse(&url)
        .map_err(|_| AppError::Config("the supplied URL is not valid".to_owned()))?;

    if parsed.scheme() != "https" {
        return Err(AppError::Config("only https URLs may be opened".to_owned()));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::Config("the supplied URL has no host".to_owned()))?;

    if !(host == "github.com" || host.ends_with(".github.com")) {
        return Err(AppError::Config(format!("opening {host} is not permitted")));
    }

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| AppError::Internal(format!("cannot open the browser: {error}")))
}
