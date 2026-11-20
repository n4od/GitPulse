//! Webhook ingestion.
//!
//! GitHub cannot deliver to `127.0.0.1`, so there are three transports and the
//! user picks one:
//!
//! - **Off** — polling only. Always correct, just slower.
//! - **Relay** — a public endpoint forwards deliveries down a long-lived
//!   connection. Zero configuration beyond a URL.
//! - **Local** — bind a loopback listener and put a tunnel in front of it. The
//!   tunnel is the user's to run; Pulse can verify it reaches the listener.
//!
//! All three share one signature verifier, so a transport cannot be "the weak
//! one".

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::db::{self, Db};
use crate::error::AppError;
use crate::github::client::build_streaming_client;
use crate::poller::Poller;

pub mod relay;
pub mod router;
pub mod server;
pub mod signature;

#[cfg(test)]
mod tests;

/// Keychain entry holding the signing secret. Never `SQLite`, never the webview
/// except when the user explicitly asks to see it for setup.
const SIGNING_SECRET_KEY: &str = "github-webhook-secret";

/// The unguessable path segment. Not a secret in the cryptographic sense — the
/// signature is the real protection — but unguessable enough that a stray
/// local process gets a 404 instead of reaching the verifier.
const PATH_SECRET_META_KEY: &str = "webhook_path_secret";

const KIND_META_KEY: &str = "webhook_transport";
const RELAY_URL_META_KEY: &str = "webhook_relay_url";
const LOCAL_PORT_META_KEY: &str = "webhook_local_port";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum TransportKind {
    #[default]
    Off,
    Local,
    Relay,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct WebhookConfig {
    pub kind: TransportKind,
    pub running: bool,
    pub local_port: Option<u16>,
    pub relay_url: Option<String>,
    /// The URL to paste into a tunnel or anywhere else that asks.
    pub local_url: Option<String>,
    #[ts(type = "number")]
    pub deliveries: i64,
    pub last_delivery_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

struct Running {
    cancel: Option<CancellationToken>,
    kind: TransportKind,
    local_port: Option<u16>,
    relay_url: Option<String>,
    last_error: Option<String>,
}

pub struct WebhookService {
    db: Db,
    poller: Arc<Poller>,
    http: reqwest::Client,
    running: Mutex<Running>,
}

impl WebhookService {
    pub fn new(db: Db, poller: Arc<Poller>) -> Result<Self, AppError> {
        Ok(Self {
            db,
            poller,
            http: build_streaming_client()?,
            running: Mutex::new(Running {
                cancel: None,
                kind: TransportKind::Off,
                local_port: None,
                relay_url: None,
                last_error: None,
            }),
        })
    }

    /// Starts whatever transport the settings name. Safe to call repeatedly.
    pub async fn start(&self) -> Result<(), AppError> {
        self.stop();

        let (kind, port, relay_url) = self.load_settings().await?;

        match kind {
            TransportKind::Off => {
                self.set_running(TransportKind::Off, port, relay_url, None);
                Ok(())
            }
            TransportKind::Local => self.start_local(port).await,
            TransportKind::Relay => self.start_relay(relay_url),
        }
    }

    pub fn stop(&self) {
        let cancel = {
            let Ok(mut guard) = self.running.lock() else {
                return;
            };
            guard.cancel.take()
        };

        if let Some(cancel) = cancel {
            cancel.cancel();
        }
    }

    pub async fn config(&self) -> Result<WebhookConfig, AppError> {
        let (kind, port, relay_url) = self.load_settings().await?;

        let (running, local_port, last_error) = {
            let guard = self.lock()?;
            (
                guard.cancel.is_some(),
                guard.local_port,
                guard.last_error.clone(),
            )
        };

        let (deliveries, last_delivery_at) = self
            .db
            .with_connection(|connection| {
                Ok((
                    db::deliveries::count(connection)?,
                    db::deliveries::last_received_at(connection)?,
                ))
            })
            .await?;

        let local_url = match (kind, local_port.or(port)) {
            (TransportKind::Local, Some(port)) => {
                let path = self.path_secret().await?;
                Some(format!("http://127.0.0.1:{port}/{path}/webhook"))
            }
            _ => None,
        };

        Ok(WebhookConfig {
            kind,
            running: running && kind != TransportKind::Off,
            local_port: local_port.or(port),
            relay_url,
            local_url,
            deliveries,
            last_delivery_at,
            last_error,
        })
    }

    pub async fn set_transport(
        &self,
        kind: TransportKind,
        local_port: Option<u16>,
        relay_url: Option<String>,
    ) -> Result<(), AppError> {
        if kind == TransportKind::Relay {
            let url = relay_url.as_deref().unwrap_or_default();
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err(AppError::Config(
                    "the relay URL must be an http(s) URL".to_owned(),
                ));
            }
        }

        let (kind_value, port_value, relay_value) = (
            format!("{kind:?}"),
            local_port.map(|port| port.to_string()),
            relay_url.clone(),
        );

        self.db
            .with_connection(move |connection| {
                db::meta::set(connection, KIND_META_KEY, &kind_value)?;
                if let Some(port) = port_value {
                    db::meta::set(connection, LOCAL_PORT_META_KEY, &port)?;
                }
                if let Some(url) = relay_value {
                    db::meta::set(connection, RELAY_URL_META_KEY, &url)?;
                }
                Ok(())
            })
            .await?;

        self.start().await
    }

    /// Returns the signing secret, generating one on first use.
    pub fn signing_secret(&self) -> Result<String, AppError> {
        if let Some(existing) = crate::secrets::get(SIGNING_SECRET_KEY)? {
            return Ok(existing);
        }

        self.rotate_secret()
    }

    /// Replaces the signing secret. The old one stops working immediately, so
    /// the caller must update GitHub too.
    pub fn rotate_secret(&self) -> Result<String, AppError> {
        let secret = signature::generate_secret()?;
        crate::secrets::set(SIGNING_SECRET_KEY, &secret)?;
        tracing::info!("webhook signing secret rotated");
        Ok(secret)
    }

    /// Checks that a public URL actually reaches this listener.
    pub async fn verify_public_url(&self, url: &str) -> Result<bool, AppError> {
        let path = self.path_secret().await?;
        let base = url.trim_end_matches('/');

        if !(base.starts_with("https://") || base.starts_with("http://")) {
            return Err(AppError::Config(
                "the public URL must be an http(s) URL".to_owned(),
            ));
        }

        let probe = format!("{base}/{path}/health");

        match self.http.get(&probe).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(error) => Err(AppError::network(error, true)),
        }
    }

    /// The unguessable path segment, generated once and kept in `meta`.
    async fn path_secret(&self) -> Result<String, AppError> {
        let existing = self
            .db
            .with_connection(|connection| db::meta::get(connection, PATH_SECRET_META_KEY))
            .await?;

        if let Some(existing) = existing {
            return Ok(existing);
        }

        let generated = signature::generate_secret()?;
        let value = generated[..32].to_owned();
        let stored = value.clone();

        self.db
            .with_connection(move |connection| {
                db::meta::set(connection, PATH_SECRET_META_KEY, &stored)
            })
            .await?;

        Ok(value)
    }

    async fn load_settings(
        &self,
    ) -> Result<(TransportKind, Option<u16>, Option<String>), AppError> {
        let (kind, port, url) = self
            .db
            .with_connection(|connection| {
                Ok((
                    db::meta::get(connection, KIND_META_KEY)?,
                    db::meta::get(connection, LOCAL_PORT_META_KEY)?,
                    db::meta::get(connection, RELAY_URL_META_KEY)?,
                ))
            })
            .await?;

        let kind = match kind.as_deref() {
            Some("Local") => TransportKind::Local,
            Some("Relay") => TransportKind::Relay,
            _ => TransportKind::Off,
        };

        Ok((kind, port.and_then(|value| value.parse().ok()), url))
    }

    async fn start_local(&self, port: Option<u16>) -> Result<(), AppError> {
        let path_secret = self.path_secret().await?;
        let signing_secret = self.signing_secret()?;

        let state = server::ServerState {
            db: self.db.clone(),
            poller: Arc::clone(&self.poller),
            path_secret: Arc::from(path_secret.as_str()),
            signing_secret: Arc::from(signing_secret.as_str()),
        };

        let cancel = CancellationToken::new();

        // Bind synchronously so a port conflict surfaces here rather than in a
        // task nobody is watching.
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port.unwrap_or(0)))
            .await
            .map_err(|error| {
                AppError::Config(format!("cannot bind the webhook listener: {error}"))
            })?;

        let bound = listener
            .local_addr()
            .map_err(|error| AppError::Internal(format!("cannot read the bound port: {error}")))?
            .port();

        let shutdown = cancel.clone();
        tokio::spawn(async move {
            if let Err(error) = server::serve_with_listener(listener, state, shutdown).await {
                tracing::warn!(%error, "the webhook listener stopped");
            }
        });

        self.set_running(TransportKind::Local, Some(bound), None, Some(cancel));
        tracing::info!(port = bound, "webhook listener started");
        Ok(())
    }

    fn start_relay(&self, url: Option<String>) -> Result<(), AppError> {
        let Some(url) = url else {
            return Err(AppError::Config("no relay URL is configured".to_owned()));
        };

        let signing_secret = self.signing_secret()?;
        let cancel = CancellationToken::new();

        let (db, poller, http) = (self.db.clone(), Arc::clone(&self.poller), self.http.clone());
        let shutdown = cancel.clone();
        let secret: Arc<str> = Arc::from(signing_secret.as_str());

        tokio::spawn(async move {
            relay::run(http, db, poller, url, secret, shutdown).await;
        });

        self.set_running(TransportKind::Relay, None, None, Some(cancel));
        Ok(())
    }

    fn set_running(
        &self,
        kind: TransportKind,
        local_port: Option<u16>,
        relay_url: Option<String>,
        cancel: Option<CancellationToken>,
    ) {
        if let Ok(mut guard) = self.running.lock() {
            guard.cancel = cancel;
            guard.kind = kind;
            guard.local_port = local_port;
            guard.relay_url = relay_url;
            guard.last_error = None;
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Running>, AppError> {
        self.running
            .lock()
            .map_err(|_| AppError::Internal("the webhook service lock was poisoned".to_owned()))
    }
}
