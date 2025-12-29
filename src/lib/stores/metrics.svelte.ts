import { listen } from "@tauri-apps/api/event";
import { METRICS } from "$lib/constants";

export interface MetricsSnapshot {
    values: Record<string, number>;
}

class MetricsStore {
    snapshot = $state<MetricsSnapshot | null>(null);
    cpuHistory = $state<number[]>([]);

    constructor() {
        this.init();
    }

    async init() {
        // Listen for global snapshots
        await listen<MetricsSnapshot>("metrics:snapshot", (event) => {
            this.snapshot = event.payload;

            // Maintain CPU history
            const cpu = this.get("system.cpu"); // Normalized 0-100 usually, or 0-1? Sysinfo gives 0-100 per core usually? 
            // sysinfo cpu_usage() is 0-100. My backend logic: process.cpu_usage() / cores.
            // If process.cpu_usage() is total across all cores (e.g. 800%), dividing by cores gives 0-100%. 
            // So it should be 0-100.

            const next = [...this.cpuHistory, cpu];
            if (next.length > METRICS.HISTORY_SIZE) next.shift();
            this.cpuHistory = next;
        });
    }

    get(key: string): number {
        return this.snapshot?.values[key] ?? 0;
    }

    get cpu() { return this.get("system.cpu"); }
    get threads() { return this.get("system.threads"); }
    get pid() { return this.get("system.pid"); }
}

export const metrics = new MetricsStore();
