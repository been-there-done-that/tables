use crate::agent_manager::{AgentManager, AgentSession, AgentMessage};
use tauri::State;
use std::sync::Arc;

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

#[tauri::command]
pub async fn update_agent_session(
    id: String,
    title: String,
    state: State<'_, AgentManagerState>,
) -> Result<AgentSession, String> {
    state.0.update_session(&id, title)
}

#[derive(serde::Deserialize)]
struct OpenAiModel {
    id: String,
}

#[derive(serde::Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[tauri::command]
pub async fn fetch_models(
    api_url: String,
    api_key: Option<String>,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let base_url = api_url.trim_end_matches('/');
    
    // Helper to try fetching
    async fn try_fetch(client: &reqwest::Client, url: &str, key: Option<&String>) -> Result<Vec<String>, String> {
        let mut request = client.get(url);
        if let Some(k) = key {
            if !k.is_empty() {
                request = request.header("Authorization", format!("Bearer {}", k));
            }
        }

        match request.send().await {
            Ok(res) => {
                if res.status().is_success() {
                    let json: OpenAiModelsResponse = res.json().await.map_err(|e| e.to_string())?;
                    let mut models: Vec<String> = json.data.into_iter().map(|m| m.id).collect();
                    models.sort();
                    models.dedup(); // Remove duplicates
                    Ok(models)
                } else {
                    Err(format!("Request failed with status: {}", res.status()))
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }

    // 1. Try exact URL + /models
    if let Ok(models) = try_fetch(&client, &format!("{}/models", base_url), api_key.as_ref()).await {
        return Ok(models);
    }

    // 2. Try appending /v1/models if not present
    if !base_url.ends_with("/v1") {
        if let Ok(models) = try_fetch(&client, &format!("{}/v1/models", base_url), api_key.as_ref()).await {
            return Ok(models);
        }
    }

    // 3. Fallback: maybe the user pasted full URL
    if base_url.ends_with("/models") {
         if let Ok(models) = try_fetch(&client, base_url, api_key.as_ref()).await {
            return Ok(models);
        }
    }

    Err("Failed to fetch models from any attempted endpoint".to_string())
}
