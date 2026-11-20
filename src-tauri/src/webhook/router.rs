//! Turning a verified delivery into work.
//!
//! The router deliberately does **not** apply the payload to the database. It
//! records the delivery, works out which repository it concerns, and asks the
//! poller to refresh that repository immediately.
//!
//! That is one API request per event instead of zero, and it buys three things
//! worth more than the request: the mapping from event to state stays in the
//! one place that already has tests for it, there is no second writer racing
//! the poller's transaction, and a webhook for an event the client does not
//! understand still results in correct state rather than a gap.

use std::sync::Arc;

use serde_json::Value;

use crate::db::{self, Db};
use crate::error::AppError;
use crate::poller::Poller;

/// Events Pulse reacts to.
///
/// Anything else is recorded and ignored; a webhook that returns 500 because
/// GitHub added an event type would be retried forever.
pub const HANDLED_EVENTS: &[&str] = &[
    "ping",
    "workflow_run",
    "workflow_job",
    "deployment",
    "deployment_status",
    "pull_request",
    "check_run",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// GitHub's "your endpoint works" event. Nothing to do.
    Ping,
    /// Already seen; GitHub retries, and a relay can replay.
    Duplicate,
    /// A watched repository was queued for refresh.
    Refreshed { repo_id: i64 },
    /// Recorded, but there was nothing to refresh.
    Ignored(&'static str),
}

impl Outcome {
    #[must_use]
    pub const fn describe(&self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::Duplicate => "duplicate delivery",
            Self::Refreshed { .. } => "queued a refresh",
            Self::Ignored(reason) => reason,
        }
    }
}

pub async fn handle(
    db: Db,
    poller: Arc<Poller>,
    delivery_id: String,
    event: String,
    body: Vec<u8>,
) -> Result<Outcome, AppError> {
    if !HANDLED_EVENTS.contains(&event.as_str()) {
        return Ok(Outcome::Ignored("event type is not handled"));
    }

    let repository = extract_repository(&body);

    let is_new = {
        let (delivery_id, event, repository) = (delivery_id, event.clone(), repository.clone());

        db.with_connection(move |connection| {
            db::deliveries::record(connection, &delivery_id, &event, repository.as_deref())
        })
        .await?
    };

    if !is_new {
        return Ok(Outcome::Duplicate);
    }

    if event == "ping" {
        return Ok(Outcome::Ping);
    }

    let Some(full_name) = repository else {
        return Ok(Outcome::Ignored("payload carries no repository"));
    };

    let repo_id = db
        .with_connection(move |connection| {
            Ok(db::repos::by_full_name(connection, &full_name)?.map(|repo| repo.id))
        })
        .await?;

    match repo_id {
        Some(repo_id) => {
            poller.request_refresh(repo_id);
            Ok(Outcome::Refreshed { repo_id })
        }
        None => Ok(Outcome::Ignored("repository is not watched")),
    }
}

/// GitHub puts the repository under `repository.full_name` in every event we
/// handle. Read from the raw bytes without re-serialising anything.
fn extract_repository(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;

    value
        .get("repository")?
        .get("full_name")?
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_repository_from_a_payload() {
        let body = br#"{"action":"completed","repository":{"full_name":"acme/api"}}"#;

        assert_eq!(extract_repository(body).as_deref(), Some("acme/api"));
    }

    #[test]
    fn a_payload_without_a_repository_is_not_an_error() {
        assert_eq!(
            extract_repository(br#"{"zen":"Design for failure."}"#),
            None
        );
    }

    #[test]
    fn a_payload_that_is_not_json_is_not_an_error() {
        assert_eq!(extract_repository(b"not json"), None);
    }
}
