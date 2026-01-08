import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { settingsStore } from "$lib/stores/settings.svelte";

export interface LogEntry {
    id?: number;
    timestamp: number;
    connectionId: string;
    database: string;
    query: string;
    durationMs: number;
    status: "success" | "error";
    error?: string;
    rows?: number;
}

class LogsStore {
    logs = $state<LogEntry[]>([]);

    get isOpen() {
        return settingsStore.logsPanelVisible;
    }

    set isOpen(v: boolean) {
        settingsStore.logsPanelVisible = v;
    }

    constructor() {
        // Listen for log events
        listen<LogEntry>("query-log", (event) => {
            this.addLog(event.payload);
        });

        // Initial load
        this.init();
    }

    async init() {
        try {
            const history = await invoke<LogEntry[]>("fetch_query_logs", { limit: 100 });
            if (history && Array.isArray(history)) {
                // Backend fetched DESC (newest first) then reversed to ASC (oldest first).
                // So we can assign directly.
                this.logs = history;
            }
        } catch (e) {
            console.error("Failed to fetch query logs:", e);
        }
    }

    addLog(entry: LogEntry) {
        this.logs.push(entry);
        // Keep memory usage in check
        if (this.logs.length > 500) {
            this.logs.shift();
        }
    }

    toggle() {
        this.isOpen = !this.isOpen;
    }
}

export const logsStore = new LogsStore();
