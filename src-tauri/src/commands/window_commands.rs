use tauri::{Manager, WebviewWindowBuilder, TitleBarStyle};

#[tauri::command]
pub async fn open_datasource_window(app: tauri::AppHandle) -> Result<(), String> {
    const LABEL: &str = "datasource-window";

    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let builder = WebviewWindowBuilder::new(&app, LABEL, tauri::WebviewUrl::App("/datasource".into()))
        .title("Datasource")
        .inner_size(960.0, 640.0)
        .resizable(false)
        .decorations(true)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .transparent(true)
        .focused(true);

    let _window = builder
        .build()
        .map_err(|e| format!("Failed to create datasource window: {}", e))?;

    Ok(())
}
