use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Listener, Manager};

/// Port of the running harness process, set once when it's ready.
static HARNESS_PORT: OnceLock<u16> = OnceLock::new();

fn binary_name() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "harness-aarch64-apple-darwin";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "harness-x86_64-apple-darwin";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "harness-x86_64-unknown-linux-gnu";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "harness-x86_64-pc-windows-msvc.exe";
    #[allow(unreachable_code)]
    "harness"
}

pub fn spawn(app: AppHandle) {
    log::info!("[harness] spawn() called");

    // Dev bypass: HARNESS_PORT env var skips spawning the binary
    if let Ok(port_str) = std::env::var("HARNESS_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            log::info!("[harness] HARNESS_PORT env var set → using port {port}");
            HARNESS_PORT.set(port).ok();
            app.emit("harness://ready", port).ok();
            return;
        }
    }

    #[cfg(debug_assertions)]
    let binary_path = {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(binary_name());
        log::info!("[harness] debug binary path: {:?} exists={}", p, p.exists());
        p
    };

    #[cfg(not(debug_assertions))]
    let binary_path = {
        let exe = match std::env::current_exe() {
            Ok(e) => e,
            Err(e) => { log::error!("[harness] cannot resolve current exe: {e}"); return; }
        };
        let p = exe.parent().unwrap().join(binary_name());
        log::info!("[harness] release binary path: {:?} exists={}", p, p.exists());
        p
    };

    if !binary_path.exists() {
        log::error!("[harness] binary not found — agent panel unavailable");
        return;
    }

    log::info!("[harness] spawning process...");
    std::thread::spawn(move || {
        // Pass through shell env so the harness can find the claude CLI in PATH
        let path_env = std::env::var("PATH").unwrap_or_default();
        let home_env = std::env::var("HOME").unwrap_or_default();
        log::info!("[harness] PATH={}", path_env);

        let mut child = match Command::new(&binary_path)
            .env("PATH", path_env)
            .env("HOME", home_env)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(c) => { log::info!("[harness] spawned pid={}", c.id()); c }
            Err(e) => { log::error!("[harness] spawn failed: {e}"); return; }
        };

        let stdout = child.stdout.take().unwrap();
        for line in BufReader::new(stdout).lines().flatten() {
            if let Some(port_str) = line.strip_prefix("HARNESS_PORT=") {
                if let Ok(port) = port_str.parse::<u16>() {
                    log::info!("[harness] ready on port {port}");
                    HARNESS_PORT.set(port).ok();
                    // Emit to all current windows
                    app.emit("harness://ready", port).ok();
                    // Re-emit whenever a new window opens (covers late-loading WebViews)
                    let app2 = app.clone();
                    app.listen("window-created", move |_| {
                        app2.emit("harness://ready", port).ok();
                    });
                    break;
                }
            }
        }
    });
}

/// Tauri command: frontend calls this on startup to get the port immediately,
/// avoiding the race where the event fires before the listener is registered.
#[tauri::command]
pub fn get_harness_port() -> Option<u16> {
    let port = HARNESS_PORT.get().copied();
    log::info!("[harness] get_harness_port() → {:?}", port);
    port
}
