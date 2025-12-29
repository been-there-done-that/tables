use std::collections::{HashMap, HashSet};
use std::env;
use std::time::Duration;
use sysinfo::{Pid, System};
use serde::Serialize;
use log::debug;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

/// System metrics response structure
#[derive(Serialize, Clone)]
pub struct SystemMetrics {
    /// CPU usage normalized to 0–100 across all logical cores.
    pub cpu_percent: f32,
    /// Number of threads across the process tree.
    pub threads: usize,
    /// PID of the root (current) process.
    pub pid: i32,
}

fn metrics_interval_ms() -> u64 {
    env::var("TABLES_METRICS_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v >= 200) // guard against too-fast polling
        .unwrap_or(1000)
}

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

fn collect_metrics(sys: &mut System) -> Result<SystemMetrics, String> {
    // Refresh twice with a short pause so CPU usage has a delta window
    sys.refresh_processes();
    std::thread::sleep(Duration::from_millis(200));
    sys.refresh_processes();

    // Get current process info
    let pid_u32 = std::process::id();
    let pid = Pid::from_u32(pid_u32);
    if sys.process(pid).is_some() {
        let (total_cpu, total_threads) = collect_process_tree(sys, pid);
        let cores = sys.cpus().len().max(1) as f32;
        let cpu_percent = total_cpu / cores;
        let metrics = SystemMetrics {
            cpu_percent,
            threads: total_threads,
            pid: pid_u32 as i32,
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

/// Get current system metrics for the app process (one-shot).
#[tauri::command]
pub fn get_system_metrics() -> Result<SystemMetrics, String> {
    debug!("Collecting system metrics (one-shot)");
    let mut sys = System::new_all();
    collect_metrics(&mut sys)
}

/// Start background metrics emitter; sends "metrics:update" events to all windows.
pub fn start_metrics_emitter(app_handle: AppHandle) {
    let interval_ms = metrics_interval_ms();
    log::info!("Starting metrics emitter with interval {} ms", interval_ms);
    tauri::async_runtime::spawn(async move {
        let mut sys = System::new_all();
        loop {
            if let Ok(metrics) = collect_metrics(&mut sys) {
                let _ = app_handle.emit("metrics:update", metrics);
            }
            sleep(Duration::from_millis(interval_ms)).await;
        }
    });
}
