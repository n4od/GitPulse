//! Token custody: the in-memory cache, expiry policy, and refresh.
//!
//! The whole point of this module is one invariant: **at most one refresh is in
//! flight at a time**. GitHub rotates refresh tokens, so two concurrent
//! refreshes would leave one of them holding a token that the other has already
//! invalidated — the user would be signed out by a race.

use std::sync::{Arc, Mutex as StdMutex, MutexGuard};

use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};

use crate::error::AppError;

use super::oauth::{OAuthClient, TokenResponse, Viewer, EXPIRY_SKEW_SECS};
use super::token_store::{secret, StoredTokens, TokenMetadata, TokenStore};
use super::types::AuthSession;

/// Lets the HTTP client ask for a token without importing the auth internals.
#[async_trait::async_trait]
impl crate::github::api::TokenProvider for TokenManager {
    async fn access_token(&self) -> Result<SecretString, AppError> {
        TokenManager::access_token(self).await
    }
}

struct SessionState {
    access_token: SecretString,
    refresh_token: Option<SecretString>,
    expires_at: Option<DateTime<Utc>>,
    metadata: TokenMetadata,
    session: AuthSession,
}

pub struct TokenManager {
    oauth: Arc<OAuthClient>,
    store: Arc<dyn TokenStore>,
    state: StdMutex<Option<SessionState>>,
    /// Held across a refresh so concurrent callers queue behind one request.
    refresh_gate: tokio::sync::Mutex<()>,
}

impl TokenManager {
    #[must_use]
    pub fn new(oauth: Arc<OAuthClient>, store: Arc<dyn TokenStore>) -> Self {
        Self {
            oauth,
            store,
            state: StdMutex::new(None),
            refresh_gate: tokio::sync::Mutex::new(()),
        }
    }

    /// Loads the active account's stored session into memory. Called once at
    /// startup.
    ///
    /// The active pointer is what makes this unambiguous now that more than one
    /// account's tokens can be on the machine.
    pub fn hydrate(&self) -> Result<Option<AuthSession>, AppError> {
        let Some(login) = self.store.active()? else {
            return Ok(None);
        };

        let Some(stored) = self.store.load(&login)? else {
            return Ok(None);
        };

        let session = Self::session_from(&stored.metadata, stored.refresh_token.is_some(), None);

        let mut guard = self.lock()?;
        *guard = Some(SessionState {
            access_token: stored.access_token,
            refresh_token: stored.refresh_token,
            expires_at: stored.metadata.expires_at,
            metadata: stored.metadata,
            session: session.clone(),
        });

        Ok(Some(session))
    }

    #[must_use]
    pub fn session(&self) -> Option<AuthSession> {
        self.lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|state| state.session.clone()))
    }

    /// The account the in-memory session belongs to, if there is one.
    fn login(&self) -> Option<String> {
        self.lock().ok().and_then(|guard| {
            guard
                .as_ref()
                .and_then(|state| state.metadata.login.clone())
        })
    }

    /// Returns a usable access token, refreshing first if it is expired or
    /// about to expire.
    pub async fn access_token(&self) -> Result<SecretString, AppError> {
        if let Some(token) = self.fresh_token()? {
            return Ok(token);
        }

        let _gate = self.refresh_gate.lock().await;

        // Another caller may have refreshed while we waited for the gate. This
        // re-check is what collapses N concurrent callers into one request.
        if let Some(token) = self.fresh_token()? {
            return Ok(token);
        }

        self.refresh().await
    }

    /// Completes a first sign-in: persists the grant and records the identity.
    pub fn adopt(&self, tokens: &TokenResponse, viewer: &Viewer) -> Result<AuthSession, AppError> {
        let metadata = TokenMetadata {
            login: Some(viewer.login.clone()),
            avatar_url: viewer.avatar_url.clone(),
            display_name: viewer.display_name.clone(),
            expires_at: tokens.expires_at,
            refresh_expires_at: tokens.refresh_expires_at,
            scopes: tokens.scopes.clone(),
        };

        self.apply(tokens, metadata)
    }

    /// Confirms the stored token still works, and refreshes the identity.
    ///
    /// A `401` here means the token was revoked or the user signed the app out
    /// on GitHub; the only honest response is to clear it and ask again.
    pub async fn validate(&self) -> Result<AuthSession, AppError> {
        let token = self.access_token().await?;

        match self.oauth.fetch_viewer(token.expose_secret()).await {
            Ok(viewer) => self.update_identity(&viewer),
            Err(AppError::TokenRevoked) => {
                self.sign_out()?;
                Err(AppError::AuthRequired)
            }
            Err(error) => Err(error),
        }
    }

    /// Signs out of the active account, and only that one.
    ///
    /// Other accounts' tokens stay where they are: with one entry per account,
    /// "sign out" is a decision about an account rather than about the machine.
    pub fn sign_out(&self) -> Result<(), AppError> {
        let login = match self.login() {
            Some(login) => Some(login),
            None => self.store.active()?,
        };

        if let Some(login) = login {
            self.store.remove(&login)?;
        }

        let mut guard = self.lock()?;
        *guard = None;
        Ok(())
    }

    /// True when the token is absent or past its expiry margin.
    fn fresh_token(&self) -> Result<Option<SecretString>, AppError> {
        let guard = self.lock()?;
        let Some(state) = guard.as_ref() else {
            return Ok(None);
        };

        let usable = match state.expires_at {
            // No expiry: an OAuth App token, or a GitHub App with token expiry
            // turned off. It is usable until GitHub says otherwise.
            None => true,
            Some(expires_at) => {
                expires_at > Utc::now() + chrono::Duration::seconds(EXPIRY_SKEW_SECS)
            }
        };

        Ok(usable.then(|| state.access_token.clone()))
    }

    async fn refresh(&self) -> Result<SecretString, AppError> {
        let (refresh_token, metadata) = {
            let guard = self.lock()?;
            let state = guard.as_ref().ok_or(AppError::AuthRequired)?;
            let refresh_token = state.refresh_token.clone().ok_or(AppError::AuthRequired)?;
            (refresh_token, state.metadata.clone())
        };

        tracing::info!("refreshing the GitHub access token");
        let tokens = self.oauth.refresh(refresh_token.expose_secret()).await?;

        let session = self.apply(&tokens, metadata)?;
        tracing::info!(login = %session.login, "access token refreshed");

        Ok(secret(tokens.access_token))
    }

    /// Persists a grant and swaps it into memory.
    ///
    /// The refresh token comes from the response and nowhere else. GitHub
    /// rotates refresh tokens, so the one we just spent is already dead;
    /// resurrecting it would only move the failure later.
    fn apply(
        &self,
        tokens: &TokenResponse,
        metadata: TokenMetadata,
    ) -> Result<AuthSession, AppError> {
        let refresh_token = tokens
            .refresh_token
            .as_ref()
            .map(|rotated| secret(rotated.clone()));

        // The account name is part of the key the tokens are stored under, so it
        // cannot be missing here: `adopt` learns it from GitHub and `refresh`
        // carries it forward. Reaching this without one is a programming error,
        // not a state a user can be in.
        let login = metadata
            .login
            .clone()
            .filter(|login| !login.is_empty())
            .ok_or_else(|| {
                AppError::Internal("refusing to store tokens without an account name".to_owned())
            })?;

        let stored = StoredTokens {
            access_token: secret(tokens.access_token.clone()),
            refresh_token: refresh_token.clone(),
            metadata: metadata.clone(),
        };
        self.store.save(&login, &stored)?;

        let effective_metadata = TokenMetadata {
            expires_at: tokens.expires_at,
            refresh_expires_at: tokens.refresh_expires_at.or(metadata.refresh_expires_at),
            ..metadata
        };

        let session = Self::session_from(
            &effective_metadata,
            refresh_token.is_some(),
            tokens.expires_at,
        );

        let mut guard = self.lock()?;
        *guard = Some(SessionState {
            access_token: stored.access_token,
            refresh_token,
            expires_at: tokens.expires_at,
            metadata: effective_metadata,
            session: session.clone(),
        });

        Ok(session)
    }

    fn update_identity(&self, viewer: &Viewer) -> Result<AuthSession, AppError> {
        let (session, metadata, previous) = {
            let mut guard = self.lock()?;
            let state = guard.as_mut().ok_or(AppError::AuthRequired)?;
            let previous = state.metadata.login.clone();

            state.session.login.clone_from(&viewer.login);
            state.session.avatar_url.clone_from(&viewer.avatar_url);
            state.session.display_name.clone_from(&viewer.display_name);

            state.metadata.login = Some(viewer.login.clone());
            state.metadata.avatar_url.clone_from(&viewer.avatar_url);
            state.metadata.display_name.clone_from(&viewer.display_name);

            (state.session.clone(), state.metadata.clone(), previous)
        };

        // A rename on GitHub changes the account's name, and the name is part of
        // the key the tokens live under, so they have to move with it. Leaving
        // them under the old name would strand a working token where nothing
        // looks for it.
        let renamed = previous.filter(|previous| !previous.eq_ignore_ascii_case(&viewer.login));

        if let Some(previous) = renamed {
            if let Some(mut stored) = self.store.load(&previous)? {
                stored.metadata = metadata;
                self.store.remove(&previous)?;
                self.store.save(&viewer.login, &stored)?;
                return Ok(session);
            }
        }

        // Persist so a restart shows the current login rather than a stale one.
        if let Some(mut stored) = self.store.load(&viewer.login)? {
            stored.metadata = metadata;
            self.store.save(&viewer.login, &stored)?;
        }

        Ok(session)
    }

    fn session_from(
        metadata: &TokenMetadata,
        can_refresh: bool,
        expires_at: Option<DateTime<Utc>>,
    ) -> AuthSession {
        AuthSession {
            login: metadata.login.clone().unwrap_or_default(),
            avatar_url: metadata.avatar_url.clone(),
            display_name: metadata.display_name.clone(),
            expires_at: expires_at.or(metadata.expires_at),
            scopes: metadata.scopes.clone(),
            can_refresh,
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Option<SessionState>>, AppError> {
        self.state.lock().map_err(|_| {
            AppError::Internal("the token cache was poisoned by an earlier panic".to_owned())
        })
    }
}
