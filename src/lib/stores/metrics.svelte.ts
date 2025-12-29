import { listen } from "@tauri-apps/api/event";
import { METRICS } from "$lib/constants";

export interface MetricsSnapshot {
    values: Record<string, number>;
    histories: Record<string, [number, number][]>;
}

export type HistoryItem = { id: number; val: number };

class MetricsStore {
    snapshot = $state<MetricsSnapshot | null>(null);

    constructor() {
        this.init();
    }

    async init() {
        // Listen for global snapshots
        await listen<MetricsSnapshot>("metrics:snapshot", (event) => {
            this.snapshot = event.payload;
        });
    }

    get(key: string): number {
        return this.snapshot?.values[key] ?? 0;
    }

    getHistory(key: string): HistoryItem[] {
        const data = this.snapshot?.histories[key];
        if (!data) return [];
        // Convert tuples to objects
        return data.map(([id, val]) => ({ id, val }));
    }

    get cpu() { return this.get("system.cpu"); }
    get cpuHistory() { return this.getHistory("system.cpu.history"); }
    get memory() { return this.get("system.memory"); }
    get threads() { return this.get("system.threads"); }
    get pid() { return this.get("system.pid"); }
}

export const metrics = new MetricsStore();
