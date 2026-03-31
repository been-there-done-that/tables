// src-tauri/src/commands/export_commands.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use serde::{Deserialize, Serialize};

// ─── Public types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Tsv,
    Json,
    Jsonl,
    SqlInsert,
    SqlScript,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Connecting,
    Executing,
    Streaming,
    Done,
    Error,
    Cancelled,
}

/// Emitted to the frontend on each progress tick (every ~500ms or 1000 rows).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub export_id: String,
    pub status: ExportStatus,
    pub rows_written: u64,
    pub bytes_written: u64,
    pub elapsed_ms: u64,
    pub file_path: String,
    pub error: Option<String>,
}

// ─── State ──────────────────────────────────────────────────────────────────

struct ActiveExport {
    token: CancellationToken,
}

#[derive(Default)]
pub struct ExportState {
    pub active: Arc<Mutex<HashMap<String, ActiveExport>>>,
}

use tauri::State;

#[tauri::command]
pub async fn cancel_export(
    export_id: String,
    export_state: State<'_, ExportState>,
) -> Result<(), String> {
    let map = export_state.active.lock().await;
    if let Some(handle) = map.get(&export_id) {
        handle.token.cancel();
    }
    Ok(())
}
