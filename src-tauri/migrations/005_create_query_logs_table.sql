CREATE TABLE IF NOT EXISTS query_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id TEXT NOT NULL,
    connection_name TEXT NOT NULL,
    database_name TEXT,
    timestamp INTEGER NOT NULL,
    query_text TEXT NOT NULL,
    duration_ms INTEGER,
    row_count INTEGER,
    status TEXT NOT NULL, -- 'success' or 'error'
    error_message TEXT
);

CREATE INDEX idx_query_logs_timestamp ON query_logs(timestamp DESC);
