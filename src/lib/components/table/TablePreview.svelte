<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import TableToolbar from "$lib/components/table/TableToolbar.svelte";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import { invoke } from "@tauri-apps/api/core";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { getDefaultDatabase, getDefaultSchema } from "$lib/engine-config";

    interface Props {
        context: {
            tableName: string;
            schemaName?: string;
            databaseName?: string;
            [key: string]: any;
        };
    }

    let { context = $bindable() }: Props = $props();

    // Derive values from context
    const tableName = $derived(context.tableName);
    const schemaName = $derived(context.schemaName);
    const databaseName = $derived(context.databaseName);

    // Derive connection ID and engine from schemaStore
    const connectionId = $derived(schemaStore.activeConnection?.id || "");
    const engine = $derived(schemaStore.activeConnection?.engine);

    // Use centralized engine config for defaults (supports PostgreSQL, SQLite, MySQL, SQL Server, etc.)
    const effectiveSchema = $derived(schemaName || getDefaultSchema(engine));
    const effectiveDatabase = $derived(
        databaseName ||
            schemaStore.selectedDatabase ||
            getDefaultDatabase(engine),
    );

    // Unique key for forcing Table remount when table changes
    const tableKey = $derived(
        `${connectionId}:${effectiveDatabase}:${effectiveSchema}:${tableName}`,
    );

    // Toolbar state
    let tableRef: any = $state(null);
    let currentOffset = $state(0);
    // currentBatchSize: number of rows returned in the last fetch
    let currentBatchSize = $state(0);
    let totalRows = $state(0);
    // isExactTotal: true if we have authoritative total from backend, false if we are guessing/accumulating
    let isExactTotal = $state(false);

    let pageSize = $state(500);
    let whereClause = $state("");
    let orderByClause = $state("");
    let columns = $state<Column[]>([]);
    let isLoading = $state(false);
    let executionTime = $state<number | undefined>(undefined);

    // Create a reactive dataFetcher that uses current prop values
    // This function is recreated when dependencies change
    const dataFetcher: DataFetcher = $derived.by(() => {
        // Capture current values in closure
        const currentConnectionId = connectionId;
        const currentDatabase = effectiveDatabase;
        const currentSchema = effectiveSchema;
        const currentTable = tableName;
        const currentWhere = whereClause;
        const currentOrderBy = orderByClause;

        return async (params) => {
            const { offset, limit } = params;

            if (!currentConnectionId) {
                return { rows: [], total: 0, columns: [] };
            }

            try {
                const result = await invoke<{
                    rows: any[];
                    columns: { name: string; type: string }[];
                    total: number | null;
                    duration_ms: number;
                }>("fetch_table_preview", {
                    connectionId: currentConnectionId,
                    database: currentDatabase,
                    schema: currentSchema,
                    tableName: currentTable,
                    offset: offset ?? 0,
                    limit: limit ?? 100,
                    whereClause: currentWhere || undefined,
                    orderByClause: currentOrderBy || undefined,
                    fetchTotal: false, // Don't fetch total by default (expensive)
                });

                // Use backend timing
                executionTime = result.duration_ms;

                // Convert backend column info to Table component format
                const fetchedColumns: Column[] = result.columns.map(
                    (col, idx) => ({
                        id: col.name,
                        label: col.name,
                        type: inferColumnType(col.type),
                        sortable: true,
                        filterable: true,
                        pinnable: true,
                        editable: true,
                    }),
                );

                // Update toolbar state
                columns = fetchedColumns;

                // Logic for total rows and batch size
                currentBatchSize = result.rows.length;

                if (result.total !== null) {
                    isExactTotal = true;
                    totalRows = result.total;
                } else {
                    // We don't have an exact total.
                    // If we found rows, we know at least (offset + count) exist.
                    isExactTotal = false;
                    const loadedCount = (offset ?? 0) + result.rows.length;
                    totalRows = loadedCount;
                }

                currentOffset = offset ?? 0;

                // Add _rowId to each row for the Table component
                const rowsWithId = result.rows.map((row, idx) => ({
                    ...row,
                    _rowId: (offset ?? 0) + idx,
                }));

                return {
                    rows: rowsWithId,
                    total: result.total ?? totalRows, // Pass best guess or exact
                    columns: fetchedColumns,
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

    import type { RowEdit, EditResult } from "$lib/components/table/types";

    async function handleApplyEdits(edits: RowEdit[]): Promise<EditResult> {
        if (!connectionId)
            return { success: false, conflicts: ["No active connection"] };

        const dbName = effectiveDatabase;
        const schema = effectiveSchema;
        const table = tableName;

        // Find table metadata to get Primary Keys and Column Types
        // We look in schemaStore.databases
        const dbMeta = schemaStore.databases.find((d) => d.name === dbName);
        const schemaMeta = dbMeta?.schemas.find((s) => s.name === schema);
        const tableMeta = schemaMeta?.tables.find(
            (t) => t.table_name === table,
        );

        if (!tableMeta) {
            return {
                success: false,
                conflicts: ["Table metadata not found. Please refresh schema."],
            };
        }

        const pkCols = tableMeta.columns
            .filter((c) => c.is_primary_key)
            .sort((a, b) => a.ordinal_position - b.ordinal_position);

        const errs: string[] = [];

        for (const edit of edits) {
            const { originalRow, changes } = edit;

            // 1. Build SET clause
            const setClauses: string[] = [];
            for (const [colName, newValue] of Object.entries(changes)) {
                // Find column meta for type-aware formatting
                const colMeta = tableMeta.columns.find(
                    (c) => c.column_name === colName,
                );
                const formattedVal = formatSqlValue(
                    newValue,
                    colMeta?.raw_type,
                );
                setClauses.push(`"${colName}" = ${formattedVal}`);
            }

            if (setClauses.length === 0) continue;

            // 2. Build WHERE clause
            const whereClauses: string[] = [];

            if (pkCols.length > 0) {
                // Use Primary Keys
                for (const pk of pkCols) {
                    const val = originalRow[pk.column_name];
                    const formattedVal = formatSqlValue(val, pk.raw_type);
                    whereClauses.push(`"${pk.column_name}" = ${formattedVal}`);
                }
            } else {
                // Fallback: Use all columns (optimistic concurrency match)
                // We shouldn't use _rowId as it's synthetic
                for (const [key, val] of Object.entries(originalRow)) {
                    if (key === "_rowId") continue;
                    // Skip if value is complex object/array as strict equality might fail in SQL without casting?
                    // For now, try best effort.
                    const colMeta = tableMeta.columns.find(
                        (c) => c.column_name === key,
                    );
                    if (val === null) {
                        whereClauses.push(`"${key}" IS NULL`);
                    } else {
                        const formattedVal = formatSqlValue(
                            val,
                            colMeta?.raw_type,
                        );
                        whereClauses.push(`"${key}" = ${formattedVal}`);
                    }
                }
            }

            if (whereClauses.length === 0) {
                errs.push(`Cannot determine identity for row ${edit.rowId}`);
                continue;
            }

            const sql = `UPDATE "${schema}"."${table}" SET ${setClauses.join(", ")} WHERE ${whereClauses.join(" AND ")}`;
            console.log("[TablePreview] Executing update:", sql);

            try {
                const res = await invoke("execute_query", {
                    connectionId,
                    database: dbName,
                    schema,
                    query: sql,
                });
                // Check res? execute_query returns QueryResult.
                // If it didn't throw, it's likely success.
            } catch (e: any) {
                console.error("Update failed", e);
                errs.push(String(e));
            }
        }

        if (errs.length > 0) {
            return { success: false, conflicts: errs };
        }

        return { success: true };
    }

    function formatSqlValue(val: any, rawType?: string): string {
        if (val === null || val === undefined) return "NULL";
        if (typeof val === "boolean") return val ? "TRUE" : "FALSE";
        if (typeof val === "number") return String(val);

        // Handle dates/timestamps if they are objects?
        // Usually they come as strings from JSON.

        // Escape single quotes
        const strVal = String(val).replace(/'/g, "''");
        return `'${strVal}'`;
    }

    // Toolbar handlers
    function handleExecute() {
        tableRef?.refresh?.();
    }

    function handleRefresh() {
        tableRef?.refresh?.();
    }

    function handlePageChange(newOffset: number) {
        currentOffset = newOffset;
        // Offset is bound to Table component, so no need to manually refresh if Table reacts to it?
        // Actually, Table relies on dataFetcher reference changing or explicit refresh call.
        // Changing 'offset' prop might trigger updates if Table uses it purely for display or next fetch.
        // But Table calls loadData().
        // Let's ensure Table refreshes.
        tableRef?.refresh?.();
    }

    function handlePageSizeChange(newSize: number) {
        pageSize = newSize;
        currentOffset = 0; // Reset to first page
        tableRef?.refresh?.();
    }

    function handleExport(format: "csv" | "tsv" | "json" | "sql") {
        // TODO: Implement export logic - copy formatted data to clipboard
        console.log("[TablePreview] Export requested:", format);
    }

    function handleShowDdl() {
        // TODO: Implement DDL display
        console.log("[TablePreview] Show DDL requested");
    }

    function handleWhereChange(value: string) {
        whereClause = value;
    }

    function handleOrderByChange(value: string) {
        orderByClause = value;
    }

    let isCountLoading = $state(false);

    async function handleCountUpdate() {
        if (!connectionId) return;

        isCountLoading = true;
        try {
            const sql = `SELECT COUNT(*) as count FROM "${effectiveSchema}"."${tableName}" ${whereClause ? `WHERE ${whereClause}` : ""}`;
            const res = await invoke<{ rows: any[] }>("execute_query", {
                connectionId,
                database: effectiveDatabase,
                schema: effectiveSchema,
                query: sql,
            });

            if (res.rows && res.rows.length > 0) {
                const count = Number(Object.values(res.rows[0])[0]);
                totalRows = count;
                isExactTotal = true;
            }
        } catch (e) {
            console.error("Failed to update count", e);
        } finally {
            isCountLoading = false;
        }
    }

    async function handleCancel() {
        if (!connectionId) return;

        try {
            const cancelled = await invoke<boolean>("cancel_query", {
                connectionId,
            });
            if (cancelled) {
                console.log("[TablePreview] Query cancelled");
            }
        } catch (error) {
            console.error("[TablePreview] Failed to cancel query:", error);
        }
    }
</script>

<div class="h-full w-full flex flex-col">
    <TableToolbar
        bind:tableRef
        {columns}
        {currentOffset}
        {totalRows}
        {pageSize}
        {whereClause}
        {orderByClause}
        onExecute={handleExecute}
        onRefresh={handleRefresh}
        onPageChange={handlePageChange}
        onPageSizeChange={handlePageSizeChange}
        onExport={handleExport}
        onShowDdl={handleShowDdl}
        onWhereChange={handleWhereChange}
        onOrderByChange={handleOrderByChange}
        onCancel={handleCancel}
        onCountUpdate={handleCountUpdate}
        {currentBatchSize}
        {isExactTotal}
        {isCountLoading}
        {isLoading}
        {executionTime}
    />
    {#key tableKey}
        <div class="flex-1 min-h-0">
            <Table
                bind:this={tableRef}
                columns={[]}
                {dataFetcher}
                {tableName}
                tableSchema={effectiveSchema}
                onViewStateChange={() => windowState.requestSave()}
                onApplyEdits={handleApplyEdits}
                bind:isLoading
                limit={pageSize}
                bind:offset={currentOffset}
            />
        </div>
    {/key}
</div>
