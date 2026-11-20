//! The OS keychain.
//!
//! One place knows how to talk to it, so the error mapping — and the
//! "a missing entry is not an error" rule — is stated once.
//!
//! Nothing here writes to disk. If the keychain is unavailable the caller gets
//! `KeyringUnavailable` and decides what to do; it never falls back to a file.

use crate::error::AppError;

/// Namespaced by bundle identifier so entries cannot collide with another app's.
pub const SERVICE: &str = "dev.gitpulse.workflowmonitor";

fn unavailable(error: &keyring::Error) -> AppError {
    AppError::KeyringUnavailable(error.to_string())
}

fn entry(key: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(SERVICE, key).map_err(|error| unavailable(&error))
}

/// `Ok(None)` when nothing is stored under `key`.
pub fn get(key: &str) -> Result<Option<String>, AppError> {
    match entry(key)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(unavailable(&error)),
    }
}

pub fn set(key: &str, value: &str) -> Result<(), AppError> {
    entry(key)?
        .set_password(value)
        .map_err(|error| unavailable(&error))
}

/// Deleting something that is not there is a success: the postcondition holds.
pub fn delete(key: &str) -> Result<(), AppError> {
    match entry(key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(unavailable(&error)),
    }
}
