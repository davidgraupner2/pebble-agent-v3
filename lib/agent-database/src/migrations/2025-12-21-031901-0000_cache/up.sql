CREATE TABLE cache (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL COLLATE NOCASE,
    description VARCHAR NULL,
    type VARCHAR NOT NULL,
    value TEXT NOT NULL,
    source VARCHAR NOT NULL,
    created_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at NULL
);

CREATE TRIGGER cache_updated_at 
AFTER UPDATE on cache
FOR EACH ROW
BEGIN
    UPDATE cache SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Unique Index to ensure no duplicates
CREATE UNIQUE INDEX idx_cache ON cache(name  COLLATE NOCASE);
