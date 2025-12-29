use tauri::{Manager, WebviewWindowBuilder, TitleBarStyle, Emitter};
use log::{info, debug, warn, error, trace};

#[tauri::command]
pub async fn open_datasource_window(app: tauri::AppHandle) -> Result<(), String> {
    const LABEL: &str = "datasource-window";
    debug!("Opening datasource window");

    if let Some(window) = app.get_webview_window(LABEL) {
        trace!("Datasource window already exists, focusing");
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    debug!("Creating new datasource window");
    let builder = WebviewWindowBuilder::new(&app, LABEL, tauri::WebviewUrl::App("/datasource".into()))
        .title("Datasource")
        .inner_size(960.0, 640.0)
        .resizable(true)
        .decorations(true)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .transparent(true)
        .focused(true);

    let window = builder
        .build()
        .map_err(|e| {
            error!("Failed to create datasource window: {}", e);
            format!("Failed to create datasource window: {}", e)
        })?;

    trace!("Emitting window-created event for datasource");
    let _ = app.emit("window-created", window.label());

    info!("Datasource window opened successfully");
    Ok(())
}

#[tauri::command]
pub async fn open_appearance_window(app: tauri::AppHandle) -> Result<(), String> {
    const LABEL: &str = "appearance-window";
    debug!("Opening appearance window");

    if let Some(window) = app.get_webview_window(LABEL) {
        trace!("Appearance window already exists, focusing");
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    debug!("Creating new appearance window");
    let builder = WebviewWindowBuilder::new(&app, LABEL, tauri::WebviewUrl::App("/settings".into()))
        .title("Appearance")
        .inner_size(960.0, 640.0)
        .resizable(true)
        .decorations(true)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .transparent(true)
        .focused(true);

    let window = builder
        .build()
        .map_err(|e| {
            error!("Failed to create appearance window: {}", e);
            format!("Failed to create appearance window: {}", e)
        })?;

    trace!("Emitting window-created event for appearance");
    let _ = app.emit("window-created", window.label());

    info!("Appearance window opened successfully");
    Ok(())
}
