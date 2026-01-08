import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { settingsStore } from "$lib/stores/settings.svelte";
import { schemaStore } from "$lib/stores/schema.svelte";

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
    lastRefreshedConnectionId: string | null = null;

    get isOpen() {
        return settingsStore.logsPanelVisible;
    }

    set isOpen(v: boolean) {
        settingsStore.logsPanelVisible = v;
    }

    constructor() {
        // Listen for log events
        listen<LogEntry>("query-log", (event) => {
            if (this.shouldIncludeLog(event.payload)) {
                this.handleLogEvent(event.payload);
            }
        });

        // Listen for query start events
        listen<LogEntry>("query-started", (event) => {
            if (this.shouldIncludeLog(event.payload)) {
                this.addLog(event.payload);
            }
        });
    }

    private lastConnectionId: string | null = null;

    async init(connectionId?: string) {
        // Prevent duplicate calls for the same connection ID
        // Note: use undefined for "no connection/global" logic if desired, matching the argument
        const targetId = connectionId || null;
        if (this.lastRefreshedConnectionId === targetId) {
            return;
        }

        try {
            this.lastRefreshedConnectionId = targetId;

            // Simplified: Just fetch fresh history.
            // User requested "no need to do the safe handling... old data be old data"
            this.logs = [];

            const history = await invoke<LogEntry[]>("fetch_query_logs", {
                limit: 100,
                connectionId: targetId
            });

            if (history && Array.isArray(history)) {
                this.logs = history;
            }
        } catch (e) {
            console.error("Failed to fetch query logs:", e);
            // Reset on error so we can try again potentially?
            this.lastRefreshedConnectionId = null;
        }
    }

    shouldIncludeLog(entry: LogEntry): boolean {
        // Access schemaStore directly to check active connection
        const activeId = schemaStore.activeConnection?.id;
        if (!activeId) return true;
        return entry.connectionId === activeId;
    }

    handleLogEvent(entry: LogEntry) {
        // Check if we have a running entry with this correlationId
        const existingIndex = this.logs.findIndex(l => l.correlationId === entry.correlationId && l.status === "running");
        if (existingIndex !== -1) {
            // Update in place
            this.logs[existingIndex] = { ...entry };
        } else {
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
