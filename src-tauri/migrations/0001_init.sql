-- Core normalized cache.
--
-- Nothing in this schema is authoritative: GitHub is. Every row is therefore
-- written with an idempotent upsert keyed on the GitHub numeric id, and rows
-- are never merged by hand.
--
-- Identity note: `github_id` is the natural key, not `full_name`. Repositories
-- get renamed, and a rename can transiently collide with a row we still hold,
-- which would make a UNIQUE(full_name) constraint fail the whole upsert. So
-- full_name is indexed but not unique.

CREATE TABLE repos (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    github_id      INTEGER NOT NULL UNIQUE,
    owner          TEXT    NOT NULL,
    name           TEXT    NOT NULL,
    full_name      TEXT    NOT NULL,
    is_private     INTEGER NOT NULL DEFAULT 0,
    default_branch TEXT,
    avatar_url     TEXT,
    -- User preference. Never touched by an upsert; see db/repos.rs.
    monitored      INTEGER NOT NULL DEFAULT 1,
    last_polled_at TEXT,
    runs_etag      TEXT,
    created_at     TEXT    NOT NULL,
    updated_at     TEXT    NOT NULL
);

CREATE INDEX idx_repos_monitored ON repos (monitored, full_name);
CREATE INDEX idx_repos_full_name ON repos (full_name);

CREATE TABLE workflow_runs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id       INTEGER NOT NULL REFERENCES repos (id) ON DELETE CASCADE,
    github_id     INTEGER NOT NULL UNIQUE,
    workflow_name TEXT,
    display_title TEXT,
    event         TEXT,
    branch        TEXT,
    head_sha      TEXT,
    status        TEXT    NOT NULL,
    conclusion    TEXT,
    run_number    INTEGER,
    run_attempt   INTEGER,
    html_url      TEXT,
    actor_login   TEXT,
    created_at    TEXT,
    run_started_at TEXT,
    updated_at    TEXT    NOT NULL
);

-- The dashboard's hot path: newest runs for one repo.
CREATE INDEX idx_runs_repo_updated ON workflow_runs (repo_id, updated_at DESC);
-- Lets the poller find repos with something in flight.
CREATE INDEX idx_runs_status ON workflow_runs (status);
-- Retention sweep.
CREATE INDEX idx_runs_updated ON workflow_runs (updated_at);

CREATE TABLE jobs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id       INTEGER NOT NULL REFERENCES workflow_runs (id) ON DELETE CASCADE,
    github_id    INTEGER NOT NULL UNIQUE,
    name         TEXT    NOT NULL,
    status       TEXT    NOT NULL,
    conclusion   TEXT,
    started_at   TEXT,
    completed_at TEXT,
    html_url     TEXT,
    updated_at   TEXT    NOT NULL
);

CREATE INDEX idx_jobs_run ON jobs (run_id);

CREATE TABLE deployments (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id     INTEGER NOT NULL REFERENCES repos (id) ON DELETE CASCADE,
    github_id   INTEGER NOT NULL UNIQUE,
    environment TEXT    NOT NULL,
    git_ref     TEXT,
    sha         TEXT,
    state       TEXT,
    html_url    TEXT,
    updated_at  TEXT    NOT NULL
);

CREATE INDEX idx_deployments_repo ON deployments (repo_id, updated_at DESC);

CREATE TABLE pull_requests (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id      INTEGER NOT NULL REFERENCES repos (id) ON DELETE CASCADE,
    github_id    INTEGER NOT NULL UNIQUE,
    number       INTEGER NOT NULL,
    title        TEXT    NOT NULL,
    state        TEXT    NOT NULL,
    is_draft     INTEGER NOT NULL DEFAULT 0,
    author       TEXT,
    review_state TEXT,
    checks_state TEXT,
    html_url     TEXT,
    updated_at   TEXT    NOT NULL,
    -- Set on every sync. Open pull requests that were not seen in the latest
    -- sync are swept, so a PR closed on GitHub stops showing as open here.
    last_seen_at TEXT    NOT NULL
);

CREATE INDEX idx_pulls_repo_state ON pull_requests (repo_id, state);

-- Small key/value store for poller bookkeeping, the schema version and the
-- settings blob. Secrets never go here; they live in the OS keychain.
CREATE TABLE meta (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
