//! GitHub access: the HTTP client, and (from M4) the REST surface with its
//! `ETag` cache and rate-limit budget.

pub mod client;

pub use client::{build_client, Endpoints, API_VERSION, USER_AGENT};
