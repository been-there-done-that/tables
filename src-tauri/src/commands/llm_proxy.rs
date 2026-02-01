use tauri::Emitter;
use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use futures_util::StreamExt;
use log::{debug, error, info};

#[derive(Debug, Deserialize)]
pub struct StreamRequest {
    pub provider: String, // "openai", "anthropic", etc.
    pub api_key: String,
    pub api_url: Option<String>,
    pub model: String,
    pub messages: serde_json::Value,
    pub tools: Option<serde_json::Value>,
    pub temperature: Option<f32>,
    pub persist: Option<bool>, // Control database persistence
}

#[derive(Debug, Serialize, Clone)]
pub struct StreamEvent {
    pub session_id: String,
    pub chunk: Option<String>,
    pub done: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn llm_stream(
    // Inject DatabaseState
    state: tauri::State<'_, crate::DatabaseState>, 
    window: tauri::Window,
    session_id: String,
    request: StreamRequest,
) -> Result<(), String> {
    debug!("Starting LLM stream for session: {}", session_id);

    // ... (client setup) ...

    let client = reqwest::Client::new();
    
    // ... (URL construction) ...
    let url = if let Some(custom_url) = &request.api_url {
        if request.provider == "openai" && !custom_url.contains("/chat/completions") {
             format!("{}/chat/completions", custom_url.trim_end_matches('/'))
        } else {
             custom_url.clone()
        }
    } else {
        match request.provider.as_str() {
            "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
            "anthropic" => "https://api.anthropic.com/v1/messages".to_string(),
            _ => return Err(format!("Unsupported provider: {}", request.provider)),
        }
    };

    let mut body = serde_json::json!({
        "model": request.model,
        "messages": request.messages,
        "stream": true,
    });

    if let Some(tools) = request.tools {
        body.as_object_mut().unwrap().insert("tools".to_string(), tools);
    }
    if let Some(temp) = request.temperature {
        body.as_object_mut().unwrap().insert("temperature".to_string(), serde_json::json!(temp));
    }

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", request.api_key)).map_err(|e| e.to_string())?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if request.provider == "anthropic" {
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert("x-api-key", HeaderValue::from_str(&request.api_key).map_err(|e| e.to_string())?);
    }

    let response = client.post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("API error: {}", err_body));
    }

    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let should_persist = request.persist.unwrap_or(true);

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let data = String::from_utf8_lossy(&bytes);
                for line in data.lines() {
                    if line.starts_with("data: ") {
                        let content = &line[6..];
                        if content == "[DONE]" {
                            // Stream finished, persist full response
                            if should_persist {
                                if let Ok(conn) = state.conn.lock() {
                                    let message_id = uuid::Uuid::new_v4().to_string();
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs() as i64;

                                    let _ = conn.execute(
                                        "INSERT INTO agent_messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                                        rusqlite::params![message_id, session_id, "assistant", full_response, now],
                                    );
                                    info!("Persisted assistant message for session {}", session_id);
                                } else {
                                    error!("Failed to lock database for persisting message");
                                }
                            }

                            let _ = window.emit("llm-chunk", StreamEvent {
                                session_id: session_id.clone(),
                                chunk: None,
                                done: true,
                                error: None,
                            });
                            break;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                            if let Some(choices) = json.get("choices") {
                                if let Some(delta) = choices[0].get("delta") {
                                    // Handle text content
                                    if let Some(text) = delta.get("content") {
                                        let chunk_str = text.as_str().unwrap_or_default();
                                        full_response.push_str(chunk_str); // Accumulate

                                        let _ = window.emit("llm-chunk", StreamEvent {
                                            session_id: session_id.clone(),
                                            chunk: Some(chunk_str.to_string()),
                                            done: false,
                                            error: None,
                                        });
                                    }
                                    
                                    // Handle tool calls
                                    if let Some(tool_calls) = delta.get("tool_calls") {
                                        let tool_json = serde_json::to_string(tool_calls).unwrap_or_default();
                                        let _ = window.emit("llm-tool-call", serde_json::json!({
                                            "session_id": session_id.clone(),
                                            "tool_calls": tool_calls,
                                        }));
                                    }
                                }
                                
                                // Check for finish_reason to detect tool_calls completion
                                if let Some(finish_reason) = choices[0].get("finish_reason") {
                                    if finish_reason.as_str() == Some("tool_calls") {
                                        // The model wants to call tools
                                        let _ = window.emit("llm-chunk", StreamEvent {
                                            session_id: session_id.clone(),
                                            chunk: Some("[TOOL_CALLS]".to_string()),
                                            done: false,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = window.emit("llm-chunk", StreamEvent {
                    session_id: session_id.clone(),
                    chunk: None,
                    done: true,
                    error: Some(e.to_string()),
                });
                return Err(e.to_string());
            }
        }
    }

    Ok(())
}
