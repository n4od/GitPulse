//! Desktop shell settings: notifications, quiet hours, autostart, close-to-tray.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use crate::db::Db;
use crate::error::AppError;
use crate::native::{self, NativeSettings, Notifier};
use crate::ShellState;

#[tauri::command]
pub async fn native_get_settings(db: State<'_, Db>) -> Result<NativeSettings, AppError> {
    let db = db.inner().clone();
    native::load_settings(&db).await
}

/// Saves the settings and applies the ones with side effects immediately.
///
/// Nothing here is deferred to a restart: a "mute notifications" checkbox that
/// only takes effect next launch is a bug report waiting to happen.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub async fn native_set_settings(
    app: AppHandle,
    db: State<'_, Db>,
    notifier: State<'_, Arc<Notifier>>,
    settings: NativeSettings,
) -> Result<NativeSettings, AppError> {
    let database = db.inner().clone();
    native::save_settings(&database, &settings).await?;

    notifier.set_enabled(settings.notifications_enabled);
    notifier.set_quiet_hours(settings.quiet_window());

    if let Some(shell) = app.try_state::<ShellState>() {
        shell
            .close_to_tray
            .store(settings.close_to_tray, Ordering::Relaxed);
    }

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;

        let autolaunch = app.autolaunch();
        let result = if settings.autostart {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };

        if let Err(error) = result {
            tracing::warn!(%error, "could not change the autostart setting");
        }
    }

    Ok(settings)
}
