use std::time::{Duration as StdDuration, Instant};

use chrono::{Duration, Utc};
use rusqlite::Connection;

use super::models::{DeploymentUpsert, JobUpsert, PullRequestUpsert, RepoUpsert, RunUpsert};
use super::{cache, diff, meta, migrate, pulls, repos, retention, runs, Db};
use crate::error::AppError;

fn connection() -> Connection {
    let connection = Connection::open_in_memory().expect("open in-memory database");
    // Mirrors what `Db::prepare` does for the connection, minus WAL (which an
    // in-memory database cannot use).
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign keys");
    migrate::apply(&connection).expect("apply migrations");
    connection
}

fn repo_upsert(github_id: i64, full_name: &str) -> RepoUpsert {
    let (owner, name) = full_name.split_once('/').expect("owner/name");

    RepoUpsert {
        github_id,
        owner: owner.to_owned(),
        name: name.to_owned(),
        full_name: full_name.to_owned(),
        is_private: true,
        default_branch: Some("main".to_owned()),
        avatar_url: None,
    }
}

fn run_upsert(repo_id: i64, github_id: i64, status: &str, conclusion: Option<&str>) -> RunUpsert {
    RunUpsert {
        repo_id,
        github_id,
        workflow_name: Some("ci".to_owned()),
        display_title: Some("Fix the thing".to_owned()),
        event: Some("push".to_owned()),
        branch: Some("main".to_owned()),
        head_sha: Some("a1b2c3d4".to_owned()),
        status: status.to_owned(),
        conclusion: conclusion.map(str::to_owned),
        run_number: Some(42),
        run_attempt: Some(1),
        html_url: Some("https://github.com/acme/api/actions/runs/1".to_owned()),
        actor_login: Some("octocat".to_owned()),
        created_at: Some(Utc::now()),
        run_started_at: Some(Utc::now()),
        updated_at: Utc::now(),
    }
}

#[test]
fn migrations_apply_once_and_are_recorded() {
    let connection = connection();

    assert_eq!(
        migrate::current_version(&connection).expect("version"),
        Some("0006_drop_webhook_settings".to_owned())
    );

    // Re-running must be a no-op rather than an error about existing tables.
    migrate::apply(&connection).expect("second apply");

    let applied: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(applied, 6);
}

#[test]
fn a_poll_error_is_recorded_and_cleared_per_repository() {
    let connection = connection();
    let id = repos::upsert(&connection, &repo_upsert(1, "acme/api")).expect("insert repo");

    repos::set_last_error(&connection, id, Some(("forbidden", "403 forbidden"))).expect("record");

    let recorded = repos::by_id(&connection, id)
        .expect("read")
        .expect("present");
    assert_eq!(recorded.last_error.as_deref(), Some("403 forbidden"));
    assert_eq!(recorded.last_error_code.as_deref(), Some("forbidden"));
    assert!(
        recorded.last_error_at.is_some(),
        "a failure with no time cannot be acted on"
    );

    // A later success clears all three, so nothing stale survives the fix.
    repos::set_last_error(&connection, id, None).expect("clear");

    let cleared = repos::by_id(&connection, id)
        .expect("read")
        .expect("present");
    assert_eq!(cleared.last_error, None);
    assert_eq!(cleared.last_error_code, None);
    assert_eq!(cleared.last_error_at, None);
}

#[test]
fn repo_round_trips_through_storage() {
    let connection = connection();

    let id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("upsert");
    let stored = repos::by_id(&connection, id)
        .expect("by id")
        .expect("present");

    assert_eq!(stored.github_id, 7);
    assert_eq!(stored.full_name, "acme/api");
    assert_eq!(stored.owner, "acme");
    assert_eq!(stored.name, "api");
    assert!(stored.is_private);
    assert!(stored.monitored, "a new repository is monitored by default");
    assert_eq!(stored.default_branch.as_deref(), Some("main"));
}

#[test]
fn upsert_never_resets_user_preferences() {
    let connection = connection();

    let id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("upsert");
    assert!(repos::set_monitored(&connection, id, false).expect("set monitored"));

    // A later sync of the same repo must not quietly re-enable it.
    let id_again = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("re-upsert");
    let stored = repos::by_id(&connection, id_again)
        .expect("by id")
        .expect("present");

    assert_eq!(id, id_again, "the same GitHub id maps to the same row");
    assert!(
        !stored.monitored,
        "the poll must not overwrite the preference"
    );
}

#[test]
fn rename_updates_the_row_in_place() {
    let connection = connection();

    let id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("upsert");
    let same = repos::upsert(&connection, &repo_upsert(7, "acme/api-service")).expect("rename");

    assert_eq!(id, same);
    let stored = repos::by_id(&connection, same)
        .expect("by id")
        .expect("present");
    assert_eq!(stored.full_name, "acme/api-service");
    assert_eq!(repos::count(&connection).expect("count"), 1);
}

#[test]
fn deleting_a_repo_cascades_to_its_children() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    let run = runs::upsert(
        &connection,
        &run_upsert(repo_id, 100, "completed", Some("success")),
    )
    .expect("run");

    connection
        .execute(
            "INSERT INTO jobs (run_id, github_id, name, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![run.id, 500, "build", "completed", Utc::now()],
        )
        .expect("job");

    connection
        .execute(
            "INSERT INTO deployments (repo_id, github_id, environment, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![repo_id, 900, "production", Utc::now()],
        )
        .expect("deployment");

    assert!(repos::delete(&connection, repo_id).expect("delete"));

    let remaining: i64 = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM workflow_runs)
                  + (SELECT COUNT(*) FROM jobs)
                  + (SELECT COUNT(*) FROM deployments)",
            [],
            |row| row.get(0),
        )
        .expect("counts");

    assert_eq!(remaining, 0, "cascade must clear every child table");
}

#[test]
fn etag_survives_a_round_trip() {
    let connection = connection();
    let id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    assert_eq!(repos::runs_etag(&connection, id).expect("read"), None);

    repos::set_runs_etag(&connection, id, Some("W/\"abc123\"")).expect("write");
    assert_eq!(
        repos::runs_etag(&connection, id).expect("read").as_deref(),
        Some("W/\"abc123\"")
    );

    repos::set_runs_etag(&connection, id, None).expect("clear");
    assert_eq!(repos::runs_etag(&connection, id).expect("read"), None);
}

#[test]
fn diff_writes_only_runs_that_changed() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    let incoming = vec![
        run_upsert(repo_id, 100, "in_progress", None),
        run_upsert(repo_id, 101, "completed", Some("success")),
    ];

    let first = diff::diff_runs(&connection, repo_id, &incoming).expect("first poll");
    assert_eq!(
        first.changed.len(),
        2,
        "everything is new on the first poll"
    );
    assert_eq!(first.unchanged, 0);

    let second = diff::diff_runs(&connection, repo_id, &incoming).expect("second poll");
    assert!(second.is_empty(), "an identical poll must write nothing");
    assert_eq!(second.unchanged, 2);

    let mut moved = incoming.clone();
    moved[0].status = "completed".to_owned();
    moved[0].conclusion = Some("failure".to_owned());

    let third = diff::diff_runs(&connection, repo_id, &moved).expect("third poll");
    assert_eq!(third.changed.len(), 1);
    assert_eq!(third.changed[0].github_id, 100);
    assert_eq!(third.changed[0].conclusion.as_deref(), Some("failure"));
    assert_eq!(third.unchanged, 1);
    assert_eq!(
        runs::count_by_repo(&connection, repo_id).expect("count"),
        2,
        "diffing must not duplicate rows"
    );
}

#[test]
fn diff_ignores_a_bare_updated_at_bump() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    let mut run = run_upsert(repo_id, 100, "in_progress", None);
    diff::diff_runs(&connection, repo_id, std::slice::from_ref(&run)).expect("first");

    // GitHub touches updated_at on transitions we do not render. That alone is
    // not a reason to emit a UI event.
    run.updated_at = Utc::now() + Duration::minutes(5);

    let second = diff::diff_runs(&connection, repo_id, std::slice::from_ref(&run)).expect("second");
    assert!(
        second.is_empty(),
        "updated_at alone must not count as a change"
    );
}

#[test]
fn diff_treats_a_new_attempt_as_a_change() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    let mut run = run_upsert(repo_id, 100, "completed", Some("failure"));
    diff::diff_runs(&connection, repo_id, std::slice::from_ref(&run)).expect("first");

    // Re-running a failed workflow keeps status/conclusion but bumps the attempt.
    run.run_attempt = Some(2);

    let second = diff::diff_runs(&connection, repo_id, std::slice::from_ref(&run)).expect("second");
    assert_eq!(second.changed.len(), 1);
    assert_eq!(second.changed[0].run_attempt, Some(2));
}

#[test]
fn has_active_reports_only_in_flight_runs() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    runs::upsert(
        &connection,
        &run_upsert(repo_id, 100, "completed", Some("success")),
    )
    .expect("run");
    assert!(!runs::has_active(&connection, repo_id).expect("inactive"));

    runs::upsert(&connection, &run_upsert(repo_id, 101, "in_progress", None)).expect("run");
    assert!(runs::has_active(&connection, repo_id).expect("active"));
}

#[test]
fn retention_prunes_old_rows_and_keeps_recent_ones() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    let old = Utc::now() - Duration::days(200);
    let recent = Utc::now() - Duration::days(1);

    let mut stale = run_upsert(repo_id, 100, "completed", Some("success"));
    stale.updated_at = old;
    runs::upsert(&connection, &stale).expect("stale run");

    let mut fresh = run_upsert(repo_id, 101, "completed", Some("success"));
    fresh.updated_at = recent;
    runs::upsert(&connection, &fresh).expect("fresh run");

    let report = retention::prune_with_defaults(&connection).expect("prune");

    assert_eq!(report.runs_deleted, 1);
    assert_eq!(runs::count_by_repo(&connection, repo_id).expect("runs"), 1);
}

#[test]
fn vacuum_runs_at_most_once_per_interval() {
    let connection = connection();

    let first = retention::prune_with_defaults(&connection).expect("first prune");
    assert!(first.vacuumed, "the first sweep should compact");

    let second = retention::prune_with_defaults(&connection).expect("second prune");
    assert!(!second.vacuumed, "a second sweep must not vacuum again");
}

#[test]
fn pull_request_sweep_removes_rows_the_sync_did_not_report() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");

    let pull = |github_id: i64, number: i64| PullRequestUpsert {
        repo_id,
        github_id,
        number,
        title: format!("PR {number}"),
        state: "open".to_owned(),
        is_draft: false,
        author: Some("octocat".to_owned()),
        review_state: None,
        checks_state: None,
        html_url: None,
        updated_at: Utc::now(),
    };

    pulls::upsert(&connection, &pull(1, 1)).expect("pr 1");
    pulls::upsert(&connection, &pull(2, 2)).expect("pr 2");

    let cutoff = Utc::now();
    std::thread::sleep(StdDuration::from_millis(5));

    // Only PR 1 came back in this sync.
    pulls::upsert(&connection, &pull(1, 1)).expect("refresh pr 1");

    let swept = pulls::sweep_unseen(&connection, repo_id, cutoff).expect("sweep");
    assert_eq!(
        swept, 1,
        "the unseen pull request is the one merged upstream"
    );

    let remaining = pulls::list_by_repo(&connection, repo_id).expect("list");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].number, 1);
}

#[test]
fn job_and_deployment_upserts_are_idempotent() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(7, "acme/api")).expect("repo");
    let run =
        runs::upsert(&connection, &run_upsert(repo_id, 100, "in_progress", None)).expect("run");

    let job = JobUpsert {
        run_id: run.id,
        github_id: 500,
        name: "build".to_owned(),
        status: "in_progress".to_owned(),
        conclusion: None,
        started_at: Some(Utc::now()),
        completed_at: None,
        html_url: None,
        updated_at: Utc::now(),
    };

    super::jobs::upsert(&connection, &job).expect("job");
    let mut finished = job.clone();
    finished.status = "completed".to_owned();
    finished.conclusion = Some("success".to_owned());
    super::jobs::upsert(&connection, &finished).expect("job again");

    let stored = super::jobs::list_by_run(&connection, run.id).expect("list");
    assert_eq!(stored.len(), 1, "the same job must not be duplicated");
    assert_eq!(stored[0].conclusion.as_deref(), Some("success"));

    let deployment = DeploymentUpsert {
        repo_id,
        github_id: 900,
        environment: "production".to_owned(),
        git_ref: Some("main".to_owned()),
        sha: Some("a1b2c3d4".to_owned()),
        state: Some("success".to_owned()),
        html_url: None,
        updated_at: Utc::now(),
    };

    super::deployments::upsert(&connection, &deployment).expect("deployment");
    super::deployments::upsert(&connection, &deployment).expect("deployment again");

    assert_eq!(
        super::deployments::list_by_repo(&connection, repo_id, 10)
            .expect("list")
            .len(),
        1
    );
}

#[test]
fn dashboard_queries_scale_to_250_repositories() {
    let mut connection = connection();

    let transaction = connection.transaction().expect("transaction");
    for index in 0..250 {
        let repo_id = repos::upsert(
            &transaction,
            &repo_upsert(1000 + index, &format!("acme/service-{index}")),
        )
        .expect("repo");

        for run_index in 0..20 {
            runs::upsert(
                &transaction,
                &run_upsert(
                    repo_id,
                    100_000 + index * 100 + run_index,
                    "completed",
                    Some("success"),
                ),
            )
            .expect("run");
        }
    }
    transaction.commit().expect("commit");

    // The dashboard's steady state: list every repo, then show its newest run.
    let started = Instant::now();
    let all = repos::list(&connection).expect("list repos");
    let mut with_latest = 0_usize;
    for repo in &all {
        if runs::latest_for_repo(&connection, repo.id)
            .expect("latest run")
            .is_some()
        {
            with_latest += 1;
        }
    }
    let elapsed = started.elapsed();

    assert_eq!(all.len(), 250);
    assert_eq!(with_latest, 250);
    println!("250 repos / 5000 runs: list + 250 latest-run lookups in {elapsed:?}");

    // Generous bound so a loaded CI runner does not flake, while still catching
    // an accidental full table scan or a missing index.
    assert!(
        elapsed < StdDuration::from_millis(250),
        "dashboard queries took {elapsed:?}"
    );
}

#[tokio::test]
async fn db_wrapper_opens_a_file_and_uses_wal() {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = Db::new(directory.path().join("nested")).expect("handle");

    // Before an account is known the handle reads from memory, so a query returns
    // nothing rather than failing and making every caller handle a state the
    // window behind it does not show.
    assert!(db.path().is_none());
    assert_eq!(db.with_connection(repos::count).await.expect("read"), 0);

    db.open_account("octocat").expect("open");

    let journal_mode: String = db
        .with_connection(|connection| {
            connection
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .map_err(AppError::db)
        })
        .await
        .expect("journal mode");

    assert_eq!(journal_mode.to_lowercase(), "wal");

    // The file is named after the account, and the generated directory was
    // created rather than assumed.
    let path = db.path().expect("open");
    assert!(path.ends_with("gitpulse-octocat.db"), "{}", path.display());
    assert!(path.exists());

    let full_name = "acme/api".to_owned();
    db.with_connection(move |connection| {
        repos::upsert(connection, &repo_upsert(7, &full_name))?;
        Ok(())
    })
    .await
    .expect("write");

    let count = db.with_connection(repos::count).await.expect("read");

    assert_eq!(count, 1);
}

#[test]
fn one_account_is_one_file() {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = Db::new(directory.path()).expect("handle");

    for (login, expected) in [
        ("octocat", "gitpulse-octocat.db"),
        // Lowercased, or a case-insensitive filesystem would hold two accounts.
        ("N4OD", "gitpulse-n4od.db"),
    ] {
        db.open_account(login).expect("open");

        let path = db.path().expect("open");
        assert_eq!(path.file_name().expect("name").to_string_lossy(), expected);
    }
}

#[test]
fn switching_accounts_switches_files() {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = Db::new(directory.path()).expect("handle");

    db.open_account("octocat").expect("open");
    let first = db.path().expect("open");

    db.with_connection_blocking(|connection| {
        repos::upsert(connection, &repo_upsert(1, "acme/api"))?;
        Ok(())
    })
    .expect("write");

    db.open_account("n4od").expect("open");

    assert_ne!(db.path().expect("open"), first);
    assert_eq!(
        db.with_connection_blocking(repos::count).expect("read"),
        0,
        "the second account must not see the first account's rows"
    );

    // And going back finds them exactly where they were left.
    db.open_account("octocat").expect("open");
    assert_eq!(db.with_connection_blocking(repos::count).expect("read"), 1);
}

#[test]
fn signing_out_deletes_only_that_accounts_file() {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = Db::new(directory.path()).expect("handle");

    db.open_account("octocat").expect("open");
    let kept = db.path().expect("open");

    db.open_account("n4od").expect("open");
    let removed = db.path().expect("open");

    db.forget_current().expect("forget");

    assert!(!removed.exists(), "the signed-out account's file is gone");
    assert!(kept.exists(), "the other account's file is untouched");
    assert!(
        db.path().is_none(),
        "and the handle reads from memory again"
    );

    // Closing a file is not the same as deleting it.
    db.open_account("octocat").expect("open");
    assert_eq!(db.path().expect("open"), kept);
    db.close().expect("close");
    assert!(kept.exists());
}

#[test]
fn a_database_claiming_another_account_is_discarded() {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = Db::new(directory.path()).expect("handle");

    db.open_account("octocat").expect("open");
    db.with_connection_blocking(|connection| {
        repos::upsert(connection, &repo_upsert(1, "acme/api"))?;
        // The file is octocat's by name; make its contents claim someone else, as
        // a copied or renamed file would.
        cache::claim_for_account(connection, "someone-else")?;
        repos::upsert(connection, &repo_upsert(2, "acme/web"))?;
        Ok(())
    })
    .expect("write");

    // Opening it again is the cross-check: the contents claim another account, so
    // they are discarded rather than served under the wrong name.
    db.close().expect("close");
    db.open_account("octocat").expect("open");

    assert_eq!(db.with_connection_blocking(repos::count).expect("read"), 0);
    assert_eq!(
        db.with_connection_blocking(cache::account_owner)
            .expect("owner")
            .as_deref(),
        Some("octocat")
    );
}

#[test]
fn the_pre_accounts_database_is_filed_under_its_owner() {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = Db::new(directory.path()).expect("handle");

    // The shape of a database written before there was one file per account: a
    // single `gitpulse.db` with its owner recorded inside it.
    db.open_account("octocat").expect("open");
    let per_account = db.path().expect("open");
    db.with_connection_blocking(|connection| {
        repos::upsert(connection, &repo_upsert(1, "acme/api"))?;
        Ok(())
    })
    .expect("write");
    db.close().expect("close");

    let legacy = directory.path().join("gitpulse.db");
    std::fs::rename(&per_account, &legacy).expect("rename to the pre-accounts name");

    db.open_account("octocat").expect("open");

    assert!(!legacy.exists(), "the old file is moved, not duplicated");
    assert!(per_account.exists());
    assert_eq!(
        db.with_connection_blocking(repos::count).expect("read"),
        1,
        "filing it under its owner keeps the rows"
    );
}

#[test]
fn a_pre_accounts_database_with_no_owner_is_discarded() {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = Db::new(directory.path()).expect("handle");

    db.open_account("octocat").expect("open");
    let per_account = db.path().expect("open");
    db.with_connection_blocking(|connection| {
        repos::upsert(connection, &repo_upsert(1, "acme/api"))?;
        // Rows with no recorded owner: the shape a cache written before 0.5.5
        // has, which cannot be attributed to whoever is signing in now.
        meta::remove(connection, cache::ACCOUNT_KEY)?;
        Ok(())
    })
    .expect("write");
    db.close().expect("close");

    let legacy = directory.path().join("gitpulse.db");
    std::fs::rename(&per_account, &legacy).expect("rename");

    db.open_account("octocat").expect("open");

    assert!(!legacy.exists(), "an unattributable cache is not adopted");
    assert_eq!(
        db.with_connection_blocking(repos::count).expect("read"),
        0,
        "its rows are dropped rather than shown to whoever signed in"
    );
    assert_eq!(
        db.with_connection_blocking(cache::account_owner)
            .expect("owner")
            .as_deref(),
        Some("octocat")
    );
}

#[test]
fn signing_out_purges_the_account_cache() {
    let connection = connection();
    let repo_id = repos::upsert(&connection, &repo_upsert(1, "acme/api")).expect("insert repo");
    runs::upsert(
        &connection,
        &run_upsert(repo_id, 500, "completed", Some("success")),
    )
    .expect("insert run");

    let removed = cache::purge_account_data(&connection).expect("purge");

    assert!(
        removed >= 2,
        "expected the repo and its run to be removed, removed {removed}"
    );
    assert_eq!(repos::count(&connection).expect("count repos"), 0);
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM workflow_runs", [], |row| row
                .get::<_, i64>(0))
            .expect("count runs"),
        0
    );
    // No owner is recorded once the cache is empty, so the next sign-in claims
    // it from scratch rather than comparing against a stale login.
    assert_eq!(
        super::meta::get(&connection, cache::ACCOUNT_KEY).expect("read owner"),
        None
    );
}

#[test]
fn a_different_account_discards_the_previous_cache() {
    let connection = connection();

    // Claiming an empty cache records the owner and deletes nothing.
    assert!(!cache::claim_for_account(&connection, "hubot").expect("claim"));

    repos::upsert(&connection, &repo_upsert(1, "acme/api")).expect("insert repo");

    // The same account keeps its cache.
    assert!(!cache::claim_for_account(&connection, "hubot").expect("claim"));
    assert_eq!(repos::count(&connection).expect("count"), 1);

    // A different account inherits nothing from the first one.
    assert!(cache::claim_for_account(&connection, "someone-else").expect("claim"));
    assert_eq!(repos::count(&connection).expect("count"), 0);
    assert_eq!(
        super::meta::get(&connection, cache::ACCOUNT_KEY)
            .expect("read owner")
            .as_deref(),
        Some("someone-else")
    );
}

#[test]
fn an_unowned_cache_is_not_trusted() {
    // Rows written before the owner key existed cannot be attributed to whoever
    // signs in now, so they are discarded rather than displayed. This is the
    // upgrade path: the first launch after the fix must not keep showing the
    // previous account's repositories.
    let connection = connection();
    repos::upsert(&connection, &repo_upsert(1, "someones-else/api")).expect("insert repo");

    assert!(cache::claim_for_account(&connection, "hubot").expect("claim"));
    assert_eq!(repos::count(&connection).expect("count"), 0);
    assert_eq!(
        super::meta::get(&connection, cache::ACCOUNT_KEY)
            .expect("read owner")
            .as_deref(),
        Some("hubot")
    );

    // And it is a one-time cost: the next launch sees a matching owner.
    assert!(!cache::claim_for_account(&connection, "hubot").expect("claim"));
}
