use sysinfo::{get_current_pid, System};
use serde::Serialize;
use log::debug;

/// System metrics response structure
#[derive(Serialize, Clone)]
pub struct SystemMetrics {
    pub cpu_usage: f32,      // CPU usage percentage
    pub memory_kb: u64,      // Memory usage in KB
    pub thread_count: usize, // Number of threads
}

/// Get current system metrics for the app process
#[tauri::command]
pub fn get_system_metrics() -> Result<SystemMetrics, String> {
    debug!("Collecting system metrics");

    let mut sys = System::new_all();

    // Refresh system info
    sys.refresh_all();

    // Get current process info
    let pid = get_current_pid().map_err(|e| e.to_string())?;
    if let Some(process) = sys.process(pid) {
        let metrics = SystemMetrics {
            cpu_usage: process.cpu_usage(),
            // sysinfo 0.30 returns bytes; convert to KB for display
            memory_kb: process.memory() / 1024,
            thread_count: process.tasks().map(|tasks| tasks.len()).unwrap_or(0),
        };

        debug!("System metrics collected: CPU={:.2}%, Memory={}KB, Threads={}",
               metrics.cpu_usage, metrics.memory_kb, metrics.thread_count);

        Ok(metrics)
    } else {
        Err("Could not find current process".to_string())
    }
}
