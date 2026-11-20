//! The loopback webhook listener.
//!
//! Bound to `127.0.0.1` only. The path carries an unguessable segment so a
//! drive-by POST from another local process gets a 404 rather than a signature
//! check, and the signature itself is verified over the raw bytes before
//! anything is parsed.
//!
//! GitHub cannot deliver to loopback addresses at all, which is why this is one
//! of three transports rather than the default — see the README.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::db::Db;
use crate::error::AppError;
use crate::poller::Poller;

use super::router;
use super::signature::{verify, MAX_BODY_BYTES, SIGNATURE_HEADER};

#[derive(Clone)]
pub struct ServerState {
    pub db: Db,
    pub poller: Arc<Poller>,
    pub path_secret: Arc<str>,
    pub signing_secret: Arc<str>,
}

/// Binds a loopback port and serves until `cancel` fires, returning the port.
pub async fn serve(
    state: ServerState,
    port: u16,
    cancel: CancellationToken,
) -> Result<u16, AppError> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| AppError::Config(format!("cannot bind the webhook listener: {error}")))?;

    let bound = listener
        .local_addr()
        .map_err(|error| AppError::Internal(format!("cannot read the bound port: {error}")))?
        .port();

    serve_with_listener(listener, state, cancel).await?;

    Ok(bound)
}

/// Serves on an already-bound listener, so the caller can report the port
/// before committing to the long-running task.
pub async fn serve_with_listener(
    listener: TcpListener,
    state: ServerState,
    cancel: CancellationToken,
) -> Result<(), AppError> {
    let app = Router::new()
        .route("/{path_secret}/webhook", post(receive))
        .route("/{path_secret}/health", get(health))
        // Rejected before the body is buffered, not after.
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { cancel.cancelled().await })
        .await
        .map_err(|error| AppError::Internal(format!("the webhook listener failed: {error}")))
}

async fn receive(
    State(state): State<ServerState>,
    Path(path_secret): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    if path_secret != state.path_secret.as_ref() {
        return StatusCode::NOT_FOUND;
    }

    let Some(signature) = header(&headers, SIGNATURE_HEADER) else {
        return StatusCode::BAD_REQUEST;
    };

    // Before parsing, before anything else.
    if !verify(state.signing_secret.as_bytes(), &body, signature) {
        tracing::warn!("rejected a webhook whose signature did not match");
        return StatusCode::UNAUTHORIZED;
    }

    match header(&headers, "content-type") {
        Some(value) if value.starts_with("application/json") => {}
        _ => return StatusCode::UNSUPPORTED_MEDIA_TYPE,
    }

    let (Some(delivery), Some(event)) = (
        header(&headers, "x-github-delivery"),
        header(&headers, "x-github-event"),
    ) else {
        return StatusCode::BAD_REQUEST;
    };

    let (delivery, event) = (delivery.to_owned(), event.to_owned());
    let body = body.to_vec();
    let (db, poller) = (state.db.clone(), Arc::clone(&state.poller));

    // Acknowledge immediately: GitHub gives an endpoint ten seconds and retries
    // on anything slower, so doing the work inline would turn a slow poll into
    // a delivery storm.
    tokio::spawn(async move {
        match router::handle(db, poller, delivery, event, body).await {
            Ok(outcome) => tracing::debug!(outcome = outcome.describe(), "webhook processed"),
            Err(error) => tracing::warn!(%error, "webhook processing failed"),
        }
    });

    StatusCode::ACCEPTED
}

async fn health(State(state): State<ServerState>, Path(path_secret): Path<String>) -> StatusCode {
    if path_secret == state.path_secret.as_ref() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}
