use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use serde::Serialize;
use sysinfo::{Pid, System};
use log::{error, info};
use crate::constants::ENABLE_METRICS_EMISSION;

// --- Core Registry Implementation ---

pub enum Metric {
    Counter(Arc<AtomicU64>),
    Gauge(Arc<AtomicU64>), // store f64 via bits
    // History is handled separately to avoid enum complexity with Generics/Mutexes
}

pub struct MetricsRegistry {
    metrics: RwLock<HashMap<&'static str, Metric>>,
    histories: RwLock<HashMap<&'static str, HistoryHandle>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(HashMap::new()),
            histories: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_counter(&self, name: &'static str) -> CounterHandle {
        let mut metrics = self.metrics.write().unwrap();
        let entry = metrics.entry(name).or_insert_with(|| {
            Metric::Counter(Arc::new(AtomicU64::new(0)))
        });

        match entry {
            Metric::Counter(v) => CounterHandle(v.clone()),
            _ => panic!("Metric {name} already registered with different type"),
        }
    }

    pub fn register_gauge(&self, name: &'static str) -> GaugeHandle {
        let mut metrics = self.metrics.write().unwrap();
        let entry = metrics.entry(name).or_insert_with(|| {
            Metric::Gauge(Arc::new(AtomicU64::new(0)))
        });

        match entry {
            Metric::Gauge(v) => GaugeHandle(v.clone()),
            _ => panic!("Metric {name} already registered with different type"),
        }
    }
    
    pub fn register_history(&self, name: &'static str, capacity: usize) -> HistoryHandle {
        let mut histories = self.histories.write().unwrap();
        let entry = histories.entry(name).or_insert_with(|| {
            // (Current Sequence ID, Buffer)
            HistoryHandle(Arc::new(Mutex::new((0, VecDeque::with_capacity(capacity)))), capacity)
        });
        
        entry.clone()
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let metrics = self.metrics.read().unwrap();
        let histories = self.histories.read().unwrap();
        
        let mut values = HashMap::new();
        let mut history_values = HashMap::new();

        for (name, metric) in metrics.iter() {
            let v = match metric {
                Metric::Counter(c) => c.load(Ordering::Relaxed) as f64,
                Metric::Gauge(g) => f64::from_bits(g.load(Ordering::Relaxed)),
            };
            values.insert(*name, v);
        }
        
        for (name, handle) in histories.iter() {
             let data = handle.0.lock().unwrap();
             // Return vector of (seq_id, value)
             history_values.insert(*name, data.1.iter().cloned().collect::<Vec<(u64, f64)>>());
        }

        MetricsSnapshot { values, histories: history_values }
    }
}

#[derive(Clone)]
pub struct CounterHandle(Arc<AtomicU64>);

impl CounterHandle {
    pub fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, v: u64) {
        self.0.fetch_add(v, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct GaugeHandle(Arc<AtomicU64>);

impl GaugeHandle {
    pub fn set(&self, v: f64) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

// (Mutex<(NextSeqId, Data)>, Capacity)
#[derive(Clone)]
pub struct HistoryHandle(Arc<Mutex<(u64, VecDeque<(u64, f64)>)>>, usize);

impl HistoryHandle {
    pub fn push(&self, v: f64) {
        let mut state = self.0.lock().unwrap();
        let (ref mut seq, ref mut data) = *state;
        
        if data.len() >= self.1 {
            data.pop_front();
        }
        data.push_back((*seq, v));
        *seq += 1;
    }
}

#[derive(Serialize, PartialEq, Debug)]
pub struct MetricsSnapshot {
    values: HashMap<&'static str, f64>,
    histories: HashMap<&'static str, Vec<(u64, f64)>>,
}

// --- Emission Logic ---

pub fn start_metrics_emitter(app: AppHandle, registry: Arc<MetricsRegistry>) {
    info!("Starting metrics snapshot emitter (push-only)");
    std::thread::spawn(move || {
        let mut last = None;

        loop {
            // Emit independent of change to ensure liveness? 
            // The user wanted "only if modified". 
            // But if we want the "Welcome" push to work, that's orthogonal.
            // Let's stick to change detection, but wait... 
            // If the user's "Welcome push" is handled in lib.rs, this loop can stay effecient.
            
            std::thread::sleep(Duration::from_millis(1000)); // 1Hz

            let snapshot = registry.snapshot();

            if last.as_ref() != Some(&snapshot) {
                if let Err(e) = app.emit("metrics:snapshot", &snapshot) {
                    error!("Failed to emit metrics snapshot: {}", e);
                }
                last = Some(snapshot);
            }
        }
    });
}

// --- System Monitor Feature (Internal) ---

pub struct SystemMonitor {
    cpu_usage: GaugeHandle,
    cpu_history: HistoryHandle,
    memory_usage: GaugeHandle,
    thread_count: GaugeHandle,
    pid: GaugeHandle,
}

impl SystemMonitor {
    pub fn new(registry: &MetricsRegistry) -> Self {
        Self {
            cpu_usage: registry.register_gauge("system.cpu"),
            cpu_history: registry.register_history("system.cpu.history", 30), // 30 seconds history
            memory_usage: registry.register_gauge("system.memory"),
            thread_count: registry.register_gauge("system.threads"),
            pid: registry.register_gauge("system.pid"),
        }
    }

    pub fn run(self) {
        std::thread::spawn(move || {
            let mut sys = System::new_all();
            let pid_u32 = std::process::id();
            let pid = Pid::from_u32(pid_u32);
            
            // Set static PID once
            self.pid.set(pid_u32 as f64);

            loop {
                sys.refresh_processes();
                
                if let Some(process) = sys.process(pid) {
                    // System-wide CPU usage (match top/Activity Monitor behavior: 100% = 1 core)
                    // Previously divided by cores for total system %, but users expect top values.
                    let raw_cpu = process.cpu_usage() as f64;
                    // Clamp to 0.0 to handle rare underflow/glitch reporting (observed in top as large negative)
                    let cpu = raw_cpu.max(0.0);
                    
                    // Memory usage in bytes
                    let memory = process.memory() as f64;

                    // Simple thread count
                    let threads = process.tasks().map(|t| t.len()).unwrap_or(1) as f64;

                    self.cpu_usage.set(cpu as f64);
                    self.cpu_history.push(cpu as f64);
                    self.memory_usage.set(memory);
                    self.thread_count.set(threads);
                }

                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }
}
