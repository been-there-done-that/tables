use tauri::{Manager, WebviewWindowBuilder, TitleBarStyle, Emitter};
use log::{info, debug, error, trace};

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
    let mut builder = WebviewWindowBuilder::new(&app, LABEL, tauri::WebviewUrl::App("/datasource".into()))
        .title("Datasource")
        .inner_size(960.0, 640.0)
        .resizable(true)
        .decorations(cfg!(target_os = "macos"));

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(TitleBarStyle::Overlay)
            .hidden_title(true);
    }

    let builder = builder
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
    let mut builder = WebviewWindowBuilder::new(&app, LABEL, tauri::WebviewUrl::App("/settings".into()))
        .title("Appearance")
        .inner_size(960.0, 640.0)
        .resizable(true)
        .decorations(cfg!(target_os = "macos"));

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(TitleBarStyle::Overlay)
            .hidden_title(true);
    }

    let builder = builder
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

#[tauri::command]
pub async fn create_new_window(app: tauri::AppHandle) -> Result<(), String> {
    let id = uuid::Uuid::new_v4();
    let label = format!("window-{}", id);
    debug!("Creating new independent window: {}", label);

    // Create a new window with the same configuration as the main window
    let mut builder = WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App("/".into()))
        .title("Tables")
        // Use a default size, or let the OS/Tauri handle it. 
        // Emulating the main window's typical startup size if desired, 
        // but typically OS placement is fine for new windows.
        .inner_size(1200.0, 800.0) 
        .min_inner_size(800.0, 600.0)
        .resizable(true)
        .decorations(cfg!(target_os = "macos"));

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(TitleBarStyle::Overlay)
            .hidden_title(true);
    }

    let builder = builder
        .transparent(true)
        .focused(true);

    let window = builder
        .build()
        .map_err(|e| {
            error!("Failed to create new window {}: {}", label, e);
            format!("Failed to create new window: {}", e)
        })?;

    // Emit event just in case we need to track it, though our current logic 
    // mainly tracks specialized windows.
    trace!("Emitting window-created event for {}", label);
    let _ = app.emit("window-created", window.label());

    info!("New window {} created successfully", label);
    Ok(())
}
