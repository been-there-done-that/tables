<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import { invoke } from "@tauri-apps/api/core";
    import { schemaStore } from "$lib/stores/schema.svelte";

    interface Props {
        tableName: string;
        schemaName?: string;
        databaseName?: string;
    }

    let { tableName, schemaName, databaseName }: Props = $props();

    // Derive connection ID from schemaStore
    const connectionId = $derived(schemaStore.activeConnection?.id || "");
    const effectiveSchema = $derived(schemaName || "public");
    const effectiveDatabase = $derived(
        databaseName || schemaStore.selectedDatabase || "main",
    );

    // Data fetcher implementation that calls our backend command
    const dataFetcher: DataFetcher = async (params) => {
        const { offset, limit } = params;

        if (!connectionId) {
            return { rows: [], total: 0, columns: [] };
        }

        try {
            const result = await invoke<{
                rows: any[];
                columns: { name: string; type: string }[];
                total: number;
            }>("fetch_table_preview", {
                connectionId,
                database: effectiveDatabase,
                schema: effectiveSchema,
                tableName,
                offset: offset ?? 0,
                limit: limit ?? 100,
            });

            // Convert backend column info to Table component format
            const columns: Column[] = result.columns.map((col, idx) => ({
                id: col.name,
                label: col.name,
                type: inferColumnType(col.type),
                sortable: true,
                filterable: true,
                pinnable: true,
                editable: false, // Read-only preview
            }));

            // Add _rowId to each row for the Table component
            const rowsWithId = result.rows.map((row, idx) => ({
                ...row,
                _rowId: (offset ?? 0) + idx,
            }));

            return {
                rows: rowsWithId,
                total: result.total,
                columns,
            };
        } catch (error) {
            console.error("[TablePreview] Failed to fetch data:", error);
            return { rows: [], total: 0, columns: [] };
        }
    };

    // Infer Table component column type from PostgreSQL/SQLite type strings
    function inferColumnType(
        pgType: string,
    ): import("$lib/components/table/types").ColumnType {
        const t = pgType.toLowerCase();

        if (t.includes("int") || t.includes("serial")) return "int";
        if (
            t.includes("float") ||
            t.includes("double") ||
            t.includes("numeric") ||
            t.includes("decimal") ||
            t.includes("real")
        )
            return "float";
        if (t.includes("bool")) return "boolean";
        if (t.includes("json")) return "json";
        if (t.includes("timestamp") || t.includes("datetime"))
            return "datetime";
        if (t.includes("date")) return "date";
        if (t.includes("time")) return "time";
        if (t.includes("uuid")) return "text"; // UUID rendered as text

        return "text";
    }
</script>

<div class="h-full w-full flex flex-col">
    <div class="flex-none px-4 py-2 border-b border-border bg-muted/30">
        <h3 class="text-sm font-medium">
            {effectiveSchema}.{tableName}
        </h3>
        <p class="text-xs text-muted-foreground">
            {effectiveDatabase} • Read-only preview
        </p>
    </div>

    <div class="flex-1 min-h-0">
        <Table
            columns={[]}
            {dataFetcher}
            {tableName}
            tableSchema={effectiveSchema}
        />
    </div>
</div>
