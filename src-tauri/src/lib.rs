//! Pulse — local-first GitHub workflow monitor.
//!
//! The binary is intentionally thin: [`run`] owns the Tauri builder, every
//! other module is a leaf that can be unit-tested without a window.

#![forbid(unsafe_code)]

mod commands;
mod error;
mod logging;
mod redact;
mod state;

use tauri::Manager;

use crate::state::AppState;

/// Starts the desktop application and blocks until the window closes.
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            logging::init(app.handle())?;
            app.manage(AppState::new());

            tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                target_os = std::env::consts::OS,
                "pulse starting"
            );

            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }

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
