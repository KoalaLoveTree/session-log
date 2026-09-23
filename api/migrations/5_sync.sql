ALTER TABLE entries
    ADD COLUMN synced_at TIMESTAMPTZ,
    ADD COLUMN other_body TEXT,
    ADD COLUMN declined_purged_at TIMESTAMPTZ;

CREATE TABLE purges (
    id TEXT PRIMARY KEY,
    purged_at TIMESTAMPTZ NOT NULL
);
