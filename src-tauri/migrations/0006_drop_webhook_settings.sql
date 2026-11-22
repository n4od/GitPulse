-- Delete the settings the webhook feature left behind.
--
-- `0005` dropped the table, but the configuration that pointed at it stayed: four
-- `meta` rows naming a transport, a path secret, a relay URL and a port. Nothing
-- reads them any more. A leaf's leftovers are still leftovers, and a path secret
-- is a secret even when the listener it guarded is gone.
--
-- Databases belonging to accounts that are not opened again keep both the empty
-- table and these rows: migrations run per file, when that file is opened. That
-- is harmless - no code path reaches either - and it resolves itself the next
-- time the account is used.

DELETE FROM meta WHERE key LIKE 'webhook%';