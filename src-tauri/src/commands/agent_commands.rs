use crate::DatabaseState;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

// ── Types returned to the frontend ────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct AgentThread {
    pub id: String,
    pub title: String,
    pub connection_id: String,
    pub database_name: Option<String>,
    pub model: String,
    pub effort: String,
    pub sdk_session_id: Option<String>,
    pub summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct AgentMessage {
    pub id: String,
    pub thread_id: String,
    pub role: String,
    pub content: String,
    pub thinking: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct AgentToolCall {
    pub id: String,
    pub thread_id: String,
    pub tool_name: String,
    pub input: String,
    pub output: Option<String>,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

// ── Thread commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_agent_thread(
    state: State<'_, DatabaseState>,
    id: String,
    connection_id: String,
    database_name: Option<String>,
    model: String,
    effort: String,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO agent_threads (id, title, connection_id, database_name, model, effort, created_at, updated_at)
         VALUES (?1, 'New chat', ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, connection_id, database_name, model, effort, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_agent_threads(
    state: State<'_, DatabaseState>,
    connection_id: String,
    database_name: Option<String>,
) -> Result<Vec<AgentThread>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, title, connection_id, database_name, model, effort, sdk_session_id, summary, created_at, updated_at
             FROM agent_threads
             WHERE connection_id = ?1
               AND (database_name IS ?2 OR ?2 IS NULL)
             ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![connection_id, database_name], |row| {
            Ok(AgentThread {
                id: row.get(0)?,
                title: row.get(1)?,
                connection_id: row.get(2)?,
                database_name: row.get(3)?,
                model: row.get(4)?,
                effort: row.get(5)?,
                sdk_session_id: row.get(6)?,
                summary: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_agent_thread_title(
    state: State<'_, DatabaseState>,
    id: String,
    title: String,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE agent_threads SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_agent_thread_sdk_session(
    state: State<'_, DatabaseState>,
    id: String,
    sdk_session_id: String,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE agent_threads SET sdk_session_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![sdk_session_id, now, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_agent_thread(
    state: State<'_, DatabaseState>,
    id: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    // CASCADE handles agent_messages and agent_tool_calls
    conn.execute("DELETE FROM agent_threads WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Message commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn append_agent_message(
    state: State<'_, DatabaseState>,
    id: String,
    thread_id: String,
    role: String,
    content: String,
    thinking: Option<String>,
    timestamp: i64,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO agent_messages (id, thread_id, role, content, thinking, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, thread_id, role, content, thinking, timestamp],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE agent_threads SET updated_at = ?1 WHERE id = ?2",
        params![now, thread_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_agent_messages(
    state: State<'_, DatabaseState>,
    thread_id: String,
) -> Result<Vec<AgentMessage>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, thread_id, role, content, thinking, timestamp
             FROM agent_messages WHERE thread_id = ?1 ORDER BY timestamp ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![thread_id], |row| {
            Ok(AgentMessage {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                thinking: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

// ── Tool call commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn upsert_agent_tool_call(
    state: State<'_, DatabaseState>,
    id: String,
    thread_id: String,
    tool_name: String,
    input: String,
    output: Option<String>,
    status: String,
    started_at: i64,
    completed_at: Option<i64>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO agent_tool_calls
         (id, thread_id, tool_name, input, output, status, started_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, thread_id, tool_name, input, output, status, started_at, completed_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_agent_tool_calls(
    state: State<'_, DatabaseState>,
    thread_id: String,
) -> Result<Vec<AgentToolCall>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, thread_id, tool_name, input, output, status, started_at, completed_at
             FROM agent_tool_calls WHERE thread_id = ?1 ORDER BY started_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![thread_id], |row| {
            Ok(AgentToolCall {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                tool_name: row.get(2)?,
                input: row.get(3)?,
                output: row.get(4)?,
                status: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
