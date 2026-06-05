CREATE TABLE connection_stats (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    endpoint VARCHAR(8000)  NOT NULL,
    status VARCHAR(15) NOT NULL DEFAULT 'connected',
    connected_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disconnected_at timestamp_with_timezone_text NULL
);

CREATE TRIGGER connection_stats_connected
AFTER INSERT on connection_stats
FOR EACH ROW
BEGIN
    UPDATE connection_stats SET connected_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER connection_stats_disconnected 
AFTER UPDATE on connection_stats
FOR EACH ROW
BEGIN
    UPDATE connection_stats SET disconnected_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

