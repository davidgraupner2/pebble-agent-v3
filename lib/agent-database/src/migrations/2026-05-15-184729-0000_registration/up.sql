CREATE TABLE registration (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    agent_id VARCHAR NOT NULL,
    jti VARCHAR NOT NULL,
    source VARCHAR NOT NULL,
    expires_at timestamp_with_timezone_text NULL,
    created_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp_with_timezone_text NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER registration_updated_at 
AFTER UPDATE on registration
FOR EACH ROW
BEGIN
    UPDATE registration SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Create an event in the events table after new registrations
CREATE TRIGGER registration_event_created
AFTER INSERT ON registration
FOR EACH ROW
BEGIN
    INSERT INTO events (event_type, aggregate_type, aggregate_id, payload)
    VALUES (
        'registration.created',
        'registration',
        CAST(NEW.id AS TEXT),
        json_object('id', NEW.id, 'jti', NEW.jti, 'agent_id', NEW.agent_id, 'source', NEW.source, 'expires_at', NEW.expires_at)
    );
END;

-- Unique Index to ensure no duplicates
CREATE UNIQUE INDEX idx_registration ON registration(jti, agent_id);

-- Delete any other registration records for the same id, when a new registration record is inserted
CREATE TRIGGER registration_ensure_only_one
AFTER INSERT ON registration
FOR EACH ROW
BEGIN
    DELETE FROM registration WHERE id NOT IN (NEW.id) and agent_id = NEW.agent_id;
END;

-- Update existing pending events in the events table when new registrations are added
-- This is to ensure current events for new registrations strings are cancelled 
CREATE TRIGGER new_registration_cancel_previous_events
BEFORE INSERT ON registration
FOR EACH ROW
BEGIN
    UPDATE events set status = "cancelled" where event_type="registration.created" and aggregate_type = "registration" and status = "pending";
END;

