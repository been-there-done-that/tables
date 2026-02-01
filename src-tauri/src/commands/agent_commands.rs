use crate::agent_manager::{AgentManager, AgentSession, AgentMessage};
use crate::DatabaseState;
use tauri::State;
use std::sync::{Arc, Mutex};

pub struct AgentManagerState(pub Arc<AgentManager>);

#[tauri::command]
pub async fn create_agent_session(
    title: String,
    state: State<'_, AgentManagerState>,
) -> Result<AgentSession, String> {
    state.0.create_session(title)
}

#[tauri::command]
pub async fn list_agent_sessions(
    state: State<'_, AgentManagerState>,
) -> Result<Vec<AgentSession>, String> {
    state.0.list_sessions()
}

#[tauri::command]
pub async fn add_agent_message(
    message: AgentMessage,
    state: State<'_, AgentManagerState>,
) -> Result<(), String> {
    state.0.add_message(message)
}

#[tauri::command]
pub async fn get_agent_messages(
    session_id: String,
    state: State<'_, AgentManagerState>,
) -> Result<Vec<AgentMessage>, String> {
    state.0.get_messages(&session_id)
}

#[tauri::command]
pub async fn delete_agent_session(
    id: String,
    state: State<'_, AgentManagerState>,
) -> Result<(), String> {
    state.0.delete_session(&id)
}
