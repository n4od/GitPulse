//! Tauri-facing orchestration for sign-in.
//!
//! Everything that reaches for the window, the browser or the event bus lives
//! here; the flow itself is in [`super::device_flow`] and knows nothing about
//! Tauri.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;

use crate::db::Db;
use crate::error::AppError;

use super::device_flow;
use super::oauth::OAuthClient;
use super::tokens::TokenManager;
use super::types::{AuthSession, AuthState};

/// Event name the frontend subscribes to.
pub const AUTH_STATE_EVENT: &str = "auth://state";

pub struct AuthService {
    oauth: Arc<OAuthClient>,
    tokens: Arc<TokenManager>,
    scopes: Vec<String>,
    db: Db,
    state: Mutex<AuthState>,
    in_flight: Mutex<Option<CancellationToken>>,
}

impl AuthService {
    #[must_use]
    pub fn new(
        oauth: Arc<OAuthClient>,
        tokens: Arc<TokenManager>,
        scopes: Vec<String>,
        db: Db,
    ) -> Self {
        Self {
            oauth,
            tokens,
            scopes,
            db,
            state: Mutex::new(AuthState::SignedOut),
            in_flight: Mutex::new(None),
        }
    }

    #[must_use]
    pub fn state(&self) -> AuthState {
        self.state
            .lock()
            .map_or_else(|_| AuthState::SignedOut, |guard| guard.clone())
    }

    pub fn tokens(&self) -> &Arc<TokenManager> {
        &self.tokens
    }

    /// The accounts this machine has tokens for, so the UI can offer them.
    pub fn known_logins(&self) -> Result<Vec<String>, AppError> {
        self.tokens.known_logins()
    }

    /// Switches to an account that is already signed in on this machine.
    ///
    /// No device flow: its tokens are in the keychain. The order is the point —
    /// the tokens are activated first, then *that* account's database is opened,
    /// and only then is the new state published — so the window cannot paint a
    /// session whose data is still the previous account's.
    pub async fn switch_account(
        &self,
        app: &AppHandle,
        login: &str,
    ) -> Result<AuthSession, AppError> {
        // Anything half-finished for the account being left is not for this one.
        self.cancel_in_flight();

        let session = self.tokens.activate(login).await?;

        self.ensure_database(&session.login).await?;

        self.publish(
            app,
            &AuthState::Authorized {
                session: session.clone(),
            },
        );

        tracing::info!(login = %session.login, "switched account");
        Ok(session)
    }

    /// Opens the account's database.
    ///
    /// Opening it is also what claims it: the file is named for the account, and
    /// the name recorded inside it is cross-checked before any of it is served.
    /// Synchronous for startup, where there is no async context to hand the file
    /// open and the migrations to.
    fn open_database(&self, login: &str) -> Result<(), AppError> {
        self.db.open_account(login)
    }

    /// Opens the account's database from a context that must not block.
    ///
    /// Cheap to call repeatedly — `open_account` returns immediately when that
    /// account is the one already open — which is why the paths that run on every
    /// validation can call it without thinking about it.
    async fn ensure_database(&self, login: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let login = login.to_owned();

        tokio::task::spawn_blocking(move || db.open_account(&login))
            .await
            .map_err(|error| AppError::Internal(format!("database worker failed: {error}")))?
    }

    /// Loads a stored session and publishes it, without hitting the network.
    pub fn bootstrap(&self, app: &AppHandle) -> Result<(), AppError> {
        let state = match self.tokens.hydrate()? {
            Some(session) => {
                // Opened — and claimed — before the Authorized state is
                // published, so the window can never paint another account's
                // repositories, and never reads from the wrong file.
                match self.open_database(&session.login) {
                    Ok(()) => AuthState::Authorized { session },
                    Err(error) => {
                        // Better to ask for a sign-in than to present a session
                        // whose data cannot be read.
                        tracing::error!(%error, "could not open the account's database");
                        AuthState::SignedOut
                    }
                }
            }
            None => AuthState::SignedOut,
        };

        self.publish(app, &state);
        Ok(())
    }

    /// Confirms the stored token still works. Runs off the critical path so a
    /// slow network cannot delay first paint.
    pub fn validate_in_background(self: &Arc<Self>, app: AppHandle) {
        if self.tokens.session().is_none() {
            return;
        }

        let service = Arc::clone(self);

        tauri::async_runtime::spawn(async move {
            match service.tokens.validate().await {
                Ok(session) => {
                    if let Err(error) = service.ensure_database(&session.login).await {
                        tracing::warn!(%error, "could not reconcile the cached account");
                    }
                    service.publish(&app, &AuthState::Authorized { session });
                }
                Err(AppError::AuthRequired) => {
                    tracing::info!("stored token is no longer valid");
                    service.publish(&app, &AuthState::SignedOut);
                }
                // A network blip is not evidence that the token is bad, so the
                // session stays as it is and the poller will retry.
                Err(error) => tracing::warn!(%error, "could not validate the stored token"),
            }
        });
    }

    /// Starts the device flow. Returns immediately; progress arrives as
    /// `auth://state` events, because the user may take minutes to approve.
    pub fn start(self: &Arc<Self>, app: AppHandle) -> Result<(), AppError> {
        self.cancel_in_flight();

        let cancel = CancellationToken::new();
        if let Ok(mut guard) = self.in_flight.lock() {
            *guard = Some(cancel.clone());
        }

        let service = Arc::clone(self);

        tauri::async_runtime::spawn(async move {
            let outcome = service.run_flow(&app, &cancel).await;

            if let Ok(mut guard) = service.in_flight.lock() {
                *guard = None;
            }

            if let Err(error) = outcome {
                service.publish_failure(&app, &error);
            }
        });

        Ok(())
    }

    pub fn cancel(&self) {
        self.cancel_in_flight();
    }

    pub fn sign_out(&self, app: &AppHandle) -> Result<(), AppError> {
        self.cancel_in_flight();
        self.tokens.sign_out()?;

        // The cache was fetched with the token that just went away, and the next
        // sign-in may be a different account. With one file per account, signing
        // out deletes this account's file and leaves the others where they are.
        if let Err(error) = self.db.forget_current() {
            tracing::warn!(%error, "could not delete the account's cache on sign-out");
        }

        self.publish(app, &AuthState::SignedOut);
        Ok(())
    }

    /// Confirms the session on demand (the UI's "retry" affordance).
    pub async fn refresh_session(&self, app: &AppHandle) -> AuthState {
        match self.tokens.validate().await {
            Ok(session) => {
                if let Err(error) = self.ensure_database(&session.login).await {
                    tracing::warn!(%error, "could not reconcile the cached account");
                }
                AuthState::Authorized { session }
            }
            Err(error) => {
                if matches!(error, AppError::AuthRequired) {
                    self.tokens.sign_out().ok();
                }
                self.publish_failure(app, &error);
                self.state()
            }
        }
    }

    async fn run_flow(&self, app: &AppHandle, cancel: &CancellationToken) -> Result<(), AppError> {
        let publisher = Arc::new(Publisher {
            app: app.clone(),
            state: Mutex::new(self.state()),
        });

        let on_state = {
            let publisher = Arc::clone(&publisher);
            move |state: AuthState| publisher.emit(&state)
        };

        let open_browser = {
            let app = app.clone();
            move |url: &str| open_in_browser(&app, url)
        };

        let tokens =
            device_flow::run(&self.oauth, &self.scopes, cancel, on_state, open_browser).await?;

        let viewer = self.oauth.fetch_viewer(&tokens.access_token).await?;
        let session = self.tokens.adopt(&tokens, &viewer)?;

        // A different account signing in must not inherit the previous account's
        // cached repositories, and this is where the switch happens: the database
        // is opened under the new account's name.
        if let Err(error) = self.ensure_database(&session.login).await {
            tracing::warn!(%error, "could not reconcile the cached account");
        }

        self.publish(app, &AuthState::Authorized { session });

        Ok(())
    }

    fn cancel_in_flight(&self) {
        if let Ok(mut guard) = self.in_flight.lock() {
            if let Some(token) = guard.take() {
                token.cancel();
            }
        }
    }

    fn publish_failure(&self, app: &AppHandle, error: &AppError) {
        let state = failure_state(error, self.tokens.session().as_ref());
        self.publish(app, &state);
    }

    fn publish(&self, app: &AppHandle, state: &AuthState) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = state.clone();
        }

        if let Err(error) = app.emit(AUTH_STATE_EVENT, state) {
            tracing::warn!(%error, "could not emit the auth state event");
        }
    }
}

/// Lets the state-transition callback own its own bookkeeping without holding
/// `AuthService`'s lock across an emit.
struct Publisher {
    app: AppHandle,
    state: Mutex<AuthState>,
}

impl Publisher {
    fn emit(&self, state: &AuthState) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = state.clone();
        }

        if let Err(error) = self.app.emit(AUTH_STATE_EVENT, state) {
            tracing::warn!(%error, "could not emit the auth state event");
        }
    }
}

fn open_in_browser(app: &AppHandle, url: &str) -> Result<(), AppError> {
    app.opener()
        .open_url(url.to_owned(), None::<&str>)
        .map_err(|error| AppError::Internal(format!("cannot open the browser: {error}")))
}

/// What to publish when a sign-in attempt ends without a new session.
///
/// Signing in *again* is how a second account is added, so a flow that is
/// cancelled, refused or expires must return to the session that is already
/// there rather than signing someone out of it. Anything else is a real failure
/// and is reported as one even though the existing session survives — hiding it
/// would leave the user wondering why the attempt did nothing.
fn failure_state(error: &AppError, existing: Option<&AuthSession>) -> AuthState {
    let ended_early = matches!(
        error,
        AppError::AuthCancelled | AppError::AuthDenied | AppError::AuthExpired
    );

    if ended_early {
        if let Some(session) = existing {
            return AuthState::Authorized {
                session: session.clone(),
            };
        }
    }

    match error {
        // Cancelling is a user action, not a failure to report.
        AppError::AuthCancelled => AuthState::SignedOut,
        AppError::AuthDenied => AuthState::Denied,
        AppError::AuthExpired => AuthState::Expired,
        other => AuthState::failed(other),
    }
}

#[cfg(test)]
mod tests {
    use super::failure_state;
    use crate::auth::types::{AuthSession, AuthState};
    use crate::error::AppError;

    fn session(login: &str) -> AuthSession {
        AuthSession {
            login: login.to_owned(),
            avatar_url: None,
            display_name: None,
            expires_at: None,
            scopes: Vec::new(),
            can_refresh: true,
        }
    }

    #[test]
    fn a_cancelled_flow_returns_to_the_account_that_is_still_signed_in() {
        // Adding a second account means signing in again. Cancelling that must
        // not sign out of the first one.
        let existing = session("n4od");

        let state = failure_state(&AppError::AuthCancelled, Some(&existing));

        assert!(
            matches!(&state, AuthState::Authorized { session } if session.login == "n4od"),
            "got {state:?}"
        );
    }

    #[test]
    fn a_cancelled_first_sign_in_is_still_signed_out() {
        let state = failure_state(&AppError::AuthCancelled, None);

        assert!(matches!(state, AuthState::SignedOut), "got {state:?}");
    }

    #[test]
    fn a_refused_or_expired_flow_also_returns_to_the_existing_session() {
        let existing = session("n4od");

        for error in [AppError::AuthDenied, AppError::AuthExpired] {
            let state = failure_state(&error, Some(&existing));

            assert!(
                matches!(state, AuthState::Authorized { .. }),
                "{error:?} gave {state:?}"
            );
        }
    }

    #[test]
    fn a_real_failure_is_reported_even_though_the_session_survives() {
        let existing = session("n4od");
        let error = AppError::network(std::io::Error::other("timed out"), true);

        let state = failure_state(&error, Some(&existing));

        assert!(
            matches!(state, AuthState::Failed { .. }),
            "a network failure is not something to swallow, got {state:?}"
        );
    }
}
