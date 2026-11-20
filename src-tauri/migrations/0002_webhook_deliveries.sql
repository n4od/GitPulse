-- Webhook delivery log.
--
-- GitHub retries deliveries, and a relay can replay them after a reconnect, so
-- the same payload can arrive more than once. The primary key is GitHub's
-- `X-GitHub-Delivery` UUID: recording it is what makes ingestion idempotent.
-- The row is also the record we prune on a retention schedule.

CREATE TABLE webhook_deliveries (
    delivery_id TEXT PRIMARY KEY,
    event       TEXT NOT NULL,
    repo_full_name TEXT,
    received_at TEXT NOT NULL
);

CREATE INDEX idx_deliveries_received ON webhook_deliveries (received_at);
