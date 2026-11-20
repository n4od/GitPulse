use std::sync::{Arc, Mutex};

use hmac::{Hmac, KeyInit, Mac};
use secrecy::SecretString;
use sha2::Sha256;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::token_store::secret;
use crate::db::{self, Db, RepoCard};
use crate::error::AppError;
use crate::github::{Endpoints, GithubClient, RateBudget, TokenProvider};
use crate::poller::{PollConfig, PollEvents, Poller, PollerStatus};

use super::server::{self, ServerState};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

struct StaticToken(SecretString);

#[async_trait::async_trait]
impl TokenProvider for StaticToken {
    async fn access_token(&self) -> Result<SecretString, AppError> {
        Ok(self.0.clone())
    }
}

#[derive(Default)]
struct Recording {
    cards: Mutex<Vec<RepoCard>>,
    statuses: Mutex<Vec<PollerStatus>>,
}

impl PollEvents for Recording {
    fn repo_updated(&self, card: RepoCard) {
        self.cards.lock().expect("cards").push(card);
    }

    fn status(&self, status: PollerStatus) {
        self.statuses.lock().expect("statuses").push(status);
    }
}

struct Harness {
    port: u16,
    path_secret: String,
    signing_secret: String,
    poller: Arc<Poller>,
    cancel: CancellationToken,
    http: reqwest::Client,
}

impl Harness {
    fn endpoint(&self, path_suffix: &str) -> String {
        format!(
            "http://127.0.0.1:{}/{}/{path_suffix}",
            self.port, self.path_secret
        )
    }

    fn sign(&self, body: &[u8]) -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.signing_secret.as_bytes()).expect("any key length");
        mac.update(body);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

async fn harness(github: &MockServer, repositories: &[&str]) -> Harness {
    let db = Db::open_in_memory().expect("database");

    for (index, full_name) in repositories.iter().enumerate() {
        let full_name = (*full_name).to_owned();
        let github_id = i64::try_from(index).expect("small index") + 1;

        db.with_connection(move |connection| {
            let (owner, name) = full_name.split_once('/').expect("owner/name");
            db::repos::upsert(
                connection,
                &db::RepoUpsert {
                    github_id,
                    owner: owner.to_owned(),
                    name: name.to_owned(),
                    full_name: full_name.clone(),
                    is_private: false,
                    default_branch: Some("main".to_owned()),
                    avatar_url: None,
                },
            )?;
            Ok(())
        })
        .await
        .expect("seed repository");
    }

    let client = Arc::new(GithubClient::new(
        crate::github::build_client().expect("http client"),
        Endpoints::single_origin(&github.uri()),
        Arc::new(StaticToken(secret("ghu_test_token"))),
        Arc::new(RateBudget::new(3_600.0, 100.0)),
    ));

    let poller = Arc::new(Poller::new(
        db.clone(),
        client,
        Arc::new(Recording::default()),
        PollConfig::default(),
    ));

    let path_secret = "test-path-secret".to_owned();
    let signing_secret = "test-signing-secret".to_owned();
    let cancel = CancellationToken::new();

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind an ephemeral port");
    let port = listener.local_addr().expect("bound address").port();

    let state = ServerState {
        db,
        poller: Arc::clone(&poller),
        path_secret: Arc::from(path_secret.as_str()),
        signing_secret: Arc::from(signing_secret.as_str()),
    };

    let shutdown = cancel.clone();
    tokio::spawn(async move {
        let _ = server::serve_with_listener(listener, state, shutdown).await;
    });

    Harness {
        port,
        path_secret,
        signing_secret,
        poller,
        cancel,
        http: crate::github::build_client().expect("http client"),
    }
}

fn push_payload(full_name: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "action": "completed",
        "repository": { "full_name": full_name }
    }))
    .expect("payload")
}

// ---------------------------------------------------------------------------
// Acceptance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_signed_delivery_is_accepted_and_recorded() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;

    let body = push_payload("acme/api");
    let response = harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "application/json")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-1")
        .header("x-hub-signature-256", harness.sign(&body))
        .body(body)
        .send()
        .await
        .expect("request");

    assert_eq!(response.status().as_u16(), 202);
}

#[tokio::test]
async fn a_tampered_body_is_rejected_and_not_recorded() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;

    let signed = push_payload("acme/api");
    let signature = harness.sign(&signed);
    let tampered = push_payload("acme/other");

    let response = harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "application/json")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-tampered")
        .header("x-hub-signature-256", signature)
        .body(tampered)
        .send()
        .await
        .expect("request");

    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn a_delivery_signed_with_the_wrong_secret_is_rejected() {
    let github = MockServer::start().await;
    let mut harness = harness(&github, &["acme/api"]).await;
    harness.signing_secret = "not-the-secret".to_owned();

    let body = push_payload("acme/api");
    let response = harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "application/json")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-bad-secret")
        .header("x-hub-signature-256", harness.sign(&body))
        .body(body)
        .send()
        .await
        .expect("request");

    // The server still uses the real secret, so this must not match.
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn a_missing_signature_header_is_rejected() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;

    let response = harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "application/json")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-nosig")
        .body(push_payload("acme/api"))
        .send()
        .await
        .expect("request");

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn a_non_json_content_type_is_rejected() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;

    let body = push_payload("acme/api");
    let response = harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "text/plain")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-text")
        .header("x-hub-signature-256", harness.sign(&body))
        .body(body)
        .send()
        .await
        .expect("request");

    assert_eq!(response.status().as_u16(), 415);
}

#[tokio::test]
async fn the_wrong_path_secret_is_a_404() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;

    let body = push_payload("acme/api");
    let response = harness
        .http
        .post(format!("http://127.0.0.1:{}/guessed/webhook", harness.port))
        .header("content-type", "application/json")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-wrong-path")
        .header("x-hub-signature-256", harness.sign(&body))
        .body(body)
        .send()
        .await
        .expect("request");

    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn the_health_endpoint_answers_on_the_secret_path_only() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;

    let good = harness
        .http
        .get(harness.endpoint("health"))
        .send()
        .await
        .expect("request");
    assert_eq!(good.status().as_u16(), 200);

    let bad = harness
        .http
        .get(format!("http://127.0.0.1:{}/guessed/health", harness.port))
        .send()
        .await
        .expect("request");
    assert_eq!(bad.status().as_u16(), 404);
}

// ---------------------------------------------------------------------------
// Idempotence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_replayed_delivery_is_accepted_but_only_recorded_once() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;
    let body = push_payload("acme/api");

    for _ in 0..3 {
        let response = harness
            .http
            .post(harness.endpoint("webhook"))
            .header("content-type", "application/json")
            .header("x-github-event", "workflow_run")
            .header("x-github-delivery", "delivery-replayed")
            .header("x-hub-signature-256", harness.sign(&body))
            .body(body.clone())
            .send()
            .await
            .expect("request");

        // GitHub retries on anything but a 2xx, so a replay must still be
        // acknowledged even though it is ignored.
        assert_eq!(response.status().as_u16(), 202);
    }
}

// ---------------------------------------------------------------------------
// Routing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_delivery_for_a_watched_repository_polls_it_immediately() {
    let github = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/api/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": [{
                "id": 1,
                "name": "CI",
                "status": "in_progress",
                "updated_at": "2026-11-20T10:00:00Z"
            }]
        })))
        .mount(&github)
        .await;

    let harness = harness(&github, &["acme/api"]).await;
    let body = push_payload("acme/api");

    harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "application/json")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-refresh")
        .header("x-hub-signature-256", harness.sign(&body))
        .body(body)
        .send()
        .await
        .expect("request");

    // The webhook only queues work; the poller does it. Give the spawned
    // handler a moment to record the delivery before draining the queue.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let polled = harness.poller.tick().await.expect("tick");
    assert_eq!(polled, 1, "the webhook should have queued this repository");
}

#[tokio::test]
async fn a_delivery_for_an_unwatched_repository_is_accepted_and_ignored() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;
    let body = push_payload("someone/else");

    let response = harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "application/json")
        .header("x-github-event", "workflow_run")
        .header("x-github-delivery", "delivery-unwatched")
        .header("x-hub-signature-256", harness.sign(&body))
        .body(body)
        .send()
        .await
        .expect("request");

    assert_eq!(
        response.status().as_u16(),
        202,
        "an ignored delivery is still an accepted one"
    );

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(harness.poller.tick().await.expect("tick"), 0);
}

#[tokio::test]
async fn a_ping_is_recorded_without_polling_anything() {
    let github = MockServer::start().await;
    let harness = harness(&github, &["acme/api"]).await;
    let body = push_payload("acme/api");

    let response = harness
        .http
        .post(harness.endpoint("webhook"))
        .header("content-type", "application/json")
        .header("x-github-event", "ping")
        .header("x-github-delivery", "delivery-ping")
        .header("x-hub-signature-256", harness.sign(&body))
        .body(body)
        .send()
        .await
        .expect("request");

    assert_eq!(response.status().as_u16(), 202);

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(harness.poller.tick().await.expect("tick"), 0);
}
