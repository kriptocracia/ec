-- Track whether election results have been successfully counted and published.
-- Allows the scheduler to retry on transient failures (e.g. Nostr relay down).

ALTER TABLE elections ADD COLUMN results_published INTEGER NOT NULL DEFAULT 0;
