//! GitHub authentication: device flow, token custody and refresh.

pub mod device_flow;
pub mod oauth;
pub mod service;
pub mod token_store;
pub mod tokens;
pub mod types;

#[cfg(test)]
mod tests;

pub use service::{AuthService, AUTH_STATE_EVENT};
pub use token_store::{KeyringStore, MemoryStore, TokenStore};
pub use tokens::TokenManager;
pub use types::{AuthSession, AuthState};

/// Scopes requested during the device flow.
///
/// Empty by default, which is correct for a GitHub App: its permissions come
/// from the app registration, and asking for scopes is ignored. An OAuth App
/// needs explicit scopes instead, which is what `GITPULSE_GITHUB_SCOPES` is for.
///
/// This is the whole of the "GitHub App or OAuth App" decision: the flow, the
/// storage and the refresh all work with either, because they branch on what
/// the token response actually contains rather than on which app type it was.
#[must_use]
pub fn configured_scopes() -> Vec<String> {
    std::env::var("GITPULSE_GITHUB_SCOPES")
        .ok()
        .map(|value| {
            value
                .split([',', ' '])
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Resolves the GitHub OAuth/GitHub App client id.
///
/// A client id is not a secret — it identifies the app, not the user — but it
/// is still not hardcoded, because every deployment points at its own app
/// registration. Runtime beats compile-time so a packaged build can be
/// repointed without a rebuild.
#[must_use]
pub fn configured_client_id() -> Option<String> {
    if let Ok(value) = std::env::var("GITPULSE_GITHUB_CLIENT_ID") {
        let value = value.trim().to_owned();
        if !value.is_empty() {
            return Some(value);
        }
    }

    option_env!("GITPULSE_GITHUB_CLIENT_ID")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
