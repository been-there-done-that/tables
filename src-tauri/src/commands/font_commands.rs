use font_kit::source::SystemSource;
use tauri::command;

#[command]
pub async fn get_system_fonts() -> Result<Vec<String>, String> {
    let source = SystemSource::new();
    let fonts = source.all_families().map_err(|e| e.to_string())?;
    
    // Sort and deduplicate
    let mut fonts_vec: Vec<String> = fonts.into_iter().collect();
    fonts_vec.sort();
    fonts_vec.dedup();
    
    Ok(fonts_vec)
}
