-- Per-repository poll failure.
--
-- The poller keeps one global error, which is right for "the sweep is paused"
-- and wrong for "this one repository cannot be read". Recording it per row lets
-- a single broken repository show its own state instead of the failure being
-- invisible until someone notices a stale card.

ALTER TABLE repos ADD COLUMN last_error TEXT;
