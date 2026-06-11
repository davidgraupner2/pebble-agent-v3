CREATE TABLE properties (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    registration_id TEXT NOT NULL,
    name VARCHAR NOT NULL UNIQUE,
    type VARCHAR NOT NULL CHECK(type IN ('int', 'string', 'bool', 'json')),
    description VARCHAR,
    value_int INTEGER,
    value_string TEXT,
    value_bool INTEGER,  -- SQLite stores bool as 0/1
    value_json TEXT,
    source VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_properties_agent_name ON properties(registration_id,name);
CREATE UNIQUE INDEX idx_properties_name ON properties(name);

-- Ensure exactly one value column is set based on property
CREATE TRIGGER properties_validate_value
BEFORE INSERT ON properties
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.type = 'int' AND NEW.value_int IS NULL THEN
            RAISE(ABORT, 'value_int required for type int')
        WHEN NEW.type = 'string' AND NEW.value_string IS NULL THEN
            RAISE(ABORT, 'value_string required for type string')
        WHEN NEW.type = 'bool' AND NEW.value_bool IS NULL THEN
            RAISE(ABORT, 'value_bool required for type bool')
        WHEN NEW.type = 'json' AND NEW.value_json IS NULL THEN
            RAISE(ABORT, 'value_json required for type json')
    END;
END;

CREATE TRIGGER properties_updated_at 
AFTER UPDATE on properties
FOR EACH ROW
BEGIN
    UPDATE properties SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

CREATE TRIGGER properties_protect_config_delete
BEFORE DELETE ON properties
FOR EACH ROW
WHEN LOWER(TRIM(OLD.source)) = 'config'
BEGIN
    SELECT RAISE(ABORT, 'Cannot delete config properties');
END;