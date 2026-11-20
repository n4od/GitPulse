//! Pulse — local-first GitHub workflow monitor.
//!
//! The binary is intentionally thin: [`run`] owns the Tauri builder, every
//! other module is a leaf that can be unit-tested without a window.

#![forbid(unsafe_code)]

pub mod commands;
pub mod db;
pub mod error;
pub mod logging;
pub mod redact;
pub mod state;

use tauri::Manager;

use crate::state::AppState;

/// Starts the desktop application and blocks until the window closes.
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            logging::init(app.handle())?;
            app.manage(AppState::new());

            let data_directory = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            let database = db::Db::open(data_directory.join("pulse.db"))?;

            tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                target_os = std::env::consts::OS,
                database = %database.path().display(),
                "pulse starting"
            );

            let maintenance = database.clone();
            app.manage(database);

            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }

            // Retention touches potentially many rows, so it runs after the
            // window is up rather than delaying first paint.
            tauri::async_runtime::spawn(async move {
                let outcome = maintenance
                    .with_connection(db::retention::prune_with_defaults)
                    .await;

                match outcome {
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
        })
        .invoke_handler(tauri::generate_handler![
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
