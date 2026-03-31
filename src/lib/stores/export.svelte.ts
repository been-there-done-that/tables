// src/lib/stores/export.svelte.ts
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export type ExportStatus =
    | "connecting" | "executing" | "streaming"
    | "done" | "error" | "cancelled";

export type ExportFormat =
    | "csv" | "tsv" | "json" | "jsonl" | "sql_insert" | "sql_script";

export const EXPORT_FORMAT_LABELS: Record<ExportFormat, string> = {
    csv: "CSV",
    tsv: "TSV",
    json: "JSON",
    jsonl: "JSONL",
    sql_insert: "SQL INSERT",
    sql_script: "SQL Script",
};

export const EXPORT_FORMAT_EXT: Record<ExportFormat, string> = {
    csv: "csv", tsv: "tsv", json: "json",
    jsonl: "jsonl", sql_insert: "sql", sql_script: "sql",
};

export interface ExportEntry {
    exportId: string;
    format: ExportFormat;
    query: string;
    filePath: string;
    status: ExportStatus;
    rowsWritten: number;
    bytesWritten: number;
    elapsedMs: number;
    error?: string;
    startedAt: number; // Date.now()
}

class ExportStore {
    exports = $state<ExportEntry[]>([]);

    constructor() {
        listen<any>("export-progress", (event) => {
            const p = event.payload;
            this.updateExport(p.exportId, {
                status: p.status,
                rowsWritten: p.rowsWritten,
                bytesWritten: p.bytesWritten,
                elapsedMs: p.elapsedMs,
                filePath: p.filePath,
                error: p.error ?? undefined,
            });
        });
    }

    private updateExport(exportId: string, updates: Partial<ExportEntry>) {
        const idx = this.exports.findIndex((e) => e.exportId === exportId);
        if (idx === -1) return;
        this.exports[idx] = { ...this.exports[idx], ...updates };
    }

    async startExport(params: {
        connectionId: string;
        sessionId: string;
        database?: string;
        query: string;
        format: ExportFormat;
        filePath: string;
        tableName?: string;
    }): Promise<string> {
        const exportId = await invoke<string>("start_export", {
            connectionId: params.connectionId,
            database: params.database ?? null,
            query: params.query,
            format: params.format,
            filePath: params.filePath,
            tableName: params.tableName ?? null,
        });

        this.exports.push({
            exportId,
            format: params.format,
            query: params.query,
            filePath: params.filePath,
            status: "connecting",
            rowsWritten: 0,
            bytesWritten: 0,
            elapsedMs: 0,
            startedAt: Date.now(),
        });

        return exportId;
    }

    async cancelExport(exportId: string) {
        await invoke("cancel_export", { exportId });
    }

    dismissExport(exportId: string) {
        this.exports = this.exports.filter((e) => e.exportId !== exportId);
    }

    get activeExports() {
        return this.exports.filter((e) =>
            e.status === "connecting" || e.status === "executing" || e.status === "streaming"
        );
    }

    get hasActive() {
        return this.activeExports.length > 0;
    }
}

export const exportStore = new ExportStore();
