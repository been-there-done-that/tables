<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import TableToolbar from "$lib/components/table/TableToolbar.svelte";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import { invoke } from "@tauri-apps/api/core";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { windowState } from "$lib/stores/window.svelte";

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

    // Toolbar state
    let tableRef: any = $state(null);
    let currentOffset = $state(0);
    let totalRows = $state(0);
    let pageSize = $state(500);
    let whereClause = $state("");
    let orderByClause = $state("");
    let columns = $state<Column[]>([]);
    let isLoading = $state(false);

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
                    total: number;
                }>("fetch_table_preview", {
                    connectionId: currentConnectionId,
                    database: currentDatabase,
                    schema: currentSchema,
                    tableName: currentTable,
                    offset: offset ?? 0,
                    limit: limit ?? 100,
                    whereClause: currentWhere || undefined,
                    orderByClause: currentOrderBy || undefined,
                });

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
                totalRows = result.total;
                currentOffset = offset ?? 0;

                // Add _rowId to each row for the Table component
                const rowsWithId = result.rows.map((row, idx) => ({
                    ...row,
                    _rowId: (offset ?? 0) + idx,
                }));

                return {
                    rows: rowsWithId,
                    total: result.total,
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
        // The Table component will handle this via its own pagination
        // For now, trigger a refresh
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
        onExport={handleExport}
        onShowDdl={handleShowDdl}
        onWhereChange={handleWhereChange}
        onOrderByChange={handleOrderByChange}
        {isLoading}
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
            />
        </div>
    {/key}
</div>
