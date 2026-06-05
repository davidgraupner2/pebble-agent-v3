CREATE TABLE connection_strings (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    value VARCHAR NOT NULL,
    description VARCHAR,
    source VARCHAR NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    environment VARCHAR NULL,
    created_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER connection_strings_updated_at 
AFTER UPDATE on connection_strings
FOR EACH ROW
BEGIN
    UPDATE connection_strings SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Automatically delete any existing 'pending' records before inserting a new one
CREATE TRIGGER connection_strings_pending
BEFORE INSERT on connection_strings
FOR EACH ROW
WHEN NEW.status = 'pending'
BEGIN
    DELETE FROM connection_strings WHERE status = 'pending';
END;

-- Create an event in the events table after new connection strings are added
CREATE TRIGGER connection_strings_event_created
AFTER INSERT ON connection_strings
FOR EACH ROW
BEGIN
    INSERT INTO events (event_type, aggregate_type, aggregate_id, payload)
    VALUES (
        'connection_string.created',
        'connection_string',
        CAST(NEW.id AS TEXT),
        json_object('id', NEW.id, 'value', NEW.value, 'status', NEW.status, 'source', NEW.source)
    );
END;

-- Update existing pending events in the events table when new connection strings are added
-- This is to ensure current events for new connection strings are cancelled because new connection strings
-- result in existing pending connection strings being deleted - therefore the event is null and void
CREATE TRIGGER connection_strings_event_updated
BEFORE INSERT ON connection_strings
FOR EACH ROW
BEGIN
    UPDATE events set status = "cancelled" where event_type="connection_string.created" and aggregate_type = "connection_string" and status = "pending";
END;

-- Unique Index to ensure no duplicates
CREATE UNIQUE INDEX idx_connection_strings ON connection_strings(value);

-- Delete any other in-use records from the connection string table when an update occurs to make a pending connection string active
CREATE TRIGGER connections_strings_ensure_only_one_inuse
AFTER UPDATE ON connection_strings
FOR EACH ROW
WHEN NEW.status = 'in_use' 
BEGIN
    DELETE FROM connection_strings WHERE id NOT IN (NEW.id);
END