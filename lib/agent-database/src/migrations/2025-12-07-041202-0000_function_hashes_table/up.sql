CREATE TABLE function_hashes (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    function_hash VARCHAR NOT NULL, 
    description VARCHAR,
    source VARCHAR NULL,
    created_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER function_hashes_updated_at 
AFTER UPDATE on function_hashes
FOR EACH ROW
BEGIN
    UPDATE function_hashes SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Unique Index to ensure no duplicates
CREATE UNIQUE INDEX idx_function_hashes ON function_hashes(function_hash);