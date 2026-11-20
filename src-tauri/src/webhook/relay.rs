//! Relay transport.
//!
//! GitHub cannot deliver to `127.0.0.1`, so the practical way to receive
//! webhooks on a desktop machine is a relay: a public endpoint that forwards
//! deliveries down a long-lived connection. smee.io is the one this is built
//! for, but anything that speaks the same shape works.
//!
//! **The relay must forward headers.** Pulse verifies every payload against the
//! signing secret, and the signature lives in a header. A relay that forwards
//! only the body produces frames Pulse will refuse — deliberately, because the
//! alternative is trusting anything that can reach the relay URL.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::db::Db;
use crate::error::AppError;
use crate::github::client::{transport_error, USER_AGENT};
use crate::poller::Poller;

use super::router;
use super::signature::verify;

const BASE_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(120);

/// The envelope Pulse expects inside an SSE `data:` frame.
#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(default)]
    headers: BTreeMap<String, String>,
    /// The raw webhook payload, as text, so it can be verified byte for byte.
    body: String,
}

/// Runs until cancelled, reconnecting with exponential backoff.
pub async fn run(
    http: reqwest::Client,
    db: Db,
    poller: Arc<Poller>,
    url: String,
    signing_secret: Arc<str>,
    cancel: CancellationToken,
) {
    let mut attempt = 0_u32;

    loop {
        if cancel.is_cancelled() {
            return;
        }

        match session(&http, &db, &poller, &url, &signing_secret, &cancel).await {
            Ok(()) => {
                attempt = 0;
                tracing::info!("the relay stream closed");
            }
            Err(error) => tracing::warn!(%error, "the relay stream failed"),
        }

        attempt += 1;
        let backoff = BASE_BACKOFF
            .saturating_mul(1 << attempt.min(6))
            .min(MAX_BACKOFF);

        tracing::info!(?backoff, attempt, "reconnecting to the relay");

        tokio::select! {
            () = cancel.cancelled() => return,
            () = tokio::time::sleep(backoff) => {}
        }
    }
}

async fn session(
    http: &reqwest::Client,
    db: &Db,
    poller: &Arc<Poller>,
    url: &str,
    signing_secret: &str,
    cancel: &CancellationToken,
) -> Result<(), AppError> {
    let mut response = http
        .get(url)
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(transport_error)?;

    if !response.status().is_success() {
        return Err(AppError::Internal(format!(
            "the relay answered {}",
            response.status()
        )));
    }

    let mut buffer: Vec<u8> = Vec::new();

    loop {
        let chunk = tokio::select! {
            () = cancel.cancelled() => return Ok(()),
            chunk = response.chunk() => chunk.map_err(transport_error)?,
        };

        let Some(chunk) = chunk else {
            return Ok(());
        };

        buffer.extend_from_slice(&chunk);

        while let Some(frame) = take_frame(&mut buffer) {
            dispatch(&frame, db, poller, signing_secret).await;
        }
    }
}

async fn dispatch(frame: &[u8], db: &Db, poller: &Arc<Poller>, signing_secret: &str) {
    let Some(data) = frame_data(frame) else {
        return;
    };

    // Relays send keep-alive comments and empty frames; they are not errors.
    if data.trim().is_empty() || data.trim() == "{}" {
        return;
    }

    let Ok(envelope) = serde_json::from_str::<Envelope>(&data) else {
        tracing::warn!(
            "the relay sent a frame Pulse does not recognise; it must forward the \
             webhook headers and the raw body"
        );
        return;
    };

    let signature = envelope.headers.get("x-hub-signature-256");

    let Some(signature) = signature else {
        tracing::warn!("refusing a relayed payload with no signature header");
        return;
    };

    if !verify(
        signing_secret.as_bytes(),
        envelope.body.as_bytes(),
        signature,
    ) {
        tracing::warn!("refusing a relayed payload whose signature did not match");
        return;
    }

    let Some(event) = envelope.headers.get("x-github-event").cloned() else {
        tracing::warn!("refusing a relayed payload with no event header");
        return;
    };

    let delivery = envelope
        .headers
        .get("x-github-delivery")
        .cloned()
        .unwrap_or_else(|| format!("relay-{}", uuid_like()));

    match router::handle(
        db.clone(),
        Arc::clone(poller),
        delivery,
        event,
        envelope.body.into_bytes(),
    )
    .await
    {
        Ok(outcome) => tracing::debug!(outcome = outcome.describe(), "relayed webhook processed"),
        Err(error) => tracing::warn!(%error, "relayed webhook processing failed"),
    }
}

/// Extracts the `data:` payload of one SSE frame.
fn frame_data(frame: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(frame);
    let mut data = String::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim_start());
        }
    }

    if data.is_empty() {
        None
    } else {
        Some(data)
    }
}

/// Removes and returns one complete frame, if the buffer holds one.
///
/// SSE frames are separated by a blank line, which may be `\n\n` or `\r\n\r\n`.
fn take_frame(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let position = buffer
        .windows(2)
        .position(|window| window == b"\n\n" || window == b"\r\r")
        .or_else(|| {
            buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 1)
        })?;

    Some(buffer.drain(..position + 2).collect())
}

/// A stand-in id for relays that do not forward `X-GitHub-Delivery`.
fn uuid_like() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());

    format!("{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_frames_on_a_blank_line() {
        let mut buffer = b"data: one\n\ndata: two\n\n".to_vec();

        assert_eq!(
            take_frame(&mut buffer).map(|frame| String::from_utf8_lossy(&frame).into_owned()),
            Some("data: one\n\n".to_owned())
        );
        assert_eq!(
            take_frame(&mut buffer).map(|frame| String::from_utf8_lossy(&frame).into_owned()),
            Some("data: two\n\n".to_owned())
        );
        assert!(take_frame(&mut buffer).is_none());
    }

    #[test]
    fn waits_for_a_complete_frame() {
        let mut buffer = b"data: partial".to_vec();
        assert!(
            take_frame(&mut buffer).is_none(),
            "must not split mid-frame"
        );
    }

    #[test]
    fn reads_a_data_line_with_its_optional_space() {
        let frame = b"event: message\ndata: {\"a\":1}\n\n";
        assert_eq!(frame_data(frame).as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn joins_multiple_data_lines() {
        let frame = b"data: first\ndata: second\n\n";
        assert_eq!(frame_data(frame).as_deref(), Some("first\nsecond"));
    }

    #[test]
    fn a_comment_frame_has_no_data() {
        assert_eq!(frame_data(b": keep-alive\n\n"), None);
    }

    #[test]
    fn an_envelope_parses_with_headers_and_body() {
        let envelope: Envelope = serde_json::from_str(
            r#"{"headers":{"x-github-event":"ping","x-hub-signature-256":"sha256=ab"},"body":"{\"zen\":\"x\"}"}"#,
        )
        .expect("valid envelope");

        assert_eq!(
            envelope.headers.get("x-github-event").map(String::as_str),
            Some("ping")
        );
        assert_eq!(envelope.body, r#"{"zen":"x"}"#);
    }

    #[test]
    fn an_envelope_without_headers_still_parses_so_the_caller_can_reject_it() {
        let envelope: Envelope = serde_json::from_str(r#"{"body":"{}"}"#).expect("valid envelope");

        assert!(envelope.headers.is_empty());
    }
}
