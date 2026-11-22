//! Where GitHub tokens live.
//!
//! Tokens go to the OS keychain (Windows Credential Manager, macOS Keychain,
//! Secret Service on Linux) and never to `SQLite`, a config file, or the frontend.
//!
//! One account per set of keys, plus an index of the logins this machine knows
//! about and a pointer to the active one. The naming, the write ordering and the
//! bookkeeping live once, in [`Stored`]; the only difference between the real
//! keychain and the in-memory store the tests use is the [`Vault`] underneath.
//! That is deliberate — the behaviour the tests pin is the behaviour the keychain
//! gets, rather than two implementations that agree until they do not.
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

/// Token keys without an account. A per-account key is `<key>:<login>`, and the
/// unprefixed form is what builds before accounts existed (`0.5.6` and earlier)
/// used — read once, to adopt an existing sign-in rather than making someone do
/// it again.
const ACCESS_KEY: &str = "github-access-token";
const REFRESH_KEY: &str = "github-refresh-token";
const METADATA_KEY: &str = "github-token-metadata";

/// The index of logins with stored tokens, as a JSON array.
const ACCOUNTS_KEY: &str = "github-accounts";
/// Which of them the app is signed in as.
const ACTIVE_KEY: &str = "github-active-account";

/// GitHub logins are case-insensitive, and so are the filesystems and credential
/// stores this runs on, so the account part of a key is lowercased: `n4od` and
/// `N4OD` must not become two accounts. The database file is named by the same
/// rule — see [`crate::accounts`].
use crate::accounts::normalize;

fn account_key(base: &str, login: &str) -> String {
    format!("{base}:{}", normalize(login))
}

fn poisoned() -> AppError {
    AppError::Internal("the token store was poisoned by an earlier panic".to_owned())
}

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

/// One account's tokens, and the index that says which accounts exist.
pub trait TokenStore: Send + Sync + 'static {
    /// Whether the backing store can be reached at all.
    ///
    /// A machine with no Secret Service daemon should find out here, at startup,
    /// rather than on the first sign-in.
    fn probe(&self) -> Result<(), AppError>;

    /// `Ok(None)` when nothing is stored for that account. A store that exists
    /// but has no entry for it is not an error.
    fn load(&self, login: &str) -> Result<Option<StoredTokens>, AppError>;

    /// Persists one account's tokens, records it in the index, and makes it the
    /// active account.
    fn save(&self, login: &str, tokens: &StoredTokens) -> Result<(), AppError>;

    /// Forgets one account.
    fn remove(&self, login: &str) -> Result<(), AppError>;

    /// The accounts with stored tokens, lowercased and sorted.
    fn known_logins(&self) -> Result<Vec<String>, AppError>;

    /// The account the app is signed in as, if any.
    fn active(&self) -> Result<Option<String>, AppError>;

    /// Points the app at a different account. Touches no tokens.
    fn set_active(&self, login: Option<&str>) -> Result<(), AppError>;
}

/// `secrecy` 0.10 wraps a `Box<str>`, which is noisy at every call site.
#[must_use]
pub fn secret(value: impl Into<Box<str>>) -> SecretString {
    SecretString::new(value.into())
}

/// The key-value store the tokens sit in.
pub trait Vault: Send + Sync + 'static {
    fn get(&self, key: &str) -> Result<Option<String>, AppError>;
    fn set(&self, key: &str, value: &str) -> Result<(), AppError>;
    fn delete(&self, key: &str) -> Result<(), AppError>;
}

/// The OS keychain.
pub struct Keychain;

impl Vault for Keychain {
    fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        crate::secrets::get(key)
    }

    fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        crate::secrets::set(key, value)
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        crate::secrets::delete(key)
    }
}

/// An in-memory vault.
///
/// Used by tests, and offered to the user as a session-only mode when the
/// keychain is unreachable. Nothing is written to disk, so the tokens are gone
/// when the process exits — which is the right trade for a machine that cannot
/// protect them at rest.
#[derive(Default)]
pub struct Map {
    values: Mutex<HashMap<String, String>>,
    /// The keys written, in order, so the crash-safety ordering in [`Stored`]
    /// can be asserted rather than trusted. Deletions and seeds are not
    /// operations and are not recorded.
    writes: Mutex<Vec<String>>,
}

impl Map {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, String>>, AppError> {
        self.values.lock().map_err(|_| poisoned())
    }

    /// Writes without recording it, for tests that seed state a previous run
    /// would have left behind.
    fn put_raw(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.lock()?.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn write_log(&self) -> Vec<String> {
        self.writes
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }
}

impl Vault for Map {
    fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        Ok(self.lock()?.get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.put_raw(key, value)?;
        self.writes
            .lock()
            .map_err(|_| poisoned())?
            .push(key.to_owned());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        self.lock()?.remove(key);
        Ok(())
    }
}

/// A token store over some [`Vault`].
pub struct Stored<V: Vault> {
    vault: V,
}

impl<V: Vault> Stored<V> {
    fn known(&self) -> Result<Vec<String>, AppError> {
        let Some(raw) = self.vault.get(ACCOUNTS_KEY)? else {
            return Ok(Vec::new());
        };

        Ok(serde_json::from_str(&raw).unwrap_or_else(|error| {
            // A corrupt index must not lock anyone out: every account still has
            // its own entries, so the list is recoverable and losing it costs a
            // switcher entry, not a sign-in.
            tracing::warn!(%error, "the account index was unreadable; treating it as empty");
            Vec::new()
        }))
    }

    fn put_known(&self, logins: &[String]) -> Result<(), AppError> {
        let raw = serde_json::to_string(logins).map_err(|error| {
            AppError::Internal(format!("cannot serialise the account index: {error}"))
        })?;
        self.vault.set(ACCOUNTS_KEY, &raw)
    }

    /// Moves a sign-in that predates accounts onto the per-account keys.
    ///
    /// Idempotent, and only reached from [`Stored::active`] when no active
    /// account is recorded.
    fn adopt_legacy(&self) -> Result<Option<String>, AppError> {
        let Some(access_token) = self.vault.get(ACCESS_KEY)? else {
            return Ok(None);
        };

        let metadata = match self.vault.get(METADATA_KEY)? {
            Some(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            None => TokenMetadata::default(),
        };

        let Some(login) = metadata.login.clone().filter(|login| !login.is_empty()) else {
            // Tokens that cannot be attributed to an account cannot be filed.
            // Rather than inventing a name for them, discard them: the cost is
            // one sign-in, and the alternative is an account called "".
            tracing::warn!("found stored tokens with no account recorded; discarding them");
            self.vault.delete(ACCESS_KEY)?;
            self.vault.delete(REFRESH_KEY)?;
            self.vault.delete(METADATA_KEY)?;
            return Ok(None);
        };

        let stored = StoredTokens {
            access_token: secret(access_token),
            refresh_token: self.vault.get(REFRESH_KEY)?.map(secret),
            metadata,
        };

        // `save` also records the account in the index and makes it active.
        self.save(&login, &stored)?;

        self.vault.delete(ACCESS_KEY)?;
        self.vault.delete(REFRESH_KEY)?;
        self.vault.delete(METADATA_KEY)?;

        tracing::info!(%login, "adopted the sign-in that predates accounts");
        Ok(Some(login))
    }
}

impl<V: Vault> TokenStore for Stored<V> {
    fn probe(&self) -> Result<(), AppError> {
        // Any read answers the question; the value is not the point.
        self.vault.get(ACTIVE_KEY).map(|_| ())
    }

    fn load(&self, login: &str) -> Result<Option<StoredTokens>, AppError> {
        let Some(access_token) = self.vault.get(&account_key(ACCESS_KEY, login))? else {
            return Ok(None);
        };

        let refresh_token = self
            .vault
            .get(&account_key(REFRESH_KEY, login))?
            .map(secret);

        let metadata = match self.vault.get(&account_key(METADATA_KEY, login))? {
            Some(raw) => serde_json::from_str(&raw).unwrap_or_else(|error| {
                // A corrupt metadata blob must not lock the user out: the tokens
                // are still usable, we just re-learn the identity.
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

    /// Write order is load-bearing, in two directions.
    ///
    /// GitHub invalidates a refresh token the moment it issues a new one, so if
    /// we crashed having written the access token but not yet the refresh token,
    /// the stored refresh token would already be dead and the user would have to
    /// sign in again. Writing the refresh token first means the worst case is a
    /// stale access token we can silently renew.
    ///
    /// The index and the active pointer are written *last* for the same reason
    /// read the other way: an index entry is a promise that the tokens are
    /// there. Listing an account whose tokens never landed would offer a sign-in
    /// that cannot work.
    fn save(&self, login: &str, tokens: &StoredTokens) -> Result<(), AppError> {
        if let Some(refresh_token) = &tokens.refresh_token {
            self.vault.set(
                &account_key(REFRESH_KEY, login),
                refresh_token.expose_secret(),
            )?;
        }

        let metadata = serde_json::to_string(&tokens.metadata).map_err(|error| {
            AppError::Internal(format!("cannot serialise token metadata: {error}"))
        })?;
        self.vault
            .set(&account_key(METADATA_KEY, login), &metadata)?;

        self.vault.set(
            &account_key(ACCESS_KEY, login),
            tokens.access_token.expose_secret(),
        )?;

        // Only rewritten when the list actually changes: a refresh should not
        // touch the index.
        let normalized = normalize(login);
        let mut known = self.known()?;
        if !known.contains(&normalized) {
            known.push(normalized.clone());
            known.sort();
            self.put_known(&known)?;
        }

        self.vault.set(ACTIVE_KEY, &normalized)
    }

    /// Forgets one account: its tokens, its place in the index, and the active
    /// pointer if it was pointing there.
    ///
    /// The secrets go first. A sign-out that leaves a token behind because it
    /// failed halfway is worse than one that leaves a stale index entry, which
    /// the next read ignores.
    fn remove(&self, login: &str) -> Result<(), AppError> {
        self.vault.delete(&account_key(ACCESS_KEY, login))?;
        self.vault.delete(&account_key(REFRESH_KEY, login))?;
        self.vault.delete(&account_key(METADATA_KEY, login))?;

        let normalized = normalize(login);
        let known: Vec<String> = self
            .known()?
            .into_iter()
            .filter(|entry| entry != &normalized)
            .collect();
        self.put_known(&known)?;

        // The pointer is read directly rather than through `active()`, which
        // would run the legacy adoption — no business of a sign-out. It is
        // cleared rather than quietly moved to another account: "which account
        // am I now?" is the user's decision, not the store's.
        let pointing_here = self
            .vault
            .get(ACTIVE_KEY)?
            .map(|entry| normalize(&entry))
            .as_deref()
            == Some(normalized.as_str());

        if pointing_here {
            self.vault.delete(ACTIVE_KEY)?;
        }

        Ok(())
    }

    fn known_logins(&self) -> Result<Vec<String>, AppError> {
        let mut known = self.known()?;
        known.sort();
        known.dedup();
        Ok(known)
    }

    /// The one read that is allowed to write.
    ///
    /// Only once per machine, and only for a keychain written before accounts
    /// existed: this is the first read of a store that may predate them, so it
    /// is the only place adoption can happen.
    fn active(&self) -> Result<Option<String>, AppError> {
        if let Some(login) = self.vault.get(ACTIVE_KEY)? {
            return Ok(Some(normalize(&login)));
        }

        Ok(self.adopt_legacy()?.map(|login| normalize(&login)))
    }

    /// Nothing is validated here: the callers are a switcher that has just read
    /// the account list and a sign-out that has just read the session, so the
    /// account named is one of ours by construction.
    fn set_active(&self, login: Option<&str>) -> Result<(), AppError> {
        match login {
            Some(login) => self.vault.set(ACTIVE_KEY, &normalize(login)),
            None => self.vault.delete(ACTIVE_KEY),
        }
    }
}

/// The real store: the OS keychain.
pub type KeyringStore = Stored<Keychain>;

/// Session-only store: nothing reaches the disk.
pub type MemoryStore = Stored<Map>;

impl Stored<Keychain> {
    #[must_use]
    pub fn new() -> Self {
        Self { vault: Keychain }
    }
}

impl Default for Stored<Keychain> {
    fn default() -> Self {
        Self::new()
    }
}

impl Stored<Map> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            vault: Map::default(),
        }
    }

    /// The keys written so far, in order.
    pub fn write_log(&self) -> Vec<String> {
        self.vault.write_log()
    }

    /// The raw value under a key, so a test can assert what was left behind.
    #[cfg(test)]
    pub fn peek(&self, key: &str) -> Result<Option<String>, AppError> {
        self.vault.get(key)
    }

    /// Seeds raw per-account values, so a test can simulate a partially written
    /// or corrupt entry.
    #[cfg(test)]
    pub fn seed(
        &self,
        login: &str,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<(), AppError> {
        for (key, value) in [
            (account_key(ACCESS_KEY, login), access_token),
            (account_key(REFRESH_KEY, login), refresh_token),
            (account_key(METADATA_KEY, login), metadata),
        ] {
            if let Some(value) = value {
                self.vault.put_raw(&key, value)?;
            }
        }

        Ok(())
    }

    /// Seeds the unprefixed keys that builds before accounts existed wrote, so a
    /// test can check that an existing sign-in is adopted rather than lost.
    #[cfg(test)]
    pub fn seed_legacy(
        &self,
        access_token: &str,
        refresh_token: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<(), AppError> {
        for (key, value) in [
            (ACCESS_KEY, Some(access_token)),
            (REFRESH_KEY, refresh_token),
            (METADATA_KEY, metadata),
        ] {
            if let Some(value) = value {
                self.vault.put_raw(key, value)?;
            }
        }

        Ok(())
    }
}
