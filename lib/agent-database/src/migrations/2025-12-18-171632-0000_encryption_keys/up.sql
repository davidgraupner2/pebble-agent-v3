CREATE TABLE encryption_keys (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL COLLATE NOCASE,
    enabled int default 1,
    public_key VARCHAR NOT NULL,
    source VARCHAR NOT NULL,
    created_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER encryption_keys_updated_at 
AFTER UPDATE on encryption_keys
FOR EACH ROW
BEGIN
    UPDATE encryption_keys SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Unique Index to ensure no duplicates
CREATE UNIQUE INDEX idx_encryption_keys ON encryption_keys(name COLLATE NOCASE);