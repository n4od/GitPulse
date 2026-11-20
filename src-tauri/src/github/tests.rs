use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use secrecy::SecretString;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::token_store::secret;
use crate::error::AppError;

use super::api::{GithubClient, TokenProvider};
use super::budget::RateBudget;
use super::client::{self, Endpoints};

struct StaticToken(SecretString);

#[async_trait]
impl TokenProvider for StaticToken {
    async fn access_token(&self) -> Result<SecretString, AppError> {
        Ok(self.0.clone())
    }
}

fn setup(server: &MockServer) -> (GithubClient, Arc<RateBudget>) {
    let budget = Arc::new(RateBudget::new(3_600.0, 100.0));

    let client = GithubClient::new(
        client::build_client().expect("http client"),
        Endpoints::single_origin(&server.uri()),
        Arc::new(StaticToken(secret("ghu_test_token"))),
        Arc::clone(&budget),
    );

    (client, budget)
}

async fn request_count(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .map_or(0, |requests| requests.len())
}

fn repo_payload() -> Value {
    json!([{
        "id": 9,
        "name": "api",
        "full_name": "acme/api",
        "owner": { "login": "acme" },
        "private": true,
        "default_branch": "main"
    }])
}

// ---------------------------------------------------------------------------
// Conditional requests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_304_serves_the_cached_body_and_costs_no_budget() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/api"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"v1\"")
                .set_body_json(json!({
                    "id": 9,
                    "name": "api",
                    "full_name": "acme/api",
                    "owner": { "login": "acme" },
                    "default_branch": "main"
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/api"))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;

    let (client, budget) = setup(&server);

    let first = client.repo("acme/api").await.expect("first fetch");
    assert_eq!(first.id, 9);

    let second = client.repo("acme/api").await.expect("served from cache");
    assert_eq!(second.id, 9);

    assert_eq!(
        budget.granted(),
        1,
        "a 304 must refund its local budget token"
    );

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1]
            .headers
            .get("if-none-match")
            .map(|value| value.to_str().expect("ascii")),
        Some("\"v1\""),
        "the second request must revalidate rather than refetch"
    );
}

#[tokio::test]
async fn every_request_carries_the_api_version_and_a_bearer_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_payload()))
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    client.repos(1, 10).await.expect("repos");

    let requests = server.received_requests().await.expect("requests");
    let headers = &requests[0].headers;

    assert_eq!(
        headers
            .get("x-github-api-version")
            .map(|v| v.to_str().unwrap()),
        Some("2022-11-28")
    );
    assert_eq!(
        headers.get("authorization").map(|v| v.to_str().unwrap()),
        Some("Bearer ghu_test_token")
    );
}

// ---------------------------------------------------------------------------
// Rate limiting
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_secondary_limit_pauses_the_whole_client() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(403).insert_header("retry-after", "120"))
        .mount(&server)
        .await;

    let (client, budget) = setup(&server);

    let error = client.repos(1, 10).await.expect_err("secondary limit");
    assert_eq!(error.code(), "secondary_rate_limited");
    assert!(
        budget.snapshot().paused_until.is_some(),
        "the pause must be recorded so other requests back off too"
    );

    // 120s exceeds the inline wait, so the second call is refused locally
    // instead of blocking for two minutes.
    let second = client.repos(1, 10).await.expect_err("still paused");
    assert_eq!(second.code(), "rate_limited");

    assert_eq!(
        request_count(&server).await,
        1,
        "a paused client must not reach the network"
    );
}

#[tokio::test]
async fn primary_exhaustion_reports_the_reset_time() {
    let server = MockServer::start().await;
    let reset = Utc::now().timestamp() + 3_600;

    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("x-ratelimit-remaining", "0")
                .insert_header("x-ratelimit-reset", reset.to_string()),
        )
        .mount(&server)
        .await;

    let (client, _) = setup(&server);

    let error = client.repos(1, 10).await.expect_err("primary exhausted");
    assert_eq!(error.code(), "rate_limited");

    match error {
        AppError::RateLimited { reset_at } => {
            assert_eq!(reset_at.timestamp(), reset);
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn a_429_is_treated_as_a_secondary_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "30"))
        .mount(&server)
        .await;

    let (client, budget) = setup(&server);
    let error = client.repos(1, 10).await.expect_err("429");

    assert_eq!(error.code(), "secondary_rate_limited");
    assert!(budget.snapshot().paused_until.is_some());
}

#[tokio::test]
async fn a_rate_limit_event_is_published() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ratelimit-limit", "5000")
                .insert_header("x-ratelimit-remaining", "4321")
                .insert_header("x-ratelimit-used", "679")
                .set_body_json(repo_payload()),
        )
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    let mut rate = client.rate_receiver();

    client.repos(1, 10).await.expect("repos");

    let snapshot = rate.borrow_and_update().clone();
    assert_eq!(snapshot.limit, Some(5_000));
    assert_eq!(snapshot.remaining, Some(4_321));
    assert_eq!(snapshot.used, Some(679));
}

// ---------------------------------------------------------------------------
// Retry policy
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_server_error_is_retried() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_payload()))
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    let repos = client.repos(1, 10).await.expect("the retry succeeds");

    assert_eq!(repos.len(), 1);
    assert_eq!(request_count(&server).await, 2);
}

#[tokio::test]
async fn a_client_error_is_not_retried() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/api"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    let error = client.repo("acme/api").await.expect_err("404");

    assert_eq!(error.code(), "not_found");
    assert_eq!(
        request_count(&server).await,
        1,
        "retrying a 404 would just burn budget"
    );
}

#[tokio::test]
async fn a_revoked_token_is_reported_as_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    let error = client.repos(1, 10).await.expect_err("401");

    assert_eq!(error.code(), "token_revoked");
}

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_traversal_attempt_never_reaches_the_network() {
    let server = MockServer::start().await;
    let (client, _) = setup(&server);

    let error = client.repo("../../admin").await.expect_err("rejected");
    assert_eq!(error.code(), "config");
    assert_eq!(request_count(&server).await, 0);
}

#[tokio::test]
async fn an_invalid_ref_is_rejected_before_dispatch() {
    let server = MockServer::start().await;
    let (client, _) = setup(&server);

    let error = client
        .dispatch("acme/api", 1, "feature/../main", &serde_json::Map::new())
        .await
        .expect_err("rejected");

    assert_eq!(error.code(), "config");
    assert_eq!(request_count(&server).await, 0);
}

#[tokio::test]
async fn dispatch_sends_the_ref_and_inputs() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/repos/acme/api/actions/workflows/7/dispatches"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    let mut inputs = serde_json::Map::new();
    inputs.insert("environment".to_owned(), json!("staging"));

    client
        .dispatch("acme/api", 7, "release/1.2", &inputs)
        .await
        .expect("dispatched");

    let requests = server.received_requests().await.expect("requests");
    let body: Value = serde_json::from_slice(&requests[0].body).expect("json body");

    assert_eq!(body["ref"], "release/1.2");
    assert_eq!(body["inputs"]["environment"], "staging");
}

// ---------------------------------------------------------------------------
// Log download
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_log_redirect_is_followed_without_the_authorization_header() {
    let server = MockServer::start().await;
    let cdn_url = format!("{}/cdn/archive", server.uri());

    Mock::given(method("GET"))
        .and(path("/logs"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", cdn_url.as_str()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cdn/archive"))
        .respond_with(ResponseTemplate::new(200).set_body_string("line1\nline2\n"))
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    let download = client
        .download_log(&format!("{}/logs", server.uri()))
        .await
        .expect("downloaded");

    assert_eq!(download.text, "line1\nline2\n");
    assert!(!download.truncated);

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert!(
        requests[0].headers.get("authorization").is_some(),
        "the API request is authenticated"
    );
    assert!(
        requests[1].headers.get("authorization").is_none(),
        "the CDN request must never carry the token"
    );
}

#[tokio::test]
async fn a_log_longer_than_the_cap_is_truncated() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_string("0123456789"))
        .mount(&server)
        .await;

    let (client, _) = setup(&server);
    let download = client
        .download_log_capped(&format!("{}/big", server.uri()), 4)
        .await
        .expect("downloaded");

    assert_eq!(download.text, "0123");
    assert!(download.truncated);
}

// ---------------------------------------------------------------------------
// Budget integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_burst_is_paced_by_the_local_budget() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 9,
            "name": "api",
            "full_name": "acme/api",
            "owner": { "login": "acme" }
        })))
        .mount(&server)
        .await;

    // One token per second: the second call must wait for the bucket.
    let budget = Arc::new(RateBudget::new(3_600.0, 1.0));
    let client = GithubClient::new(
        client::build_client().expect("http client"),
        Endpoints::single_origin(&server.uri()),
        Arc::new(StaticToken(secret("ghu_test_token"))),
        Arc::clone(&budget),
    );

    client.repo("acme/api").await.expect("first");

    let started = std::time::Instant::now();
    client.repo("acme/api").await.expect("second");
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_millis(900),
        "the local bucket should have paced the second call, took {elapsed:?}"
    );
    assert_eq!(budget.granted(), 2);
}
