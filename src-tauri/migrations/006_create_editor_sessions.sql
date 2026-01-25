-- Migration: 006_create_editor_sessions
-- Purpose: Store Monaco editor state for persistence across reloads

CREATE TABLE IF NOT EXISTS editor_sessions (
    id TEXT PRIMARY KEY,                 -- Unique editor session ID (typically the view ID)
    window_label TEXT NOT NULL,          -- The Tauri window label for isolation
    connection_id TEXT,                  -- Associated database connection (optional)
    schema_name TEXT,                    -- Associated schema (optional)
    content TEXT NOT NULL DEFAULT '',    -- Editor text content
    cursor_line INTEGER NOT NULL DEFAULT 1,
    cursor_column INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    last_opened_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_editor_sessions_window ON editor_sessions(window_label);
CREATE INDEX IF NOT EXISTS idx_editor_sessions_connection ON editor_sessions(connection_id);
