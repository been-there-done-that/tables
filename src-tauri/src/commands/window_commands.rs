use tauri::{Manager, WebviewWindowBuilder, TitleBarStyle, Emitter, State};
use log::{info, debug, error, trace};
use crate::constants::ENABLE_WINDOW_EVENTS;
use crate::{DatabaseState, ConnectionManagerState, ConnectionManager};

fn get_preferred_window_size(app: &tauri::AppHandle) -> (f64, f64) {
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let size = monitor.size();
        let scale_factor = monitor.scale_factor();
        let logical_width = size.width as f64 / scale_factor;
        let logical_height = size.height as f64 / scale_factor;
        (logical_width * 0.8, logical_height * 0.8)
    } else {
        (960.0, 640.0)
    }
}

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
    let (width, height) = get_preferred_window_size(&app);

    let mut builder = WebviewWindowBuilder::new(&app, LABEL, tauri::WebviewUrl::App("/datasource".into()))
        .title("Datasource")
        .inner_size(width, height)
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
    if ENABLE_WINDOW_EVENTS {
        let _ = app.emit("window-created", window.label());
    }

    info!("Datasource window opened successfully");
    Ok(())
}

#[tauri::command]
pub async fn open_appearance_window(app: tauri::AppHandle, section: Option<String>) -> Result<(), String> {
    const LABEL: &str = "appearance-window";
    debug!("Opening appearance window (section: {:?})", section);

    if let Some(window) = app.get_webview_window(LABEL) {
        trace!("Appearance window already exists, focusing");
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        // Tell the already-open settings page to switch to the requested section
        if let Some(ref sec) = section {
            let _ = window.emit("settings:switch-section", sec.clone());
        }
        return Ok(());
    }

    debug!("Creating new appearance window");

    // Calculate dynamic size (70% of primary monitor)
    let (width, height) = get_preferred_window_size(&app);

    let url_path = match &section {
        Some(sec) => format!("/settings?section={}", sec),
        None => "/settings".to_string(),
    };

    let mut builder = WebviewWindowBuilder::new(&app, LABEL, tauri::WebviewUrl::App(url_path.into()))
        .title("Appearance")
        .inner_size(width, height)
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
    if ENABLE_WINDOW_EVENTS {
        let _ = app.emit("window-created", window.label());
    }

    info!("Appearance window opened successfully");
    Ok(())
}

#[tauri::command]
pub async fn create_new_window(
    app: tauri::AppHandle,
    connection_id: Option<String>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let mut label = if let Some(ref conn_id) = connection_id {
        format!("window-isolated-{}", conn_id)
    } else {
        format!("window-{}", uuid::Uuid::new_v4())
    };

    // If the stable label is already in use, fallback to a dynamic UUID 
    // to allow multi-window support (mostly for Postgres).
    // These transient windows won't persist state by default because their labes are dynamic.
    if app.get_webview_window(&label).is_some() {
        label = format!("window-{}", uuid::Uuid::new_v4());
    }

    debug!("Creating new independent window: {}", label);
 
    // If a connection ID is provided, pre-save the window session for this new label
    if let Some(conn_id) = connection_id {
        let manager = ConnectionManager::from_state(&db_state, &conn_state);
        manager.save_window_session(&label, &conn_id)?;
        debug!("Pre-saved session for connection {} in window {}", conn_id, label);
    }

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
    if ENABLE_WINDOW_EVENTS {
        let _ = app.emit("window-created", window.label());
    }

    info!("New window {} created successfully", label);
    Ok(())
}

#[tauri::command]
pub async fn open_feedback_window(app: tauri::AppHandle) -> Result<(), String> {
    const LABEL: &str = "feedback-window";
    debug!("Opening feedback window");

    if let Some(existing) = app.get_webview_window(LABEL) {
        trace!("Feedback window already exists, focusing");
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    debug!("Creating new feedback window");

    let mut builder = WebviewWindowBuilder::new(
        &app,
        LABEL,
        tauri::WebviewUrl::App("feedback".into()),
    )
    .title("Send Feedback")
    .inner_size(560.0, 580.0)
    .resizable(false)
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

    let _window = builder
        .build()
        .map_err(|e| {
            error!("Failed to create feedback window: {}", e);
            format!("Failed to create feedback window: {}", e)
        })?;

    info!("Feedback window opened successfully");
    Ok(())
}
