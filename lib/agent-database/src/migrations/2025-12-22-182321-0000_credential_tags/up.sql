CREATE TABLE secret_tags (
    secret_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (secret_id, tag_id)
);


