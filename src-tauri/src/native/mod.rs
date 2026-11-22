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
///
/// `default` is on the container, not on the new fields, and that is load-
/// bearing: the whole struct is persisted as one JSON blob, and
/// [`load_settings`] falls back to `Default` when it cannot parse. Without this,
/// adding a field would make every existing blob unparseable and silently reset
/// the user's settings on upgrade. With it, a blob from an older version loads
/// and the fields it does not mention take their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "../../src/lib/bindings/")]
// Four independent on/off preferences, each its own checkbox. The lint is about
// a state machine hiding in a pile of bools; this is not one, and folding them
// into a bitfield or an enum would make the stored blob harder to read without
// making the UI any simpler.
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent preferences, not encoded state"
)]
pub struct NativeSettings {
    pub notifications_enabled: bool,
    /// A window like `"22-07"`, or `None` for "never quiet".
    pub quiet_hours: Option<String>,
    /// Off by default: an app that installs itself into startup uninvited is
    /// malware behaviour, however convenient.
    pub autostart: bool,
    /// Closing the window hides it instead of quitting.
    pub close_to_tray: bool,
    /// Look for a newer release when the app starts.
    ///
    /// On by default, unlike `autostart`. `GitPulse` is already a GitHub client
    /// making requests on a timer, so one more request for a small manifest is
    /// not a new class of network access - and an update nobody is told about is
    /// an update nobody installs. Nothing is downloaded until the user says so.
    pub check_updates_on_launch: bool,
}

impl Default for NativeSettings {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            quiet_hours: None,
            autostart: false,
            close_to_tray: true,
            check_updates_on_launch: true,
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
        // The one that goes the other way: looking for an update is on, because
        // a monitor that never mentions a newer version may as well not have one.
        assert!(settings.check_updates_on_launch);
    }

    #[test]
    fn a_settings_blob_written_before_a_field_existed_still_loads() {
        // Settings are one JSON blob under one meta key, and `load_settings`
        // falls back to `Default` when it cannot parse. So a new *required*
        // field would make every existing blob unparseable and silently reset
        // the user's whole configuration on upgrade - notifications, quiet
        // hours, autostart, everything. This is the test that catches that.
        let older = r#"{
            "notificationsEnabled": false,
            "quietHours": "22-07",
            "autostart": true,
            "closeToTray": false
        }"#;

        let parsed: NativeSettings =
            serde_json::from_str(older).expect("a blob from an older version must still parse");

        // Every value the user set survives.
        assert!(!parsed.notifications_enabled);
        assert_eq!(parsed.quiet_hours.as_deref(), Some("22-07"));
        assert!(parsed.autostart);
        assert!(!parsed.close_to_tray);
        // And the field that did not exist takes its default, not `false`.
        assert!(parsed.check_updates_on_launch);
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
