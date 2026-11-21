-- What kind of poll failure, and when.
--
-- `last_error` alone is a bare sentence: it says something went wrong but not
-- whether it was a permissions problem, a rate limit or a rename, and not
-- whether it happened a minute or a week ago. The code comes from
-- `AppError::code`, which is already the stable discriminator the frontend
-- branches on, and the timestamp is what makes the message actionable.

ALTER TABLE repos ADD COLUMN last_error_code TEXT;
ALTER TABLE repos ADD COLUMN last_error_at TEXT;
