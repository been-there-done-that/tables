use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;
use sysinfo::{Pid, System, get_current_pid};
use serde::Serialize;
use log::debug;

/// System metrics response structure
#[derive(Serialize, Clone)]
pub struct SystemMetrics {
    /// CPU usage normalized to 0–100 across all logical cores.
    pub cpu_percent: f32,
    /// Number of threads across the process tree.
    pub threads: usize,
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

    fn collect_process_tree(sys: &System, root_pid: sysinfo::Pid) -> (f32, usize) {
        // Build a parent->children map once so traversal is O(n).
        let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
        for (pid, proc) in sys.processes() {
            if let Some(parent) = proc.parent() {
                children.entry(parent).or_default().push(*pid);
            }
        }

        let mut total_cpu = 0.0;
        let mut total_threads = 0usize;
        let mut stack = vec![root_pid];
        let mut seen = HashSet::new();

        while let Some(pid) = stack.pop() {
            if !seen.insert(pid) {
                continue;
            }
            if let Some(process) = sys.process(pid) {
                total_cpu += process.cpu_usage();
                // tasks() is None on some platforms; fall back to 1 when unavailable.
                total_threads += process.tasks().map(|tasks| tasks.len()).unwrap_or(1);

                if let Some(child_pids) = children.get(&pid) {
                    stack.extend(child_pids.iter().copied());
                }
            }
        }

        (total_cpu, total_threads)
    }

    // Get current process info
    let pid = get_current_pid().map_err(|e| e.to_string())?;
    if sys.process(pid).is_some() {
        let (total_cpu, total_threads) = collect_process_tree(&sys, pid);
        let cores = sys.cpus().len().max(1) as f32;
        let cpu_percent = total_cpu / cores;
        let metrics = SystemMetrics {
            cpu_percent,
            threads: total_threads,
        };

        debug!(
            "System metrics collected: CPU(norm)={:.2}%, Threads={}",
            metrics.cpu_percent,
            metrics.threads
        );

        Ok(metrics)
    } else {
        Err("Could not find current process".to_string())
    }
}
