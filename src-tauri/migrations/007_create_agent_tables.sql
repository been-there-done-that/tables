-- Agent sessions to group conversations
CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Agent messages within a session
CREATE TABLE IF NOT EXISTS agent_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL, -- 'user', 'assistant', 'system', 'tool'
    content TEXT,
    tool_calls TEXT, -- JSON array of tool calls if role is assistant
    tool_call_id TEXT, -- ID of the tool call this message responds to if role is tool
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

-- Index for faster message retrieval
CREATE INDEX IF NOT EXISTS idx_agent_messages_session_id ON agent_messages(session_id);
