-- Drop the webhook deliveries table.
--
-- The webhook subsystem was removed: the relay transport could never verify a
-- signature (it re-encodes the body, so the bytes GitHub signed are gone), and
-- the local transport needed a public URL plus a port that stayed the same
-- across restarts. What it bought was latency, and it was the app's only
-- listening socket - so the cache it kept goes with it.
--
-- `0002` stays in the list rather than being deleted: it has already run on
-- machines that shipped, and migrations are append-only for exactly that reason.

DROP TABLE IF EXISTS webhook_deliveries;