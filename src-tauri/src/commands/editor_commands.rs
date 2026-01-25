use crate::DatabaseState;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSession {
    pub id: String,
    pub window_label: String,
    pub connection_id: Option<String>,
    pub schema_name: Option<String>,
    pub content: String,
    pub cursor_line: i64,
    pub cursor_column: i64,
    pub created_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSessionSummary {
    pub id: String,
    pub window_label: String,
    pub connection_id: Option<String>,
    pub schema_name: Option<String>,
    pub created_at: i64,
    pub last_opened_at: i64,
}

#[tauri::command]
pub fn save_editor_session(
    state: State<'_, DatabaseState>,
    id: String,
    window_label: String,
    connection_id: Option<String>,
    schema_name: Option<String>,
    content: String,
    cursor_line: i64,
    cursor_column: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO editor_sessions (id, window_label, connection_id, schema_name, content, cursor_line, cursor_column, created_at, last_opened_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(id) DO UPDATE SET
             window_label = excluded.window_label,
             connection_id = excluded.connection_id,
             schema_name = excluded.schema_name,
             content = excluded.content,
             cursor_line = excluded.cursor_line,
             cursor_column = excluded.cursor_column,
             last_opened_at = excluded.last_opened_at",
        params![id, window_label, connection_id, schema_name, content, cursor_line, cursor_column, now],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn load_editor_session(
    state: State<'_, DatabaseState>,
    id: String,
) -> Result<Option<EditorSession>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, window_label, connection_id, schema_name, content, cursor_line, cursor_column, created_at, last_opened_at
             FROM editor_sessions WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let session = stmt
        .query_row(params![id], |row| {
            Ok(EditorSession {
                id: row.get(0)?,
                window_label: row.get(1)?,
                connection_id: row.get(2)?,
                schema_name: row.get(3)?,
                content: row.get(4)?,
                cursor_line: row.get(5)?,
                cursor_column: row.get(6)?,
                created_at: row.get(7)?,
                last_opened_at: row.get(8)?,
            })
        })
        .optional()
        .map_err(|e| e.to_string())?;

    Ok(session)
}

#[tauri::command]
pub fn list_editor_sessions(
    state: State<'_, DatabaseState>,
    window_label: Option<String>,
    connection_id: Option<String>,
) -> Result<Vec<EditorSessionSummary>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let mut query = "SELECT id, window_label, connection_id, schema_name, created_at, last_opened_at FROM editor_sessions WHERE 1=1".to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(ref wl) = window_label {
        query.push_str(" AND window_label = ?");
        params_vec.push(Box::new(wl.clone()));
    }
    if let Some(ref cid) = connection_id {
        query.push_str(" AND connection_id = ?");
        params_vec.push(Box::new(cid.clone()));
    }
    query.push_str(" ORDER BY last_opened_at DESC");

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(EditorSessionSummary {
                id: row.get(0)?,
                window_label: row.get(1)?,
                connection_id: row.get(2)?,
                schema_name: row.get(3)?,
                created_at: row.get(4)?,
                last_opened_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row.map_err(|e| e.to_string())?);
    }

    Ok(sessions)
}

#[tauri::command]
pub fn delete_editor_session(
    state: State<'_, DatabaseState>,
    id: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM editor_sessions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
