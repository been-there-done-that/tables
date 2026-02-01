use tauri::Emitter;
use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use futures_util::StreamExt;
use log::{debug, error, info};

#[derive(Debug, Deserialize)]
pub struct StreamRequest {
    pub provider: String, // "openai", "anthropic", etc.
    pub api_key: String,
    pub model: String,
    pub messages: serde_json::Value,
    pub tools: Option<serde_json::Value>,
    pub temperature: Option<f32>,
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
    window: tauri::Window,
    session_id: String,
    request: StreamRequest,
) -> Result<(), String> {
    debug!("Starting LLM stream for session: {}", session_id);

    let client = reqwest::Client::new();
    let url = match request.provider.as_str() {
        "openai" => "https://api.openai.com/v1/chat/completions",
        "anthropic" => "https://api.anthropic.com/v1/messages",
        _ => return Err(format!("Unsupported provider: {}", request.provider)),
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

    // Anthropic specific headers
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

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let data = String::from_utf8_lossy(&bytes);
                // Process SSE chunks
                for line in data.lines() {
                    if line.starts_with("data: ") {
                        let content = &line[6..];
                        if content == "[DONE]" {
                            let _ = window.emit("llm-chunk", StreamEvent {
                                session_id: session_id.clone(),
                                chunk: None,
                                done: true,
                                error: None,
                            });
                            break;
                        }

                        // For OpenAI format
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                            if let Some(choices) = json.get("choices") {
                                if let Some(delta) = choices[0].get("delta") {
                                    if let Some(text) = delta.get("content") {
                                        let _ = window.emit("llm-chunk", StreamEvent {
                                            session_id: session_id.clone(),
                                            chunk: Some(text.as_str().unwrap_or_default().to_string()),
                                            done: false,
                                            error: None,
                                        });
                                    }
                                    // Tool calls support
                                    if let Some(tool_calls) = delta.get("tool_calls") {
                                        let _ = window.emit("llm-chunk", StreamEvent {
                                            session_id: session_id.clone(),
                                            chunk: Some(tool_calls.to_string()),
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
