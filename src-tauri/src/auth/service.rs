//! Tauri-facing orchestration for sign-in.
//!
//! Everything that reaches for the window, the browser or the event bus lives
//! here; the flow itself is in [`super::device_flow`] and knows nothing about
//! Tauri.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;

use crate::db::{cache, Db};
use crate::error::AppError;

use super::device_flow;
use super::oauth::OAuthClient;
use super::tokens::TokenManager;
use super::types::AuthState;

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

    /// Makes the cache belong to `login`, discarding another account's data.
    ///
    /// The synchronous form is for startup, where there is no async context to
    /// hand the connection to.
    fn claim_account_blocking(&self, login: &str) {
        if let Err(error) = self
            .db
            .with_connection_blocking(|connection| cache::claim_for_account(connection, login))
        {
            tracing::warn!(%error, "could not reconcile the cached account");
        }
    }

    async fn claim_account(&self, login: &str) -> Result<(), AppError> {
        let login = login.to_owned();
        self.db
            .with_connection(move |connection| cache::claim_for_account(connection, &login))
            .await?;
        Ok(())
    }

    /// Loads a stored session and publishes it, without hitting the network.
    pub fn bootstrap(&self, app: &AppHandle) -> Result<(), AppError> {
        let state = match self.tokens.hydrate()? {
            Some(session) => {
                // Reconciled before the Authorized state is published, so the
                // window can never paint the previous account's repositories.
                self.claim_account_blocking(&session.login);
                AuthState::Authorized { session }
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
                    if let Err(error) = service.claim_account(&session.login).await {
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
        // sign-in may be a different account: those repositories are not this
        // account's to show.
        if let Err(error) = self.db.with_connection_blocking(cache::purge_account_data) {
            tracing::warn!(%error, "could not purge the account cache on sign-out");
        }

        self.publish(app, &AuthState::SignedOut);
        Ok(())
    }

    /// Confirms the session on demand (the UI's "retry" affordance).
    pub async fn refresh_session(&self, app: &AppHandle) -> AuthState {
        match self.tokens.validate().await {
            Ok(session) => {
                if let Err(error) = self.claim_account(&session.login).await {
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

        // A different account signing in must not inherit the previous
        // account's cached repositories.
        if let Err(error) = self.claim_account(&session.login).await {
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
        let state = match error {
            // Cancelling is a user action, not a failure to report.
            AppError::AuthCancelled => AuthState::SignedOut,
            AppError::AuthDenied => AuthState::Denied,
            AppError::AuthExpired => AuthState::Expired,
            other => AuthState::failed(other),
        };

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
