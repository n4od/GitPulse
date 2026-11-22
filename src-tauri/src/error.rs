//! The single error type crossing the Rust <-> TypeScript boundary.
//!
//! Design rules:
//!  - Variants describe *what the user can do about it*, not which crate failed.
//!  - `Serialize` emits `{ code, message, retryable }` so the frontend can branch
//!    on `code` without parsing English.
//!  - Underlying causes are boxed rather than typed, so adding a dependency
//!    later never changes this public surface.

use std::error::Error as StdError;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Boxed cause, keeping `AppError` independent of concrete crate error types.
pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, thiserror::Error)]
// Variants are wired up as their milestone lands (M2 auth, M3 db, M4 network).
// The enum is the wire contract with the frontend, so it is
// defined once rather than grown ad hoc — hence the module-level allow.
#[allow(
    dead_code,
    reason = "wire contract defined ahead of the milestones that raise these variants"
)]
pub enum AppError {
    #[error("authentication required")]
    AuthRequired,

    #[error("authorization was denied")]
    AuthDenied,

    #[error("the stored token has been revoked")]
    TokenRevoked,

    #[error("the device code expired before authorization completed")]
    AuthExpired,

    #[error("sign-in was cancelled")]
    AuthCancelled,

    #[error("the OS keychain is unavailable: {0}")]
    KeyringUnavailable(String),

    #[error("network request failed")]
    Network {
        retryable: bool,
        #[source]
        source: BoxError,
    },

    #[error("primary rate limit exhausted until {reset_at}")]
    RateLimited { reset_at: DateTime<Utc> },

    #[error("secondary rate limit hit; retry in {retry_after:?}")]
    SecondaryRateLimited { retry_after: Duration },

    #[error("resource not found")]
    NotFound,

    #[error("insufficient permissions")]
    Forbidden { scope: Option<String> },

    #[error("database error")]
    Db {
        #[source]
        source: BoxError,
    },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("internal error: {0}")]
    Internal(String),
}

#[allow(
    dead_code,
    reason = "constructors land with the milestones that raise these variants"
)]
impl AppError {
    /// Stable machine-readable discriminator consumed by the frontend.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::AuthRequired => "auth_required",
            Self::AuthDenied => "auth_denied",
            Self::TokenRevoked => "token_revoked",
            Self::AuthExpired => "auth_expired",
            Self::AuthCancelled => "auth_cancelled",
            Self::KeyringUnavailable(_) => "keyring_unavailable",
            Self::Network { .. } => "network",
            Self::RateLimited { .. } => "rate_limited",
            Self::SecondaryRateLimited { .. } => "secondary_rate_limited",
            Self::NotFound => "not_found",
            Self::Forbidden { .. } => "forbidden",
            Self::Db { .. } => "db",
            Self::Config(_) => "config",
            Self::Internal(_) => "internal",
        }
    }

    /// Whether the same call may succeed if retried unchanged.
    #[must_use]
    pub const fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Network {
                retryable: true,
                ..
            } | Self::RateLimited { .. }
                | Self::SecondaryRateLimited { .. }
        )
    }

    /// Wraps a retryable transport failure.
    pub fn network(source: impl Into<BoxError>, retryable: bool) -> Self {
        Self::Network {
            retryable,
            source: source.into(),
        }
    }

    /// Wraps a database failure.
    pub fn db(source: impl Into<BoxError>) -> Self {
        Self::Db {
            source: source.into(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 3)?;
        state.serialize_field("code", self.code())?;
        // `Display` is already written to be user-facing; run it through the
        // redactor so a stray token in a cause never reaches the webview.
        state.serialize_field("message", &crate::redact::redact(&self.to_string()))?;
        state.serialize_field("retryable", &self.retryable())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_the_wire_contract() {
        let json = serde_json::to_value(AppError::NotFound).expect("serializable");
        assert_eq!(json["code"], "not_found");
        assert_eq!(json["message"], "resource not found");
        assert_eq!(json["retryable"], false);
    }

    #[test]
    fn marks_rate_limits_as_retryable() {
        let error = AppError::SecondaryRateLimited {
            retry_after: Duration::from_secs(30),
        };
        assert!(error.retryable());
    }

    #[test]
    fn redacts_tokens_in_messages() {
        let error = AppError::Internal("leaked ghp_abcdefghijklmnopqrstuvwxyz012345".into());
        let json = serde_json::to_value(error).expect("serializable");
        let message = json["message"].as_str().expect("message is a string");
        assert!(!message.contains("ghp_abcdefghijklmnopqrstuvwxyz012345"));
    }
}
