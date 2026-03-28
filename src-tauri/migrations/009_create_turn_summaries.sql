CREATE TABLE IF NOT EXISTS agent_turn_summaries (
    id          TEXT    PRIMARY KEY,
    thread_id   TEXT    NOT NULL REFERENCES agent_threads(id) ON DELETE CASCADE,
    total_ms    INTEGER NOT NULL,
    model       TEXT    NOT NULL DEFAULT '',
    cancelled   INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);
