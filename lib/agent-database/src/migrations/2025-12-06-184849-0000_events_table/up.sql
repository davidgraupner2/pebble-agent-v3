CREATE TABLE events (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    event_type VARCHAR NOT NULL,           -- e.g., 'connection_string.created', 'connection_string.updated'
    aggregate_type VARCHAR NOT NULL,       -- e.g., 'connection_string', 'user', 'config'
    aggregate_id VARCHAR NOT NULL,         -- the ID of the entity (connection_strings.id as string)
    payload TEXT NOT NULL,                 -- JSON blob with event data
    metadata TEXT,                         -- JSON for correlation_id, user_id, source, etc.
    status TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'processing', 'processed', 'failed'
    retry_count INTEGER NOT NULL DEFAULT 0,
    processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER events_processed_at 
AFTER UPDATE on events
FOR EACH ROW
BEGIN
    UPDATE events SET processed_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Index for efficient polling by watchers
CREATE INDEX idx_events_status_created ON events(status, created_at);
CREATE INDEX idx_events_type_status ON events(event_type, status);
CREATE INDEX idx_events_aggregate ON events(aggregate_type, aggregate_id);