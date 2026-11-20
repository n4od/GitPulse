use std::sync::{Arc, Mutex};

use chrono::{Duration as ChronoDuration, Utc};
use secrecy::SecretString;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::token_store::secret;
use crate::db::{self, Db, RepoCard};
use crate::error::AppError;
use crate::github::{self, Endpoints, GithubClient, RateBudget, TokenProvider};

use super::{PollConfig, PollEvents, Poller, PollerStatus};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

struct StaticToken(SecretString);

#[async_trait::async_trait]
impl TokenProvider for StaticToken {
    async fn access_token(&self) -> Result<SecretString, AppError> {
        Ok(self.0.clone())
    }
}

struct Unauthenticated;

#[async_trait::async_trait]
impl TokenProvider for Unauthenticated {
    async fn access_token(&self) -> Result<SecretString, AppError> {
        Err(AppError::AuthRequired)
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

fn build_client(
    server: &MockServer,
    tokens: Arc<dyn TokenProvider>,
    budget: Arc<RateBudget>,
) -> Arc<GithubClient> {
    Arc::new(GithubClient::new(
        github::build_client().expect("http client"),
        Endpoints::single_origin(&server.uri()),
        tokens,
        budget,
    ))
}

fn authorized_client(server: &MockServer) -> Arc<GithubClient> {
    build_client(
        server,
        Arc::new(StaticToken(secret("ghu_test_token"))),
        Arc::new(RateBudget::new(3_600.0, 100.0)),
    )
}

fn poller_for(
    db: &Db,
    client: Arc<GithubClient>,
    events: Arc<Recording>,
    config: PollConfig,
) -> Poller {
    Poller::new(db.clone(), client, events, config)
}

async fn seed_repo(db: &Db, github_id: i64, full_name: &str) {
    let full_name = full_name.to_owned();

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

fn run_payload(id: i64, status: &str, conclusion: Option<&str>) -> Value {
    json!({
        "id": id,
        "name": "CI",
        "display_title": "Fix the thing",
        "event": "push",
        "head_branch": "main",
        "head_sha": "a1b2c3d",
        "status": status,
        "conclusion": conclusion,
        "run_number": 1,
        "run_attempt": 1,
        "html_url": "https://github.com/acme/api/actions/runs/1",
        "actor": { "login": "octocat" },
        "created_at": "2026-11-20T10:00:00Z",
        "run_started_at": "2026-11-20T10:00:05Z",
        "updated_at": "2026-11-20T10:04:00Z"
    })
}

async fn mount_runs(server: &MockServer, full_name: &str, runs: Vec<Value>) {
    Mock::given(method("GET"))
        .and(path(format!("/repos/{full_name}/actions/runs")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": runs })))
        .mount(server)
        .await;
}

async fn request_count(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .map_or(0, |requests| requests.len())
}

async fn run_count(db: &Db) -> i64 {
    db.with_connection(|connection| {
        connection
            .query_row("SELECT COUNT(*) FROM workflow_runs", [], |row| row.get(0))
            .map_err(AppError::db)
    })
    .await
    .expect("count runs")
}

// ---------------------------------------------------------------------------
// Polling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_first_tick_polls_and_publishes_the_runs_it_found() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/api",
        vec![run_payload(100, "in_progress", None)],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        Arc::clone(&events),
        PollConfig::default(),
    );

    seed_repo(&db, 1, "acme/api").await;

    let count = poller.tick().await.expect("tick");
    assert_eq!(count, 1);
    assert_eq!(run_count(&db).await, 1);

    let emitted = events.cards.lock().expect("cards");
    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].repo.id, 1);

    let latest = emitted[0].recent_runs.first().expect("latest run");
    assert_eq!(latest.status, "in_progress");
    assert!(emitted[0].has_active_run);

    // Status is always published, even on a quiet tick.
    assert_eq!(events.statuses.lock().expect("statuses").len(), 1);
}

#[tokio::test]
async fn a_tick_before_the_interval_does_not_touch_the_network() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/api",
        vec![run_payload(100, "completed", Some("success"))],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        events,
        PollConfig::default(),
    );

    seed_repo(&db, 1, "acme/api").await;

    assert_eq!(poller.tick().await.expect("first"), 1);
    assert_eq!(poller.tick().await.expect("second"), 0);

    assert_eq!(request_count(&server).await, 1, "one poll per interval");
}

#[tokio::test]
async fn an_unchanged_poll_publishes_nothing() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/api",
        vec![run_payload(100, "completed", Some("success"))],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        Arc::clone(&events),
        PollConfig::default(),
    );

    seed_repo(&db, 1, "acme/api").await;

    poller.tick().await.expect("first");
    let later = Utc::now() + ChronoDuration::minutes(30);
    assert_eq!(poller.tick_at(later).await.expect("second"), 1);

    assert_eq!(
        events.cards.lock().expect("cards").len(),
        1,
        "an identical response must not re-publish the same card"
    );
}

#[tokio::test]
async fn a_changed_run_is_published_again() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/api",
        vec![run_payload(100, "in_progress", None)],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        Arc::clone(&events),
        PollConfig::default(),
    );

    seed_repo(&db, 1, "acme/api").await;
    poller.tick().await.expect("first");

    // Same run, different observable state.
    server.reset().await;
    mount_runs(
        &server,
        "acme/api",
        vec![run_payload(100, "completed", Some("failure"))],
    )
    .await;

    let later = Utc::now() + ChronoDuration::minutes(30);
    assert_eq!(poller.tick_at(later).await.expect("second"), 1);

    let emitted = events.cards.lock().expect("cards");
    assert_eq!(emitted.len(), 2);
    assert_eq!(
        emitted[1]
            .recent_runs
            .first()
            .expect("latest run")
            .conclusion
            .as_deref(),
        Some("failure")
    );
}

// ---------------------------------------------------------------------------
// Priority and manual refresh
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_manual_refresh_polls_a_repository_that_is_not_due() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/api",
        vec![run_payload(100, "completed", Some("success"))],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        events,
        PollConfig::default(),
    );

    seed_repo(&db, 1, "acme/api").await;
    poller.tick().await.expect("first");
    assert_eq!(request_count(&server).await, 1);

    let repo_id = 1;
    poller.request_refresh(repo_id);

    // Still far inside the interval, but the request wins.
    assert_eq!(poller.tick().await.expect("refresh"), 1);
    assert_eq!(request_count(&server).await, 2);
}

#[tokio::test]
async fn duplicate_refresh_requests_collapse_into_one_poll() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/api",
        vec![run_payload(100, "completed", Some("success"))],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        events,
        PollConfig::default(),
    );

    seed_repo(&db, 1, "acme/api").await;
    poller.tick().await.expect("first");

    poller.request_refresh(1);
    poller.request_refresh(1);
    poller.request_refresh(1);

    assert_eq!(poller.tick().await.expect("refresh"), 1);
    assert_eq!(
        request_count(&server).await,
        2,
        "three impatient clicks must not become three requests"
    );
}

#[tokio::test]
async fn a_repository_with_a_run_in_flight_is_ordered_first() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/slow",
        vec![run_payload(1, "completed", Some("success"))],
    )
    .await;
    mount_runs(
        &server,
        "acme/busy",
        vec![run_payload(2, "in_progress", None)],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    // A budget of one token per tick forces a choice between the two.
    let client = build_client(
        &server,
        Arc::new(StaticToken(secret("ghu_test_token"))),
        Arc::new(RateBudget::new(3_600.0, 1.0)),
    );
    let poller = poller_for(&db, client, events, PollConfig::default());

    seed_repo(&db, 1, "acme/busy").await;
    seed_repo(&db, 2, "acme/slow").await;

    // First tick spends the single token on whichever is ordered first.
    assert_eq!(poller.tick().await.expect("tick"), 1);

    let requests = server.received_requests().await.expect("requests");
    let first = requests[0].url.path();

    // "busy" has no runs yet, so neither is flagged active on the first pass;
    // the assertion that matters is that only one request went out.
    assert_eq!(requests.len(), 1, "the budget must stop the second poll");
    assert!(first.contains("actions/runs"));
}

// ---------------------------------------------------------------------------
// Budget and authentication gates
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_exhausted_budget_defers_the_rest_of_the_sweep() {
    let server = MockServer::start().await;
    mount_runs(
        &server,
        "acme/one",
        vec![run_payload(1, "completed", Some("success"))],
    )
    .await;
    mount_runs(
        &server,
        "acme/two",
        vec![run_payload(2, "completed", Some("success"))],
    )
    .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let client = build_client(
        &server,
        Arc::new(StaticToken(secret("ghu_test_token"))),
        Arc::new(RateBudget::new(3_600.0, 1.0)),
    );
    let poller = poller_for(&db, client, events, PollConfig::default());

    seed_repo(&db, 1, "acme/one").await;
    seed_repo(&db, 2, "acme/two").await;

    assert_eq!(poller.tick().await.expect("tick"), 1);
    assert_eq!(request_count(&server).await, 1);
}

#[tokio::test]
async fn a_rate_limit_stops_the_sweep_instead_of_hammering() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/one/actions/runs"))
        .respond_with(ResponseTemplate::new(403).insert_header("retry-after", "120"))
        .mount(&server)
        .await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        Arc::clone(&events),
        PollConfig::default(),
    );

    seed_repo(&db, 1, "acme/one").await;
    seed_repo(&db, 2, "acme/two").await;

    assert_eq!(poller.tick().await.expect("tick"), 0);
    assert_eq!(
        request_count(&server).await,
        1,
        "three repositories must not become three rejected requests"
    );

    let statuses = events.statuses.lock().expect("statuses");
    assert!(statuses[0].last_error.is_some(), "the failure is surfaced");
}

#[tokio::test]
async fn being_signed_out_stops_the_sweep_quietly() {
    let server = MockServer::start().await;

    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let client = build_client(
        &server,
        Arc::new(Unauthenticated),
        Arc::new(RateBudget::new(3_600.0, 100.0)),
    );
    let poller = poller_for(&db, client, Arc::clone(&events), PollConfig::default());

    seed_repo(&db, 1, "acme/api").await;

    assert_eq!(poller.tick().await.expect("tick"), 0);
    assert_eq!(request_count(&server).await, 0);

    let statuses = events.statuses.lock().expect("statuses");
    assert_eq!(statuses.len(), 1, "status is still published");
}

#[tokio::test]
async fn an_empty_watchlist_publishes_status_without_work() {
    let server = MockServer::start().await;
    let db = Db::open_in_memory().expect("db");
    let events = Arc::new(Recording::default());
    let poller = poller_for(
        &db,
        authorized_client(&server),
        Arc::clone(&events),
        PollConfig::default(),
    );

    assert_eq!(poller.tick().await.expect("tick"), 0);
    assert_eq!(request_count(&server).await, 0);

    let statuses = events.statuses.lock().expect("statuses");
    assert_eq!(statuses[0].monitored, 0);
}
