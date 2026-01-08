<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import { invoke } from "@tauri-apps/api/core";
    import { schemaStore } from "$lib/stores/schema.svelte";

    interface Props {
        context: {
            tableName: string;
            schemaName?: string;
            databaseName?: string;
            [key: string]: any;
        };
    }

    let { context }: Props = $props();

    // Derive values from context
    const tableName = $derived(context.tableName);
    const schemaName = $derived(context.schemaName);
    const databaseName = $derived(context.databaseName);

    // Derive connection ID from schemaStore
    const connectionId = $derived(schemaStore.activeConnection?.id || "");
    const effectiveSchema = $derived(schemaName || "public");
    const effectiveDatabase = $derived(
        databaseName || schemaStore.selectedDatabase || "main",
    );

    // Unique key for forcing Table remount when table changes
    const tableKey = $derived(
        `${connectionId}:${effectiveDatabase}:${effectiveSchema}:${tableName}`,
    );

    // Create a reactive dataFetcher that uses current prop values
    // This function is recreated when dependencies change
    const dataFetcher: DataFetcher = $derived.by(() => {
        // Capture current values in closure
        const currentConnectionId = connectionId;
        const currentDatabase = effectiveDatabase;
        const currentSchema = effectiveSchema;
        const currentTable = tableName;

        return async (params) => {
            const { offset, limit } = params;

            if (!currentConnectionId) {
                return { rows: [], total: 0, columns: [] };
            }

            try {
                const result = await invoke<{
                    rows: any[];
                    columns: { name: string; type: string }[];
                    total: number;
                }>("fetch_table_preview", {
                    connectionId: currentConnectionId,
                    database: currentDatabase,
                    schema: currentSchema,
                    tableName: currentTable,
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
                    editable: true,
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
    });

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
        if (t.includes("uuid")) return "text";

        return "text";
    }
</script>

<div class="h-full w-full">
    {#key tableKey}
        <Table
            columns={[]}
            {dataFetcher}
            {tableName}
            tableSchema={effectiveSchema}
            viewState={context}
        />
    {/key}
</div>
