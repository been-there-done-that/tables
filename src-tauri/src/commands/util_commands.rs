#[tauri::command]
pub async fn save_svg_file(path: String, svg: String) -> Result<(), String> {
    let dest = std::path::PathBuf::from(&path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
    }
    std::fs::write(&dest, svg.as_bytes()).map_err(|e| format!("write error: {e}"))?;
    Ok(())
}
