use std::sync::{Arc, Mutex};

use chrono::{Duration as ChronoDuration, Utc};
use secrecy::ExposeSecret;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::error::AppError;
use crate::github::client::Endpoints;

use super::device_flow;
use super::oauth::{OAuthClient, PollOutcome, TokenResponse, Viewer};
use super::token_store::{secret, MemoryStore, StoredTokens, TokenMetadata, TokenStore};
use super::tokens::TokenManager;
use super::types::AuthState;

const DEVICE_CODE_PATH: &str = "/login/device/code";
const TOKEN_PATH: &str = "/login/oauth/access_token";
const USER_PATH: &str = "/user";

fn oauth_client(server: &MockServer, with_client_id: bool) -> OAuthClient {
    let http = crate::github::client::build_client().expect("http client");
    let endpoints = Endpoints::single_origin(&server.uri());
    let client_id = with_client_id.then(|| "test-client".to_owned());

    OAuthClient::new(http, endpoints, client_id)
}

/// A client that will never be used for a request.
fn offline_client() -> Arc<OAuthClient> {
    let http = crate::github::client::build_client().expect("http client");
    Arc::new(OAuthClient::new(
        http,
        Endpoints::github(),
        Some("test-client".to_owned()),
    ))
}

async fn mount_device_code(server: &MockServer, interval_secs: u64, expires_in: i64) {
    Mock::given(method("POST"))
        .and(path(DEVICE_CODE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "device-1",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": expires_in,
            "interval": interval_secs,
        })))
        .mount(server)
        .await;
}

/// Mounts a token-endpoint response. `times` caps how many requests it
/// answers, which is how a "pending, then granted" sequence is expressed:
/// wiremock falls through to the next matching mock once a cap is reached.
async fn mount_token(server: &MockServer, body: serde_json::Value, times: Option<u64>) {
    let mut mock = Mock::given(method("POST"))
        .and(path(TOKEN_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body));

    if let Some(times) = times {
        mock = mock.up_to_n_times(times);
    }

    mock.mount(server).await;
}

fn granted(outcome: Result<PollOutcome, AppError>) -> TokenResponse {
    match outcome {
        Ok(PollOutcome::Granted(tokens)) => *tokens,
        Ok(other) => panic!("expected a grant, got {other:?}"),
        Err(error) => panic!("expected a grant, got an error: {error}"),
    }
}

fn expired_grant() -> TokenResponse {
    TokenResponse {
        access_token: "old-access".to_owned(),
        refresh_token: Some("refresh-1".to_owned()),
        expires_at: Some(Utc::now() - ChronoDuration::hours(1)),
        refresh_expires_at: None,
        scopes: vec!["repo".to_owned()],
    }
}

fn viewer(login: &str) -> Viewer {
    Viewer {
        login: login.to_owned(),
        avatar_url: None,
        display_name: None,
    }
}

// ---------------------------------------------------------------------------
// Device flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn device_flow_polls_until_the_user_approves() {
    let server = MockServer::start().await;
    mount_device_code(&server, 1, 900).await;

    // First poll: the user has not finished. Second: granted.
    mount_token(
        &server,
        json!({ "error": "authorization_pending" }),
        Some(1),
    )
    .await;
    mount_token(
        &server,
        json!({
            "access_token": "ghu_access",
            "scope": "repo, workflow",
            "expires_in": 28_800,
            "refresh_token": "ghr_refresh",
            "refresh_token_expires_in": 15_811_200,
        }),
        None,
    )
    .await;

    let client = oauth_client(&server, true);
    let cancel = CancellationToken::new();
    let states = Mutex::new(Vec::<AuthState>::new());
    let opened = Mutex::new(None::<String>);

    let tokens = device_flow::run(
        &client,
        &[],
        &cancel,
        |state| states.lock().expect("states").push(state),
        |url| {
            *opened.lock().expect("opened") = Some(url.to_owned());
            Ok(())
        },
    )
    .await
    .expect("the flow completes");

    assert_eq!(tokens.access_token, "ghu_access");
    assert_eq!(tokens.refresh_token.as_deref(), Some("ghr_refresh"));
    assert!(tokens.can_refresh());
    assert!(tokens.expires_at.is_some());
    assert_eq!(
        tokens.scopes,
        vec!["repo".to_owned(), "workflow".to_owned()]
    );

    assert_eq!(
        opened.lock().expect("opened").as_deref(),
        Some("https://github.com/login/device"),
        "the browser must be sent to the verification URI"
    );

    let states = states.into_inner().expect("states");
    assert!(matches!(states[0], AuthState::RequestingCode));
    assert!(
        matches!(&states[1], AuthState::AwaitingUser { user_code, .. } if user_code == "ABCD-1234"),
        "the user code must reach the UI, got {:?}",
        states[1]
    );
    assert!(states
        .iter()
        .any(|state| matches!(state, AuthState::Polling { .. })));
}

#[tokio::test]
async fn poll_maps_every_documented_error_code() {
    let cases = [
        ("authorization_pending", "pending"),
        ("slow_down", "slow-down"),
        ("access_denied", "auth_denied"),
        ("expired_token", "auth_expired"),
        ("incorrect_device_code", "config"),
        ("device_flow_disabled", "config"),
        ("incorrect_client_credentials", "config"),
        ("something_new_github_added", "internal"),
    ];

    for (code, expected) in cases {
        let server = MockServer::start().await;
        mount_token(
            &server,
            json!({ "error": code, "error_description": "from the test" }),
            None,
        )
        .await;

        let client = oauth_client(&server, true);
        let outcome = client.poll_device_code("device-1").await;

        match expected {
            "pending" => assert!(
                matches!(outcome, Ok(PollOutcome::Pending)),
                "{code} should keep polling"
            ),
            "slow-down" => assert!(
                matches!(outcome, Ok(PollOutcome::SlowDown)),
                "{code} should back off"
            ),
            other => {
                let error = outcome.expect_err("{code} should be an error");
                assert_eq!(error.code(), other, "unexpected mapping for {code}");
            }
        }
    }
}

#[tokio::test]
async fn cancelling_the_flow_stops_polling() {
    let server = MockServer::start().await;
    mount_device_code(&server, 1, 900).await;
    mount_token(&server, json!({ "error": "authorization_pending" }), None).await;

    let client = oauth_client(&server, true);
    let cancel = CancellationToken::new();
    let cancellation = cancel.clone();

    // Abandon the flow the moment polling starts, which is what the UI's
    // cancel button does.
    let result = device_flow::run(
        &client,
        &[],
        &cancel,
        |state| {
            if matches!(state, AuthState::Polling { .. }) {
                cancellation.cancel();
            }
        },
        |_| Ok(()),
    )
    .await;

    assert!(
        matches!(result, Err(AppError::AuthCancelled)),
        "a cancelled flow must not be reported as a failure"
    );
}

#[tokio::test]
async fn an_expired_device_code_is_reported_as_expired() {
    let server = MockServer::start().await;
    // Already expired when the flow starts.
    mount_device_code(&server, 1, 0).await;

    let client = oauth_client(&server, true);
    let result =
        device_flow::run(&client, &[], &CancellationToken::new(), |_| {}, |_| Ok(())).await;

    assert!(matches!(result, Err(AppError::AuthExpired)));
}

#[tokio::test]
async fn a_token_without_a_refresh_token_is_long_lived() {
    // This is the OAuth App shape, and the reason the auth model does not need
    // a code fork: the flow branches on what GitHub actually returns.
    let server = MockServer::start().await;
    mount_token(
        &server,
        json!({ "access_token": "gho_token", "scope": "repo" }),
        None,
    )
    .await;

    let client = oauth_client(&server, true);
    let tokens = granted(client.poll_device_code("device-1").await);

    assert!(!tokens.can_refresh());
    assert!(tokens.expires_at.is_none());
}

#[tokio::test]
async fn a_missing_client_id_fails_before_reaching_the_network() {
    let server = MockServer::start().await;
    let client = oauth_client(&server, false);

    let error = client
        .request_device_code(&[])
        .await
        .expect_err("no client id is configured");

    assert_eq!(error.code(), "config");
    assert_eq!(
        server.received_requests().await.expect("requests").len(),
        0,
        "we must not call GitHub without a client id"
    );
}

// ---------------------------------------------------------------------------
// Token custody
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_revoked_token_is_reported_as_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USER_PATH))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({ "message": "Bad credentials" })),
        )
        .mount(&server)
        .await;

    let client = oauth_client(&server, true);
    let error = client
        .fetch_viewer("ghu_revoked")
        .await
        .expect_err("a 401 is a revocation");

    assert_eq!(error.code(), "token_revoked");
}

#[tokio::test]
async fn validate_clears_a_revoked_token_and_asks_for_sign_in() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USER_PATH))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({ "message": "Bad credentials" })),
        )
        .mount(&server)
        .await;

    let store = Arc::new(MemoryStore::new());
    let manager = TokenManager::new(Arc::new(oauth_client(&server, true)), store.clone());

    manager
        .adopt(
            &TokenResponse {
                access_token: "ghu_access".to_owned(),
                refresh_token: None,
                expires_at: None,
                refresh_expires_at: None,
                scopes: Vec::new(),
            },
            &viewer("octocat"),
        )
        .expect("adopt");

    let error = manager.validate().await.expect_err("revoked");
    assert_eq!(error.code(), "auth_required");
    assert!(
        store.load().expect("load").is_none(),
        "a revoked token must not be left in the store"
    );
    assert!(manager.session().is_none());
}

#[tokio::test]
async fn twenty_concurrent_callers_cause_one_refresh() {
    let server = MockServer::start().await;
    mount_token(
        &server,
        json!({
            "access_token": "new-access",
            "refresh_token": "refresh-2",
            "expires_in": 28_800,
        }),
        None,
    )
    .await;

    let store = Arc::new(MemoryStore::new());
    let manager = Arc::new(TokenManager::new(
        Arc::new(oauth_client(&server, true)),
        store.clone(),
    ));

    // Seed an already-expired grant so every caller wants a refresh.
    manager
        .adopt(&expired_grant(), &viewer("octocat"))
        .expect("adopt");

    let mut handles = Vec::new();
    for _ in 0..20 {
        let manager = Arc::clone(&manager);
        handles.push(tokio::spawn(async move {
            manager
                .access_token()
                .await
                .map(|token| token.expose_secret().to_owned())
        }));
    }

    let mut seen = Vec::new();
    for handle in handles {
        seen.push(handle.await.expect("join").expect("a usable token"));
    }

    assert!(
        seen.iter().all(|token| token == "new-access"),
        "every caller must see the refreshed token"
    );

    let requests = server.received_requests().await.expect("requests");
    let refreshes = requests
        .iter()
        .filter(|request| request.url.path() == TOKEN_PATH)
        .count();

    assert_eq!(
        refreshes, 1,
        "20 concurrent callers must collapse into a single refresh"
    );

    // The rotated token must have replaced the spent one.
    let stored = store.load().expect("load").expect("still signed in");
    let rotated = stored.refresh_token.expect("refresh token");
    assert_eq!(rotated.expose_secret(), "refresh-2");
}

#[tokio::test]
async fn an_expired_token_without_a_refresh_token_requires_sign_in() {
    let store = Arc::new(MemoryStore::new());
    let manager = TokenManager::new(offline_client(), store.clone());

    manager
        .adopt(
            &TokenResponse {
                access_token: "gho_access".to_owned(),
                refresh_token: None,
                expires_at: Some(Utc::now() - ChronoDuration::hours(1)),
                refresh_expires_at: None,
                scopes: Vec::new(),
            },
            &viewer("octocat"),
        )
        .expect("adopt");

    let error = manager
        .access_token()
        .await
        .expect_err("an unrefreshable expired token cannot be used");

    assert_eq!(error.code(), "auth_required");
}

#[tokio::test]
async fn a_fresh_token_is_reused_without_a_refresh() {
    let server = MockServer::start().await;
    let store = Arc::new(MemoryStore::new());
    let manager = TokenManager::new(Arc::new(oauth_client(&server, true)), store);

    manager
        .adopt(
            &TokenResponse {
                access_token: "still-good".to_owned(),
                refresh_token: Some("refresh-1".to_owned()),
                expires_at: Some(Utc::now() + ChronoDuration::hours(4)),
                refresh_expires_at: None,
                scopes: Vec::new(),
            },
            &viewer("octocat"),
        )
        .expect("adopt");

    let token = manager.access_token().await.expect("token");
    assert_eq!(token.expose_secret(), "still-good");

    assert_eq!(
        server.received_requests().await.expect("requests").len(),
        0,
        "a token with hours of life left must not trigger a refresh"
    );
}

#[test]
fn hydrate_restores_a_session_from_the_store() {
    let store = Arc::new(MemoryStore::new());
    let manager = TokenManager::new(offline_client(), store.clone());

    manager
        .adopt(
            &TokenResponse {
                access_token: "ghu_access".to_owned(),
                refresh_token: Some("ghr_refresh".to_owned()),
                expires_at: Some(Utc::now() + ChronoDuration::hours(8)),
                refresh_expires_at: None,
                scopes: vec!["repo".to_owned()],
            },
            &viewer("octocat"),
        )
        .expect("adopt");

    // A fresh manager is what a relaunched app has.
    let restarted = TokenManager::new(offline_client(), store);
    let session = restarted.hydrate().expect("hydrate").expect("signed in");

    assert_eq!(session.login, "octocat");
    assert!(session.can_refresh);
    assert_eq!(session.scopes, vec!["repo".to_owned()]);
    assert!(restarted.session().is_some());
}

#[test]
fn signing_out_clears_the_store() {
    let store = Arc::new(MemoryStore::new());
    let manager = TokenManager::new(offline_client(), store.clone());

    manager
        .adopt(&expired_grant(), &viewer("octocat"))
        .expect("adopt");
    assert!(store.load().expect("load").is_some());

    manager.sign_out().expect("sign out");

    assert!(store.load().expect("load").is_none());
    assert!(manager.session().is_none());
}

#[test]
fn tokens_are_persisted_refresh_first() {
    // The write order is the crash-safety property: GitHub invalidates the old
    // refresh token as soon as it issues a new one, so the refresh token must
    // land before the access token.
    let store = MemoryStore::new();

    store
        .save(&StoredTokens {
            access_token: secret("access"),
            refresh_token: Some(secret("refresh")),
            metadata: TokenMetadata {
                login: Some("octocat".to_owned()),
                ..TokenMetadata::default()
            },
        })
        .expect("save");

    assert_eq!(
        store.write_log(),
        vec![
            "github-refresh-token",
            "github-token-metadata",
            "github-access-token",
        ]
    );
}

#[test]
fn a_long_lived_grant_stores_no_refresh_token() {
    let store = MemoryStore::new();

    store
        .save(&StoredTokens {
            access_token: secret("access"),
            refresh_token: None,
            metadata: TokenMetadata::default(),
        })
        .expect("save");

    assert_eq!(
        store.write_log(),
        vec!["github-token-metadata", "github-access-token"],
        "with nothing to rotate, there is no refresh write to make"
    );

    let loaded = store.load().expect("load").expect("present");
    assert!(loaded.refresh_token.is_none());
}

#[test]
fn unreadable_metadata_does_not_lock_the_user_out() {
    let store = MemoryStore::new();
    store
        .seed(Some("access"), None, Some("{ this is not json"))
        .expect("seed");

    let loaded = store
        .load()
        .expect("load")
        .expect("the tokens are still usable");

    assert_eq!(loaded.access_token.expose_secret(), "access");
    assert!(loaded.metadata.login.is_none());
}
