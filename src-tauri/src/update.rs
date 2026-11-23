//! In-app updates: notice a newer release, and install it when asked.
//!
//! A check is one HTTPS request for a small JSON manifest. Nothing is
//! downloaded until the user asks, and nothing is installed until the download
//! matches the public key compiled into the binary. So the network behaviour is
//! small and the user is always the one who decides.
//!
//! That key is the point of the whole feature. `GitPulse`'s installers are
//! *unsigned* - there is no Authenticode or Apple certificate behind them - so
//! the only thing standing between a user and a tampered update is the minisign
//! signature the updater checks against `plugins.updater.pubkey` in
//! `tauri.conf.json`. The private half of that keypair is a secret and lives
//! only in the release workflow; this module never sees it, and neither does the
//! repository.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use ts_rs::TS;

use crate::error::AppError;

/// Emitted when a check finds a newer release.
///
/// The check runs here, at startup, because the setting that gates it is read
/// during bootstrap - long before the window is ready to ask. This event is how
/// the window finds out.
pub const UPDATE_AVAILABLE_EVENT: &str = "update://available";

/// Emitted while an install is running: once as it starts, once per chunk, and
/// once when the bytes are all in and the installer takes over.
///
/// This exists because the moment before the window disappears had no feedback
/// at all. Pressing "Install and restart" changed one line of small text and
/// removed the button, which is what a click that did nothing looks like - and
/// then the app was gone. A 4.5 MB download is quick on a good connection, but
/// it is not instant, and the install that follows it is slower still.
pub const UPDATE_PROGRESS_EVENT: &str = "update://progress";

/// Which part of an install is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum UpdateStage {
    /// Re-reading the manifest, before a byte has been fetched.
    ///
    /// An install re-checks first, so a click can fail with nothing downloaded -
    /// which is precisely when a label saying "downloading" would be wrong.
    Preparing,
    /// Bytes are still arriving.
    Downloading,
    /// The bytes are in, and the installer has taken over from here.
    Installing,
}

/// How far an install has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct UpdateProgress {
    /// Bytes received so far.
    #[ts(type = "number")]
    pub downloaded: u64,
    /// The size the server declared, when it declared one. `None` is *unknown*,
    /// not zero, and the UI shows an indeterminate bar rather than a full or
    /// empty one - a bar that lies about being finished is worse than no bar.
    ///
    /// The type is spelled out because `#[ts(type = ...)]` replaces the whole
    /// annotation: applied to an `Option`, it drops the `| null` and the UI
    /// would have no way to tell "unknown" from a number.
    #[ts(type = "number | null")]
    pub total: Option<u64>,
    pub stage: UpdateStage,
}

/// A running byte count for one download.
///
/// `on_chunk` is `FnMut` and `on_download_finish` is a separate `FnOnce`, so
/// neither may borrow the counter from the other - the atomic is what lets both
/// read it without the borrow checker refusing the pair.
#[derive(Default)]
struct Received(AtomicU64);

impl Received {
    /// Adds one streamed chunk and returns the new running total.
    fn add(&self, chunk: usize) -> u64 {
        // A chunk length cannot approach u64::MAX; this is a widening cast.
        let chunk = u64::try_from(chunk).unwrap_or(u64::MAX);
        self.0.fetch_add(chunk, Ordering::Relaxed) + chunk
    }

    fn bytes(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// A newer release the updater knows about.
///
/// `None` from [`check`] is not an error - it means the running version is the
/// newest published one, which is the ordinary case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct UpdateInfo {
    pub version: String,
    /// The manifest's notes, which are the body of the GitHub release.
    pub notes: Option<String>,
    /// The release's own `pub_date`, passed through unchanged.
    pub date: Option<String>,
}

/// The most recent check result.
///
/// Kept in state because the two halves of a launch race each other: the check
/// starts during bootstrap and needs a network round trip, while the webview is
/// still loading its bundle. The frontend subscribes to
/// [`UPDATE_AVAILABLE_EVENT`] and *then* reads this, so a result that arrived in
/// between is not lost - the same subscribe-then-fetch ordering `useAuthBridge`
/// uses for auth state.
#[derive(Default)]
pub struct LastCheck {
    available: Mutex<Option<UpdateInfo>>,
}

impl LastCheck {
    /// What the last finished check found.
    ///
    /// `None` covers both "nothing newer" and "no check has finished yet". The
    /// UI does not need to tell those apart: either way there is nothing to
    /// offer the user.
    #[must_use]
    pub fn current(&self) -> Option<UpdateInfo> {
        self.available.lock().ok().and_then(|guard| guard.clone())
    }

    fn record(&self, found: Option<UpdateInfo>) {
        if let Ok(mut guard) = self.available.lock() {
            *guard = found;
        }
    }
}

/// Asks the configured endpoint whether a newer release exists.
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, AppError> {
    let update = app
        .updater()
        .map_err(|error| {
            let retryable = is_transient(&error);
            AppError::update_check(error, retryable)
        })?
        .check()
        .await
        .map_err(|error| {
            let retryable = is_transient(&error);
            AppError::update_check(error, retryable)
        })?;

    let found = update.as_ref().map(describe);

    // Recorded even when the answer is "nothing newer", so that [`pending`]
    // means "the last answer" rather than "the last time something was found".
    if let Some(state) = app.try_state::<LastCheck>() {
        state.record(found.clone());
    }

    Ok(found)
}

/// The most recent check result, without making a request.
pub fn pending(app: &AppHandle) -> Option<UpdateInfo> {
    app.try_state::<LastCheck>()
        .and_then(|state| state.current())
}

/// Runs the startup check and tells the window if it finds anything.
///
/// A failure is logged and then dropped. The user did not ask for this check,
/// and every other part of the app tolerates being offline - a monitor that
/// complains at launch about a request nobody made would be the only part that
/// does not.
pub fn spawn_launch_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        match check(&app).await {
            Ok(Some(info)) => {
                if let Err(error) = app.emit(UPDATE_AVAILABLE_EVENT, &info) {
                    tracing::warn!(%error, "could not emit the update event");
                }
            }
            Ok(None) => tracing::debug!("no update available at launch"),
            Err(error) => tracing::debug!(%error, "the launch update check failed"),
        }
    });
}

/// Downloads the pending update, verifies its signature, installs it, restarts.
pub async fn install(app: &AppHandle) -> Result<(), AppError> {
    // Announced first, because everything below can fail without a byte moving:
    // the re-check is a network request, and a click that dies there has to say
    // something other than "downloading".
    emit_progress(
        app,
        UpdateProgress {
            downloaded: 0,
            total: None,
            stage: UpdateStage::Preparing,
        },
    );

    // Checked again rather than remembered from the last check. It is one small
    // request, and re-reading the manifest is what makes a stale UI harmless: if
    // the release moved on, this installs the newer one instead of failing on a
    // signature that no longer matches.
    let update = app
        .updater()
        .map_err(AppError::update_install)?
        .check()
        .await
        .map_err(AppError::update_install)?
        .ok_or_else(|| {
            // A race, not a user error: the button is only offered when a check
            // found something. Reachable if a release is yanked between the two
            // calls, or if the endpoint changed underneath a running app.
            AppError::Config("there is no newer release to install".to_owned())
        })?;

    // The re-check passed, so the download is about to start. Emitted before the
    // first chunk as well, so a fast download still shows the bar rather than
    // nothing at all.
    let received = Arc::new(Received::default());
    let chunks = Arc::clone(&received);
    let finished = Arc::clone(&received);
    let chunk_app = app.clone();
    let finish_app = app.clone();

    emit_progress(
        app,
        UpdateProgress {
            downloaded: 0,
            total: None,
            stage: UpdateStage::Downloading,
        },
    );

    update
        .download_and_install(
            move |chunk_length, total| {
                emit_progress(
                    &chunk_app,
                    UpdateProgress {
                        downloaded: chunks.add(chunk_length),
                        total,
                        stage: UpdateStage::Downloading,
                    },
                );
            },
            move || {
                // The download is complete, so the total the bar was measuring
                // against no longer matters; it is now full by definition.
                let downloaded = finished.bytes();
                emit_progress(
                    &finish_app,
                    UpdateProgress {
                        downloaded,
                        total: Some(downloaded),
                        stage: UpdateStage::Installing,
                    },
                );
            },
        )
        .await
        .map_err(AppError::update_install)?;

    // On Windows this line is never reached: the NSIS installer has taken over
    // and this process is already gone, which is what `download_and_install`
    // promises to do there. Everywhere else nothing restarts the app for us, so
    // a user who pressed "install" would otherwise keep running the build they
    // just replaced.
    app.restart();
}

/// Reports install progress, and does not care whether anyone is listening.
///
/// A failed emit means the window is gone or never existed. The update is
/// already downloading either way, and abandoning it over a progress bar would
/// be the wrong trade.
fn emit_progress(app: &AppHandle, progress: UpdateProgress) {
    if let Err(error) = app.emit(UPDATE_PROGRESS_EVENT, progress) {
        tracing::debug!(%error, "could not emit update progress");
    }
}

/// Trims an [`Update`] down to what the UI shows.
fn describe(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone(),
        date: update
            .raw_json
            .get("pub_date")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

/// Whether the same call could succeed if it were simply repeated.
///
/// The updater's error type is `#[non_exhaustive]`, so this needs a catch-all.
/// The split is *transport* versus *answer*: a request that never arrived is
/// worth asking again for, while an unsupported platform or a signature that
/// does not match is a result, and repeating the call produces it again. The
/// frontend uses this to decide whether to offer a retry at all.
fn is_transient(error: &tauri_plugin_updater::Error) -> bool {
    use tauri_plugin_updater::Error as UpdaterError;

    matches!(
        error,
        UpdaterError::Reqwest(_)
            | UpdaterError::Network(_)
            | UpdaterError::Http(_)
            | UpdaterError::Io(_)
            // "Could not fetch a valid response from the server." Until the
            // first updater-enabled release is *published* the endpoint 404s,
            // and a 5xx is the same shape: ask again later.
            | UpdaterError::ReleaseNotFound
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_updater::Error as UpdaterError;

    #[test]
    fn a_request_that_never_arrived_is_worth_retrying() {
        assert!(is_transient(&UpdaterError::ReleaseNotFound));
        assert!(is_transient(&UpdaterError::Network(
            "connection reset".to_owned()
        )));
        assert!(is_transient(&UpdaterError::Io(std::io::Error::other(
            "no route to host"
        ))));
    }

    #[test]
    fn an_answer_is_not_worth_retrying() {
        // Each of these is the server or the platform *saying something*. Asking
        // again changes nothing, so offering a retry would be a lie.
        assert!(!is_transient(&UpdaterError::UnsupportedArch));
        assert!(!is_transient(&UpdaterError::UnsupportedOs));
        assert!(!is_transient(&UpdaterError::EmptyEndpoints));
        assert!(!is_transient(&UpdaterError::TargetNotFound(
            "windows-x86_64".to_owned()
        )));
    }

    #[test]
    fn the_last_check_remembers_an_answer() {
        let state = LastCheck::default();
        assert_eq!(state.current(), None, "nothing has been checked yet");

        let found = UpdateInfo {
            version: "9.9.9".to_owned(),
            notes: Some("a release".to_owned()),
            date: None,
        };
        state.record(Some(found.clone()));
        assert_eq!(state.current(), Some(found));

        // Recording "nothing newer" must clear it, or a stale offer would
        // survive the release being withdrawn.
        state.record(None);
        assert_eq!(state.current(), None);
    }

    #[test]
    fn received_totals_the_chunks_it_is_given() {
        // The download callback is `FnMut` and gets one slice at a time, so the
        // running total is this type's whole job - and an off-by-one cumulative
        // count is exactly the bug a progress bar shows as never reaching 100%.
        let received = Received::default();
        assert_eq!(received.bytes(), 0);

        assert_eq!(received.add(1024), 1024);
        assert_eq!(received.add(512), 1536);
        assert_eq!(received.add(0), 1536);
        assert_eq!(received.bytes(), 1536);
    }

    #[test]
    fn progress_serialises_the_shape_the_ui_reads() {
        let downloading = serde_json::to_value(UpdateProgress {
            downloaded: 512,
            total: Some(1024),
            stage: UpdateStage::Downloading,
        })
        .expect("serialise");

        assert_eq!(downloading["downloaded"], 512);
        assert_eq!(downloading["total"], 1024);
        assert_eq!(downloading["stage"], "downloading");

        // A server that declares no length must arrive as `null`, not 0: the UI
        // tells "unknown" from "nothing yet" by it, and a bar that reads zero
        // forever looks broken.
        let unknown = serde_json::to_value(UpdateProgress {
            downloaded: 0,
            total: None,
            stage: UpdateStage::Installing,
        })
        .expect("serialise");

        assert!(unknown["total"].is_null());
        assert_eq!(unknown["stage"], "installing");

        // The frontend seeds its own "preparing" state before the backend says
        // anything, so these three strings are a contract: they are written by
        // hand in `src/stores/update.ts` and must match the enum spelled here.
        let preparing = serde_json::to_value(UpdateProgress {
            downloaded: 0,
            total: None,
            stage: UpdateStage::Preparing,
        })
        .expect("serialise");

        assert_eq!(preparing["stage"], "preparing");
        assert_eq!(preparing["downloaded"], 0);
    }
}
