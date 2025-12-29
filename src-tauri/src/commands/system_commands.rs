use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use sysinfo::{Pid, System, get_current_pid};
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

    // Refresh twice with a short pause so CPU usage has a delta window
    sys.refresh_processes();
    thread::sleep(Duration::from_millis(200));
    sys.refresh_processes();

    fn collect_process_tree(sys: &System, root_pid: sysinfo::Pid) -> (f32, u64, usize) {
        let mut total_cpu = 0.0;
        let mut total_mem_bytes = 0u64;
        let mut total_threads = 0usize;
        let mut stack = vec![root_pid];
        let mut seen = HashSet::new();

        while let Some(pid) = stack.pop() {
            if !seen.insert(pid) {
                continue;
            }
            if let Some(process) = sys.process(pid) {
                total_cpu += process.cpu_usage();
                total_mem_bytes += process.memory();
                total_threads += process.tasks().map(|tasks| tasks.len()).unwrap_or(0);
                println!("Process {} has {} threads {} pid {}", process.name(), total_threads, total_cpu, pid);

                for (child_pid, child_proc) in sys.processes() {
                    if child_proc.parent() == Some(pid) {
                        stack.push(*child_pid);
                    }
                }
            }
        }

        (total_cpu, total_mem_bytes, total_threads)
    }

    // Get current process info
    let pid = get_current_pid().map_err(|e| e.to_string())?;
    if sys.process(pid).is_some() {
        let (total_cpu, total_mem_bytes, total_threads) = collect_process_tree(&sys, pid);
        let metrics = SystemMetrics {
            cpu_usage: total_cpu,
            // sysinfo 0.30 returns bytes; convert to KB for display
            memory_kb: total_mem_bytes / 1024,
            thread_count: total_threads,
        };

        debug!("System metrics collected: CPU={:.2}%, Memory={}KB, Threads={}",
               metrics.cpu_usage, metrics.memory_kb, metrics.thread_count);

        Ok(metrics)
    } else {
        Err("Could not find current process".to_string())
    }
}
