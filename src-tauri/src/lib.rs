//! `GitPulse` — local-first GitHub workflow monitor.
//!
//! The binary is intentionally thin: [`run`] owns the Tauri builder, every
//! other module is a leaf that can be unit-tested without a window. This file
//! is the composition root: it is the only place that knows about all of the
//! subsystems at once.

#![forbid(unsafe_code)]

pub mod accounts;
pub mod auth;
pub mod commands;
pub mod db;
pub mod error;
pub mod github;
pub mod logging;
pub mod native;
pub mod poller;
pub mod redact;
pub mod secrets;
pub mod state;
pub mod webhook;

#[cfg(test)]
pub mod testkit;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{Emitter, Manager};

use crate::auth::{AuthService, TokenStore};
use crate::native::tray;
use crate::state::AppState;

/// Shell preferences that must be readable from a synchronous event handler.
pub struct ShellState {
    close_to_tray: AtomicBool,
}

/// Starts the desktop application and blocks until the window closes.
pub fn run() {
    let result = tauri::Builder::default()
        // Single-instance must be first: a second launch should focus the
        // existing window before anything else initialises.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(|window, event| {
            let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };

            let close_to_tray = window
                .app_handle()
                .try_state::<ShellState>()
                .is_some_and(|state| state.close_to_tray.load(Ordering::Relaxed));

            if close_to_tray {
                // Closing the window is not quitting: a monitor that exits when
                // its window is dismissed is useless.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(bootstrap)
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_get_state,
            commands::auth::auth_start_device_flow,
            commands::auth::auth_cancel_device_flow,
            commands::auth::auth_sign_out,
            commands::auth::auth_refresh_session,
            commands::shell::native_get_settings,
            commands::shell::native_set_settings,
            commands::diagnostics::system_clear_cache,
            commands::diagnostics::system_metrics,
            commands::poller::repos_refresh,
            commands::repos::repos_available,
            commands::repos::repos_deployments,
            commands::repos::repos_list,
            commands::repos::repos_pulls,
            commands::repos::repos_set_monitored,
            commands::repos::repos_track,
            commands::runs::job_log,
            commands::runs::runs_cancel,
            commands::runs::runs_jobs,
            commands::runs::runs_list,
            commands::runs::runs_rerun,
            commands::runs::runs_rerun_failed,
            commands::system::system_info,
            commands::system::system_open_url,
            commands::webhook::webhook_config,
            commands::webhook::webhook_rotate_secret,
            commands::webhook::webhook_set_transport,
            commands::webhook::webhook_signing_secret,
            commands::webhook::webhook_test_public_url,
            commands::workflows::workflows_dispatch,
            commands::workflows::workflows_list,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        // The tracing subscriber may not exist yet if setup failed early.
        eprintln!("fatal: failed to start GitPulse: {error}");
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
    // A handle, not an open file: the database is named after the account, and
    // the account is not known until the stored session has been read. It starts
    // in memory and `AuthService::bootstrap` opens the right file.
    let database = db::Db::new(data_directory.clone())?;

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

    let auth_service = Arc::new(AuthService::new(
        oauth,
        tokens,
        auth::configured_scopes(),
        database.clone(),
    ));

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        target_os = std::env::consts::OS,
        data_directory = %data_directory.display(),
        client_id_configured = auth::configured_client_id().is_some(),
        "gitpulse starting"
    );

    auth_service.bootstrap(app.handle())?;
    auth_service.validate_in_background(app.handle().clone());

    forward_rate_limit_events(app.handle(), &github_client);

    let poller = Arc::new(poller::Poller::new(
        database.clone(),
        Arc::clone(&github_client),
        Arc::new(TauriPollEvents {
            app: app.handle().clone(),
            database: database.clone(),
        }),
        poller::PollConfig::default(),
    ));

    tauri::async_runtime::spawn({
        let poller = Arc::clone(&poller);
        async move { poller.run().await }
    });

    let notifier = Arc::new(native::Notifier::new(app.handle().clone()));

    // Tray and shell preferences.
    tray::build(app.handle())?;

    let settings =
        tauri::async_runtime::block_on(native::load_settings(&database)).unwrap_or_default();

    app.manage(ShellState {
        close_to_tray: AtomicBool::new(settings.close_to_tray),
    });
    app.manage(Arc::clone(&notifier));

    apply_native_settings(app.handle(), &notifier, &settings);

    let webhooks = Arc::new(webhook::WebhookService::new(
        database.clone(),
        Arc::clone(&poller),
    )?);
    app.manage(Arc::clone(&webhooks));

    tauri::async_runtime::spawn({
        let webhooks = Arc::clone(&webhooks);
        async move {
            if let Err(error) = webhooks.start().await {
                tracing::warn!(%error, "could not start the webhook transport");
            }
        }
    });

    app.manage(database.clone());
    app.manage(auth_service);
    app.manage(Arc::clone(&github_client));
    app.manage(poller);

    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
    }

    // Paints the tray from whatever is already cached, so it is correct before
    // the first poll completes.
    refresh_tray(app.handle());

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

/// Applies preferences that have a side effect outside the app.
fn apply_native_settings(
    app: &tauri::AppHandle,
    notifier: &Arc<native::Notifier>,
    settings: &native::NativeSettings,
) {
    notifier.set_enabled(settings.notifications_enabled);
    notifier.set_quiet_hours(settings.quiet_window());

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;

        let autostart = app.autolaunch();

        let result = if settings.autostart {
            autostart.enable()
        } else {
            autostart.disable()
        };

        if let Err(error) = result {
            tracing::warn!(%error, "could not change the autostart setting");
        }
    }
}

/// Reloads the cache and repaints the tray from it.
fn refresh_tray(app: &tauri::AppHandle) {
    let Some(database) = app.try_state::<db::Db>() else {
        return;
    };

    let signed_in = app
        .try_state::<Arc<AuthService>>()
        .is_some_and(|auth| auth.state().session().is_some());

    let database = database.inner().clone();
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        match database.with_connection(db::repos::cards).await {
            Ok(cards) => tray::refresh(&app, &cards, signed_in),
            Err(error) => tracing::debug!(%error, "could not refresh the tray"),
        }
    });
}

/// Bridges the client's watch channel onto a Tauri event.
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

/// Adapts the poller's output to Tauri events and the native surface.
struct TauriPollEvents {
    app: tauri::AppHandle,
    database: db::Db,
}

impl poller::PollEvents for TauriPollEvents {
    fn repo_updated(&self, card: db::RepoCard) {
        if let Err(error) = self.app.emit(poller::RUNS_UPDATED_EVENT, &card) {
            tracing::warn!(%error, "could not emit run://updated");
        }

        if let Some(notifier) = self.app.try_state::<Arc<native::Notifier>>() {
            notifier.maybe_notify(&card);
        }

        // The tray's icon must not wait for the next status tick.
        refresh_tray(&self.app);
    }

    fn status(&self, status: poller::PollerStatus) {
        if let Err(error) = self.app.emit(poller::POLLER_STATUS_EVENT, &status) {
            tracing::warn!(%error, "could not emit poller://status");
        }

        let _ = &self.database;
    }
}

/// Picks where tokens live.
fn token_store() -> Arc<dyn TokenStore> {
    let keyring = auth::KeyringStore::new();

    match keyring.probe() {
        Ok(()) => Arc::new(keyring),
        Err(error) => {
            tracing::warn!(
                %error,
                "the OS keychain is unavailable; sign-in will not persist beyond this session"
            );
            Arc::new(auth::MemoryStore::new())
        }
    }
}

/// How long the tray's mute item silences notifications.
const _: Duration = Duration::from_secs(60 * 60);
