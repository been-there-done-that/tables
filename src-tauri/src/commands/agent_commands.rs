use crate::DatabaseState;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

// ── Types returned to the frontend ────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentThread {
    pub id: String,
    pub title: String,
    pub connection_id: String,
    pub database_name: Option<String>,
    pub model: String,
    pub effort: String,
    pub sdk_session_id: Option<String>,
    pub summary: Option<String>,
    pub parent_thread_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub thread_id: String,
    pub role: String,
    pub content: String,
    pub thinking: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurnSummary {
    pub id: String,
    pub thread_id: String,
    pub total_ms: i64,
    pub model: String,
    pub cancelled: bool,
    pub created_at: i64,
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
    parent_thread_id: Option<String>,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO agent_threads (id, title, connection_id, database_name, model, effort, parent_thread_id, created_at, updated_at)
         VALUES (?1, 'New chat', ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![id, connection_id, database_name, model, effort, parent_thread_id, now],
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
            // When database_name is None, returns all threads for the connection
            // regardless of which database they belong to. This is a "show all" fallback.
            "SELECT id, title, connection_id, database_name, model, effort, sdk_session_id, summary, parent_thread_id, created_at, updated_at
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
                parent_thread_id: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
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
    // Note: these two writes are not in an explicit transaction. If the process crashes
    // between them, updated_at may be stale, but message data is still consistent.
    // This is acceptable for a display cache. rusqlite's Mutex<Connection> prevents
    // concurrent access, so the only risk is a crash between the two statements.
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

// ── Turn summary commands ──────────────────────────────────────────────────────

#[tauri::command]
pub fn save_turn_summary(
    state: State<'_, DatabaseState>,
    id: String,
    thread_id: String,
    total_ms: i64,
    model: String,
    cancelled: bool,
    created_at: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO agent_turn_summaries (id, thread_id, total_ms, model, cancelled, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, thread_id, total_ms, model, cancelled as i64, created_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_turn_summaries(
    state: State<'_, DatabaseState>,
    thread_id: String,
) -> Result<Vec<AgentTurnSummary>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, thread_id, total_ms, model, cancelled, created_at
             FROM agent_turn_summaries WHERE thread_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![thread_id], |row| {
            Ok(AgentTurnSummary {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                total_ms: row.get(2)?,
                model: row.get(3)?,
                cancelled: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

// ── Plan commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_agent_plan(
    state: State<'_, DatabaseState>,
    id: String,
    thread_id: String,
    title: String,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO agent_plans (id, thread_id, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'pending', ?4, ?4)",
        params![id, thread_id, title, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlanRow {
    pub id: String,
    pub thread_id: String,
    pub title: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[tauri::command]
pub fn list_agent_plans(
    state: State<'_, DatabaseState>,
    thread_id: String,
) -> Result<Vec<AgentPlanRow>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, thread_id, title, status, created_at, updated_at
             FROM agent_plans WHERE thread_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![thread_id], |row| {
            Ok(AgentPlanRow {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_plan_step(
    state: State<'_, DatabaseState>,
    id: String,
    plan_id: String,
    phase: String,
    description: String,
    position: i64,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO agent_plan_steps (id, plan_id, phase, description, status, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?6)",
        params![id, plan_id, phase, description, position, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_plan_step(
    state: State<'_, DatabaseState>,
    id: String,
    status: String,
    tool_call_id: Option<String>,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE agent_plan_steps SET status = ?2, tool_call_id = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, status, tool_call_id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
