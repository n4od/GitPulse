//! Native integration: the tray, notifications, autostart and window behaviour.

pub mod icon;
pub mod notifier;
pub mod tray;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{self, Db};
use crate::error::AppError;

pub use icon::TrayState;
pub use notifier::Notifier;

const SETTINGS_KEY: &str = "native_settings";

/// Preferences that only the desktop shell cares about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct NativeSettings {
    pub notifications_enabled: bool,
    /// A window like `"22-07"`, or `None` for "never quiet".
    pub quiet_hours: Option<String>,
    /// Off by default: an app that installs itself into startup uninvited is
    /// malware behaviour, however convenient.
    pub autostart: bool,
    /// Closing the window hides it instead of quitting.
    pub close_to_tray: bool,
}

impl Default for NativeSettings {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            quiet_hours: None,
            autostart: false,
            close_to_tray: true,
        }
    }
}

impl NativeSettings {
    /// Parses `quiet_hours` into `(start, end)`, rejecting anything malformed.
    #[must_use]
    pub fn quiet_window(&self) -> Option<(u8, u8)> {
        let raw = self.quiet_hours.as_deref()?;
        let (start, end) = raw.split_once('-')?;

        let start: u8 = start.trim().parse().ok()?;
        let end: u8 = end.trim().parse().ok()?;

        if start > 23 || end > 23 || start == end {
            return None;
        }

        Some((start, end))
    }
}

pub async fn load_settings(db: &Db) -> Result<NativeSettings, AppError> {
    let raw = db
        .with_connection(|connection| db::meta::get(connection, SETTINGS_KEY))
        .await?;

    Ok(raw
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default())
}

pub async fn save_settings(db: &Db, settings: &NativeSettings) -> Result<(), AppError> {
    if let Some(raw) = settings.quiet_hours.as_deref() {
        if settings.quiet_window().is_none() {
            return Err(AppError::Config(format!(
                "quiet hours must look like \"22-07\", got {raw:?}"
            )));
        }
    }

    let json = serde_json::to_string(settings)
        .map_err(|error| AppError::Internal(format!("cannot serialise settings: {error}")))?;

    db.with_connection(move |connection| db::meta::set(connection, SETTINGS_KEY, &json))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_quiet_and_unobtrusive() {
        let settings = NativeSettings::default();

        assert!(settings.notifications_enabled);
        assert!(!settings.autostart, "installing into startup is opt-in");
        assert!(settings.close_to_tray);
        assert_eq!(settings.quiet_hours, None);
    }

    #[test]
    fn a_quiet_window_round_trips() {
        let settings = NativeSettings {
            quiet_hours: Some("22-07".to_owned()),
            ..NativeSettings::default()
        };

        assert_eq!(settings.quiet_window(), Some((22, 7)));
    }

    #[test]
    fn malformed_quiet_windows_are_rejected_not_guessed() {
        for raw in [
            "", "22", "22-", "-07", "24-07", "22-99", "22-22", "ten-five",
        ] {
            let settings = NativeSettings {
                quiet_hours: Some(raw.to_owned()),
                ..NativeSettings::default()
            };

            assert_eq!(settings.quiet_window(), None, "{raw:?} should be rejected");
        }
    }
}
