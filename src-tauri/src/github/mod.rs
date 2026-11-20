//! GitHub access: HTTP client, request pipeline, endpoints, and log download.
//!
//! The layering is deliberate:
//!
//! - [`client`] owns the `reqwest` configuration and the base URLs.
//! - [`budget`] owns how fast we may talk to GitHub.
//! - [`etag`] owns the conditional-request cache.
//! - [`api`] is the single pipeline every request goes through.
//! - [`endpoints`] is a thin, typed surface over that pipeline.
//! - [`models`] maps GitHub's JSON into the cache rows.

pub mod api;
pub mod budget;
pub mod client;
pub mod endpoints;
pub mod etag;
pub mod logs;
pub mod models;

#[cfg(test)]
mod tests;

pub use api::{GithubClient, RateLimitSnapshot, TokenProvider};
pub use budget::RateBudget;
pub use client::{build_client, Endpoints, API_VERSION, USER_AGENT};
pub use logs::{LogDownload, LOG_CAP_BYTES};

/// Event name the frontend subscribes to for the rate-limit meter.
pub const RATE_LIMIT_EVENT: &str = "ratelimit://updated";
