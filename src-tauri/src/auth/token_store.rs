//! Where GitHub tokens live.
//!
//! Tokens go to the OS keychain (Windows Credential Manager, macOS Keychain,
//! Secret Service on Linux) and never to `SQLite`, a config file, or the frontend.
//!
//! The trait exists for two reasons: tests must not touch the developer's real
//! keychain, and a machine with no Secret Service daemon needs a documented
//! fallback rather than a crash.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Keychain service name. Namespaced by bundle identifier so it cannot collide
/// with another app's entries.
const SERVICE: &str = "dev.pulse.workflowmonitor";
const ACCESS_KEY: &str = "github-access-token";
const REFRESH_KEY: &str = "github-refresh-token";
const METADATA_KEY: &str = "github-token-metadata";

/// Everything about a session except the secrets themselves.
///
/// This is stored alongside the tokens rather than in `SQLite` so that signing
/// out is one atomic concept with one owner.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMetadata {
    pub login: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StoredTokens {
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub metadata: TokenMetadata,
}

pub trait TokenStore: Send + Sync + 'static {
    /// `Ok(None)` when nothing is stored. A keychain that exists but has no
    /// entry is not an error.
    fn load(&self) -> Result<Option<StoredTokens>, AppError>;
    fn save(&self, tokens: &StoredTokens) -> Result<(), AppError>;
    fn clear(&self) -> Result<(), AppError>;
}

/// `secrecy` 0.10 wraps a `Box<str>`, which is noisy at every call site.
#[must_use]
pub fn secret(value: impl Into<Box<str>>) -> SecretString {
    SecretString::new(value.into())
}

fn keyring_unavailable(error: &keyring::Error) -> AppError {
    AppError::KeyringUnavailable(error.to_string())
}

/// The OS keychain.
pub struct KeyringStore;

impl KeyringStore {
    fn entry(key: &str) -> Result<keyring::Entry, AppError> {
        keyring::Entry::new(SERVICE, key).map_err(|error| keyring_unavailable(&error))
    }

    fn read(key: &str) -> Result<Option<String>, AppError> {
        match Self::entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(keyring_unavailable(&error)),
        }
    }

    fn write(key: &str, value: &str) -> Result<(), AppError> {
        Self::entry(key)?
            .set_password(value)
            .map_err(|error| keyring_unavailable(&error))
    }

    fn remove(key: &str) -> Result<(), AppError> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(keyring_unavailable(&error)),
        }
    }
}

impl TokenStore for KeyringStore {
    fn load(&self) -> Result<Option<StoredTokens>, AppError> {
        let Some(access_token) = Self::read(ACCESS_KEY)? else {
            return Ok(None);
        };

        let refresh_token = Self::read(REFRESH_KEY)?.map(secret);

        let metadata = match Self::read(METADATA_KEY)? {
            Some(raw) => serde_json::from_str(&raw).unwrap_or_else(|error| {
                // A corrupt metadata blob must not lock the user out: the
                // tokens are still usable, we just re-learn the identity.
                tracing::warn!(%error, "token metadata was unreadable; ignoring it");
                TokenMetadata::default()
            }),
            None => TokenMetadata::default(),
        };

        Ok(Some(StoredTokens {
            access_token: secret(access_token),
            refresh_token,
            metadata,
        }))
    }

    /// Write order is load-bearing.
    ///
    /// GitHub invalidates a refresh token the moment it issues a new one, so
    /// if we crashed having written the access token but not yet the refresh
    /// token, the stored refresh token would already be dead and the user would
    /// have to sign in again. Writing the refresh token first means the worst
    /// case is a stale access token that we can silently renew.
    fn save(&self, tokens: &StoredTokens) -> Result<(), AppError> {
        if let Some(refresh_token) = &tokens.refresh_token {
            Self::write(REFRESH_KEY, refresh_token.expose_secret())?;
        }

        let metadata = serde_json::to_string(&tokens.metadata).map_err(|error| {
            AppError::Internal(format!("cannot serialise token metadata: {error}"))
        })?;
        Self::write(METADATA_KEY, &metadata)?;

        Self::write(ACCESS_KEY, tokens.access_token.expose_secret())
    }

    fn clear(&self) -> Result<(), AppError> {
        Self::remove(ACCESS_KEY)?;
        Self::remove(REFRESH_KEY)?;
        Self::remove(METADATA_KEY)
    }
}

/// In-memory store.
///
/// Used by tests, and offered to the user as a session-only mode when the
/// keychain is unreachable. Nothing is written to disk, so the tokens are gone
/// when the process exits — which is the right trade for a machine that cannot
/// protect them at rest.
#[derive(Default)]
pub struct MemoryStore {
    values: Mutex<HashMap<&'static str, String>>,
    /// Records the sequence of `save` operations, so the write ordering above
    /// can be asserted rather than trusted.
    writes: Mutex<Vec<&'static str>>,
}

impl MemoryStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The operations performed so far, in order.
    pub fn write_log(&self) -> Vec<&'static str> {
        self.writes
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }

    fn put(&self, key: &'static str, value: String) -> Result<(), AppError> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| AppError::Internal("in-memory token store poisoned".to_owned()))?;
        values.insert(key, value);
        if let Ok(mut log) = self.writes.lock() {
            log.push(key);
        }
        Ok(())
    }

    fn take(&self, key: &str) -> Result<Option<String>, AppError> {
        let values = self
            .values
            .lock()
            .map_err(|_| AppError::Internal("in-memory token store poisoned".to_owned()))?;
        Ok(values.get(key).cloned())
    }

    /// Seeds raw values, so a test can simulate a partially written or corrupt
    /// keychain entry.
    #[cfg(test)]
    pub fn seed(
        &self,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<(), AppError> {
        for (key, value) in [
            (ACCESS_KEY, access_token),
            (REFRESH_KEY, refresh_token),
            (METADATA_KEY, metadata),
        ] {
            if let Some(value) = value {
                self.put(key, value.to_owned())?;
            }
        }
        Ok(())
    }
}

impl TokenStore for MemoryStore {
    fn load(&self) -> Result<Option<StoredTokens>, AppError> {
        let Some(access_token) = self.take(ACCESS_KEY)? else {
            return Ok(None);
        };

        let refresh_token = self.take(REFRESH_KEY)?.map(secret);

        let metadata = match self.take(METADATA_KEY)? {
            Some(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            None => TokenMetadata::default(),
        };

        Ok(Some(StoredTokens {
            access_token: secret(access_token),
            refresh_token,
            metadata,
        }))
    }

    fn save(&self, tokens: &StoredTokens) -> Result<(), AppError> {
        if let Some(refresh_token) = &tokens.refresh_token {
            self.put(REFRESH_KEY, refresh_token.expose_secret().to_owned())?;
        }

        let metadata = serde_json::to_string(&tokens.metadata).map_err(|error| {
            AppError::Internal(format!("cannot serialise token metadata: {error}"))
        })?;
        self.put(METADATA_KEY, metadata)?;

        self.put(ACCESS_KEY, tokens.access_token.expose_secret().to_owned())
    }

    fn clear(&self) -> Result<(), AppError> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| AppError::Internal("in-memory token store poisoned".to_owned()))?;
        values.clear();
        Ok(())
    }
}
