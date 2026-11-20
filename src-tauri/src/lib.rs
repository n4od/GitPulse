//! Pulse — local-first GitHub workflow monitor.
//!
//! The binary is intentionally thin: [`run`] owns the Tauri builder, every
//! other module is a leaf that can be unit-tested without a window. This file
//! is the composition root: it is the only place that knows about all of the
//! subsystems at once.

#![forbid(unsafe_code)]

pub mod auth;
pub mod commands;
pub mod db;
pub mod error;
pub mod github;
pub mod logging;
pub mod poller;
pub mod redact;
pub mod state;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use crate::auth::TokenStore;
use crate::state::AppState;

/// Starts the desktop application and blocks until the window closes.
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(bootstrap)
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_get_state,
            commands::auth::auth_start_device_flow,
            commands::auth::auth_cancel_device_flow,
            commands::auth::auth_sign_out,
            commands::auth::auth_refresh_session,
            commands::poller::repos_refresh,
            commands::system::system_info,
            commands::system::system_open_url,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        // The tracing subscriber may not exist yet if setup failed early.
        eprintln!("fatal: failed to start Pulse: {error}");
        std::process::exit(1);
    }
}

/// Wires every subsystem together and starts the background tasks.
fn bootstrap(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    logging::init(app.handle())?;
    app.manage(AppState::new());

    let data_directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let database = db::Db::open(data_directory.join("pulse.db"))?;

    let http = github::client::build_client()?;
    let endpoints = github::client::Endpoints::github();

    let oauth = Arc::new(auth::oauth::OAuthClient::new(
        http.clone(),
        endpoints.clone(),
        auth::configured_client_id(),
    ));

    let tokens = Arc::new(auth::TokenManager::new(Arc::clone(&oauth), token_store()));
    let token_provider: Arc<dyn github::TokenProvider> = tokens.clone();

    let github_client = Arc::new(github::GithubClient::new(
        http,
        endpoints,
        token_provider,
        Arc::new(github::RateBudget::for_github()),
    ));

    let auth_service = Arc::new(auth::AuthService::new(
        oauth,
        tokens,
        auth::configured_scopes(),
    ));

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        target_os = std::env::consts::OS,
        database = %database.path().display(),
        client_id_configured = auth::configured_client_id().is_some(),
        "pulse starting"
    );

    auth_service.bootstrap(app.handle())?;
    auth_service.validate_in_background(app.handle().clone());

    forward_rate_limit_events(app.handle(), &github_client);

    let poller = Arc::new(poller::Poller::new(
        database.clone(),
        Arc::clone(&github_client),
        Arc::new(TauriPollEvents {
            app: app.handle().clone(),
        }),
        poller::PollConfig::default(),
    ));

    tauri::async_runtime::spawn({
        let poller = Arc::clone(&poller);
        async move { poller.run().await }
    });

    app.manage(database.clone());
    app.manage(auth_service);
    app.manage(Arc::clone(&github_client));
    app.manage(poller);

    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
    }

    // Retention touches potentially many rows, so it runs after the window is
    // up rather than delaying first paint.
    tauri::async_runtime::spawn(async move {
        match database
            .with_connection(db::retention::prune_with_defaults)
            .await
        {
            Ok(report) => tracing::info!(
                runs = report.runs_deleted,
                deliveries = report.deliveries_deleted,
                vacuumed = report.vacuumed,
                "retention sweep complete"
            ),
            Err(error) => tracing::warn!(%error, "retention sweep failed"),
        }
    });

    Ok(())
}

/// Bridges the client's watch channel onto a Tauri event.
///
/// The webview cannot poll Rust, so the rate-limit picture is pushed. This is
/// the only place that turns a publication into an event.
fn forward_rate_limit_events(handle: &tauri::AppHandle, client: &Arc<github::GithubClient>) {
    let mut rate = client.rate_receiver();
    let handle = handle.clone();

    tauri::async_runtime::spawn(async move {
        while rate.changed().await.is_ok() {
            let snapshot = rate.borrow_and_update().clone();

            if let Err(error) = handle.emit(github::RATE_LIMIT_EVENT, snapshot) {
                tracing::warn!(%error, "could not emit the rate limit event");
            }
        }
    });
}

/// Adapts the poller's output to Tauri events.
///
/// The poller knows nothing about Tauri, which is what lets its tests assert on
/// emissions without a window.
struct TauriPollEvents {
    app: tauri::AppHandle,
}

impl poller::PollEvents for TauriPollEvents {
    fn runs_changed(&self, event: poller::RunsChangedEvent) {
        if let Err(error) = self.app.emit(poller::RUNS_UPDATED_EVENT, event) {
            tracing::warn!(%error, "could not emit run://updated");
        }
    }

    fn status(&self, status: poller::PollerStatus) {
        if let Err(error) = self.app.emit(poller::POLLER_STATUS_EVENT, status) {
            tracing::warn!(%error, "could not emit poller://status");
        }
    }
}

/// Picks where tokens live.
///
/// The OS keychain is used when it works. When it does not — a Linux session
/// with no Secret Service daemon is the common case — the app degrades to
/// session-only storage in memory rather than writing secrets to disk in
/// plaintext. The user simply has to sign in again after a restart, and the
/// reason is logged.
fn token_store() -> Arc<dyn TokenStore> {
    let keyring = auth::KeyringStore;

    match keyring.load() {
        Ok(_) => Arc::new(keyring),
        Err(error) => {
            tracing::warn!(
                %error,
                "the OS keychain is unavailable; sign-in will not persist beyond this session"
            );
            Arc::new(auth::MemoryStore::new())
        }
    }
}
