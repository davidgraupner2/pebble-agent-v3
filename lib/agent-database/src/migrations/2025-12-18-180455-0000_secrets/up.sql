CREATE TABLE secrets (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL COLLATE NOCASE,
    secret_type VARCHAR NOT NULL,
    description VARCHAR NULL,
    value VARCHAR NOT NULL,
    source VARCHAR NOT NULL,
    ephemeral_key VARCHAR NULL,
    nonce VARCHAR NULL, 
    encryption_key_id INTEGER NOT NULL REFERENCES encryption_keys(id) ON DELETE CASCADE,
    created_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER secrets_updated_at 
AFTER UPDATE on secrets
FOR EACH ROW
BEGIN
    UPDATE secrets SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Unique Index to ensure no duplicates
CREATE UNIQUE INDEX idx_secrets ON secrets(name COLLATE NOCASE);
