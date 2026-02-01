use rusqlite::{params, Connection as SqliteConnection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentSession {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<String>, // JSON string
    pub tool_call_id: Option<String>,
    pub created_at: i64,
}

pub struct AgentManager {
    db: Arc<Mutex<SqliteConnection>>,
}

impl AgentManager {
    pub fn new(db: Arc<Mutex<SqliteConnection>>) -> Self {
        Self { db }
    }

    pub fn create_session(&self, title: String) -> Result<AgentSession, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        let session = AgentSession {
            id: Uuid::new_v4().to_string(),
            title,
            created_at: now,
            updated_at: now,
        };

        let conn = self.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO agent_sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![session.id, session.title, session.created_at, session.updated_at],
        ).map_err(|e| e.to_string())?;

        Ok(session)
    }

    pub fn list_sessions(&self) -> Result<Vec<AgentSession>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, title, created_at, updated_at FROM agent_sessions ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map([], |row| {
            Ok(AgentSession {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row.map_err(|e| e.to_string())?);
        }
        Ok(sessions)
    }

    pub fn add_message(&self, message: AgentMessage) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        
        conn.execute(
            "INSERT INTO agent_messages (id, session_id, role, content, tool_calls, tool_call_id, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                message.id,
                message.session_id,
                message.role,
                message.content,
                message.tool_calls,
                message.tool_call_id,
                message.created_at
            ],
        ).map_err(|e| e.to_string())?;

        // Update session updated_at
        conn.execute(
            "UPDATE agent_sessions SET updated_at = ?1 WHERE id = ?2",
            params![message.created_at, message.session_id],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn get_messages(&self, session_id: &str) -> Result<Vec<AgentMessage>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, tool_calls, tool_call_id, created_at 
             FROM agent_messages WHERE session_id = ?1 ORDER BY created_at ASC"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(AgentMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: row.get(4)?,
                tool_call_id: row.get(5)?,
                created_at: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| e.to_string())?);
        }
        Ok(messages)
    }

    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        // Cascading delete should handle messages
        conn.execute("DELETE FROM agent_sessions WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_session(&self, id: &str, title: String) -> Result<AgentSession, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
            
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        
        // Check if exists first to return error? Or just update.
        // Let's rely on execute returning 0 affected rows if not found, but we want the updated object back.
        
        conn.execute(
            "UPDATE agent_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, now, id],
        ).map_err(|e| e.to_string())?;

        // Retrieve to return full object
        let mut stmt = conn.prepare("SELECT id, title, created_at, updated_at FROM agent_sessions WHERE id = ?1")
            .map_err(|e| e.to_string())?;
            
        let session = stmt.query_row(params![id], |row| {
            Ok(AgentSession {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(session)
    }
}
