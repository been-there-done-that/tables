use base64::Engine as _;
use std::path::PathBuf;

/// Write a base64-encoded PNG (data-URL or raw base64) to the given file path.
/// Accepts a full data-URL like `data:image/png;base64,<...>` or raw base64.
#[tauri::command]
pub async fn save_png_file(path: String, data_url: String) -> Result<(), String> {
    let b64 = if let Some(rest) = data_url.strip_prefix("data:") {
        // strip "image/png;base64," header
        rest.splitn(2, ',').nth(1).unwrap_or(&data_url)
    } else {
        &data_url
    };

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("base64 decode error: {e}"))?;

    let dest = PathBuf::from(&path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
    }
    std::fs::write(&dest, &bytes).map_err(|e| format!("write error: {e}"))?;

    Ok(())
}
