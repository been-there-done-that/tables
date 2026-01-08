import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { settingsStore } from "$lib/stores/settings.svelte";

export interface LogEntry {
    id?: number;
    timestamp: number;
    correlationId?: string;
    connectionId: string;
    database: string;
    query: string;
    durationMs?: number;
    status: "success" | "error" | "running";
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
        // Listen for query completion
        listen<LogEntry>("query-log", (event) => {
            this.handleLogEvent(event.payload);
        });

        // Listen for query start
        listen<LogEntry>("query-started", (event) => {
            this.addLog(event.payload);
        });

        // Initial load
        this.init();
    }

    async init() {
        try {
            const history = await invoke<LogEntry[]>("fetch_query_logs", { limit: 100 });
            if (history && Array.isArray(history)) {
                this.logs = history;
            }
        } catch (e) {
            console.error("Failed to fetch query logs:", e);
        }
    }

    handleLogEvent(entry: LogEntry) {
        // Check if we have a running entry with this correlationId
        const existingIndex = this.logs.findIndex(l => l.correlationId === entry.correlationId && l.status === "running");
        if (existingIndex !== -1) {
            // Update in place
            this.logs[existingIndex] = { ...entry };
        } else {
            // New entry (or missed start event)
            this.addLog(entry);
        }
    }

    addLog(entry: LogEntry) {
        this.logs.push(entry);
        if (this.logs.length > 500) {
            this.logs.shift();
        }
    }

    toggle() {
        this.isOpen = !this.isOpen;
    }
}

export const logsStore = new LogsStore();
