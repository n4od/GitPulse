//! Types that cross the Rust → TypeScript boundary for authentication.
//!
//! These are exported to `src/lib/bindings/` by `ts-rs` and checked in. The
//! frontend imports them rather than redeclaring them, which is what stops the
//! two sides drifting apart.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A signed-in user.
///
/// `expires_at` is `None` when the token does not expire: an OAuth App token,
/// or a GitHub App with token expiry disabled. `can_refresh` distinguishes
/// "expires but has no refresh token" (re-authenticate) from "expires and can
/// be renewed silently".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct AuthSession {
    pub login: String,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
    pub can_refresh: bool,
}

/// Where the sign-in flow currently is.
///
/// Modelled as a discriminated union so the UI can exhaustively match on
/// `phase` instead of guessing from a pile of booleans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "phase",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum AuthState {
    /// No token, or the token was rejected and cleared.
    SignedOut,

    /// Asking GitHub for a device code.
    RequestingCode,

    /// Waiting for the user to enter `user_code` in their browser.
    AwaitingUser {
        user_code: String,
        verification_uri: String,
        expires_at: DateTime<Utc>,
    },

    /// The user has been to the browser; polling for the result.
    ///
    /// `verification_uri` is carried here too: the UI must keep offering
    /// "reopen GitHub" while polling, and a user who closed the tab needs the
    /// link back.
    Polling {
        user_code: String,
        verification_uri: String,
        expires_at: DateTime<Utc>,
    },

    Authorized {
        session: AuthSession,
    },

    /// The user clicked "cancel" on GitHub's page.
    Denied,

    /// The device code expired before the user finished.
    Expired,

    /// Anything else. `code` is a stable `AppError` discriminator.
    Failed {
        code: String,
        message: String,
    },
}

impl AuthState {
    #[must_use]
    pub fn failed(error: &crate::error::AppError) -> Self {
        Self::Failed {
            code: error.code().to_owned(),
            message: error.to_string(),
        }
    }

    /// Whether this state means "there is a usable session".
    #[must_use]
    pub fn session(&self) -> Option<&AuthSession> {
        match self {
            Self::Authorized { session } => Some(session),
            _ => None,
        }
    }
}
