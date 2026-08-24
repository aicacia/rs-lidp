CREATE TABLE IF NOT EXISTS storage_issuers (
    issuer TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    revoked_at INTEGER
) STRICT;

CREATE TABLE IF NOT EXISTS storage_issuer_keys (
    issuer TEXT NOT NULL,
    key_id INTEGER NOT NULL,
    public_key TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    revoked_at INTEGER,
    PRIMARY KEY (issuer, key_id),
    FOREIGN KEY (issuer) REFERENCES storage_issuers (issuer)
) STRICT;

CREATE INDEX IF NOT EXISTS storage_issuer_keys_active_idx
    ON storage_issuer_keys (issuer, key_id)
    WHERE revoked_at IS NULL;
