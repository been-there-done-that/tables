use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[derive(Default)]
pub struct UpdaterState {
    pub pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
    pub downloaded_bytes: Mutex<Option<Vec<u8>>>,
}

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = match updater.check().await {
        Ok(u) => u,
        Err(e) => {
            // Treat network/404 errors as "no update available" rather than propagating
            // the error — this avoids noise in dev and when no release has been published yet.
            let msg = e.to_string();
            if msg.contains("404")
                || msg.contains("Not Found")
                || msg.contains("release JSON")
                || msg.contains("failed to send request")
                || msg.contains("network")
            {
                return Ok(None);
            }
            return Err(msg);
        }
    };

    match update {
        Some(u) => {
            let info = UpdateInfo {
                version: u.version.clone(),
                body: u.body.clone(),
                date: u.date.map(|d| d.to_string()),
            };
            *state.pending_update.lock().unwrap() = Some(u);
            Ok(Some(info))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<(), String> {
    let update = {
        let guard = state.pending_update.lock().unwrap();
        guard.as_ref().ok_or("No pending update")?.clone()
    };

    let app_clone = app.clone();
    let mut downloaded_total: u64 = 0;
    let bytes = update
        .download(
            move |chunk_length, content_length| {
                downloaded_total += chunk_length as u64;
                let _ = app_clone.emit(
                    "update://progress",
                    DownloadProgress {
                        downloaded: downloaded_total,
                        total: content_length,
                    },
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    *state.downloaded_bytes.lock().unwrap() = Some(bytes);
    Ok(())
}

#[tauri::command]
pub async fn install_update(state: State<'_, UpdaterState>) -> Result<(), String> {
    let update = {
        let guard = state.pending_update.lock().unwrap();
        guard.as_ref().ok_or("No pending update")?.clone()
    };
    let bytes = state
        .downloaded_bytes
        .lock()
        .unwrap()
        .take()
        .ok_or("Update not downloaded yet")?;

    update.install(bytes).map_err(|e| e.to_string())?;
    *state.pending_update.lock().unwrap() = None;
    Ok(())
}
