CREATE TABLE cache_tags (
    cache_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    FOREIGN KEY (cache_id) REFERENCES cache(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (cache_id, tag_id)
);

