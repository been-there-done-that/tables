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

    async init(connectionId?: string) {
        try {
            // Preserve running logs for this connection
            const running = this.logs.filter(l =>
                l.status === 'running' &&
                (!connectionId || l.connectionId === connectionId)
            );

            const history = await invoke<LogEntry[]>("fetch_query_logs", {
                limit: 100,
                connectionId: connectionId || null
            });

            if (history && Array.isArray(history)) {
                // Determine the set of logs to show.
                // If we switched connections, we only want logs for the new connection.
                // 'running' filter above handles this.
                // 'history' from backend is already filtered by connectionId.

                // Merge: running queries should be at the top (newest)
                this.logs = [...running, ...history];
            }
        } catch (e) {
            console.error("Failed to fetch query logs:", e);
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
