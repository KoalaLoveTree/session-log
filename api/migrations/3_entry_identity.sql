ALTER TABLE entries
    ADD COLUMN updated_at TIMESTAMPTZ;

UPDATE entries
SET updated_at = COALESCE(deleted_at, created_at);

ALTER TABLE entries
    ALTER COLUMN updated_at SET NOT NULL,
    ALTER COLUMN updated_at SET DEFAULT now();

ALTER TABLE entries
    ALTER COLUMN id DROP DEFAULT;

ALTER TABLE entries
    ALTER COLUMN id TYPE TEXT USING id::text;

DROP SEQUENCE IF EXISTS entries_id_seq;
