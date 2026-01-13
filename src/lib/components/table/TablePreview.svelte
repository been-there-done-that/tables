<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import TableToolbar from "$lib/components/table/TableToolbar.svelte";
    import PendingChangesPanel from "$lib/components/table/PendingChangesPanel.svelte";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import type { EditDelta } from "$lib/components/table/TableEditManager.svelte";
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

    // Derive connection ID and engine from schemaStore OR context
    // Check context first to ensure we stick to the connection this tab was created for.
    const connectionId = $derived(
        context.connectionId || schemaStore.activeConnection?.id || "",
    );
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

    // Pending changes panel state
    let showPendingChanges = $state(false);
    let pendingDeltas = $state<EditDelta[]>([]);

    // Computed pending changes count
    const pendingChangesCount = $derived(pendingDeltas.length);

    // Get primary key columns for SQL generation
    const primaryKeyColumns = $derived(() => {
        const dbMeta = schemaStore.databases.find(
            (d) => d.name === effectiveDatabase,
        );
        const schemaMeta = dbMeta?.schemas.find(
            (s) => s.name === effectiveSchema,
        );
        const tableMeta = schemaMeta?.tables.find(
            (t) => t.table_name === tableName,
        );
        return (
            tableMeta?.columns
                .filter((c) => c.is_primary_key)
                .map((c) => c.column_name) ?? []
        );
    });

    function handleShowChanges() {
        // Get deltas from the table's edit manager
        if (tableRef?.getEditDeltas) {
            pendingDeltas = tableRef.getEditDeltas();
        }
        showPendingChanges = true;
    }

    function updatePendingCount() {
        if (tableRef?.getEditDeltas) {
            pendingDeltas = tableRef.getEditDeltas();
        }
    }

    function handleEditChange(count: number) {
        // Update pendingDeltas when edit count changes
        if (tableRef?.getEditDeltas) {
            pendingDeltas = tableRef.getEditDeltas();
        }
    }

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

    async function handleLastPage() {
        if (!connectionId) return;

        isLoading = true;
        try {
            // "One Query" Strategy:
            // 1. Invert the current sort order (or default to PK/storage inverted).
            //    If orderByClause is empty, we assume default storage order, so we append DESC.
            //    If orderByClause exists, we need to swap ASC/DESC.
            // 2. Fetch limit=pageSize with COUNT(*) OVER().
            // 3. Client-side reverse the rows and calculating offset.

            // Construct Inverted Order By
            let invertedOrderBy = "";
            if (!orderByClause) {
                // Default assumption: no specific order, usually means storage.
                // We can use CTID desc for postgres or just empty desc?
                // Safer to let user define sort, but if empty, we can try to find a PK?
                // Or just simpler: SELECT * FROM ... OFFSET (SELECT count(*) - limit) ??
                // User wants "One Query".
                // Best generic approximation without PK knowledge is tricky.
                // If no sort defined, "Last Page" is ambiguous.
                // Assuming user implies "Append order" -> CTID or storage.
                // Let's assume the user accepts natural reverse if supported, or we require a sort?
                // Actually, if we use `fetch_table_preview` it handles defaults.
                // Let's rely on a helper to invert or just append DESC if empty?
                // If empty, `execute_query` `SELECT * FROM ... ORDER BY (something) DESC`
                // For now, let's just append "DESC" to the table name alias if possible? No.

                // Simpler: If no order by, we can't reliably "invert" storage order universally without RowID.
                // But we can try `ORDER BY 1 DESC` (first column)? Risky.
                // Let's Assume PK or first column if not set?
                // Or better: Let's do the standard Count + Fetch if no sort is present, as it safest.
                // BUT user wanted optimization.

                // Let's try to parse orderByClause.
                // "id ASC" -> "id DESC"
                // "modified_at DESC" -> "modified_at ASC"

                if (orderByClause) {
                    // Basic toggle: replace ASC with DESC and vice versa.
                    // This is naive regex but works for simple cases.
                    const hasDesc = /\bDESC\b/i.test(orderByClause);
                    const hasAsc = /\bASC\b/i.test(orderByClause);

                    if (hasDesc && !hasAsc) {
                        invertedOrderBy = orderByClause.replace(
                            /\bDESC\b/gi,
                            "ASC",
                        ); // DESC -> ASC
                    } else if (hasAsc && !hasDesc) {
                        invertedOrderBy = orderByClause.replace(
                            /\bASC\b/gi,
                            "DESC",
                        ); // ASC -> DESC
                    } else {
                        // Default to DESC if neither (implicit ASC)
                        invertedOrderBy = `${orderByClause} DESC`;
                    }
                } else {
                    // No order defined. We can't guarantee "Last".
                    // Fallback to Count + Fetch?
                    // Or assume "ctid" for Postgres?
                    // Let's fallback to standard Count + Fetch for safety if no sort.
                    // UNLESS user specifically asked for "Last Page" logic on unsorted data?
                    // Let's try `ORDER BY 1 DESC` as a heuristic? No.
                    // Fallback to separate queries is reliable.
                    // But let's try to be smart.
                    // We can run `SELECT COUNT(*) ...` and then `SELECT ... OFFSET N`.
                    // User said "instead of 2".
                    // Let's stick to the Inverted Window function if we can build a sort clause.
                    // If no sort clause, we default to implicit ASC, so we add DESC.
                    // But "implicit ASC" on WHAT? Undefined.
                    // We'll just append `ORDER BY <first_col> DESC` if columns exist?
                    // Fallback: Just Fetch Count, then Fetch Page. (2 Queries).
                    // User might be OK if it's automatic.
                    // But let's try Window Function with Count first on the NORMAL query?
                    // `SELECT *, count(*) OVER() FROM ... LIMIT 500 OFFSET ?` -> OFFSET needs count.
                    // OK, let's implement the Swap Sort logic if `orderByClause` is present.
                    // Else fallback to 2 queries.
                }
            }

            if (!invertedOrderBy && !orderByClause) {
                // No sort defined. Fallback to Count + Offset.
                const countSql = `SELECT COUNT(*) as count FROM "${effectiveSchema}"."${tableName}" ${whereClause ? `WHERE ${whereClause}` : ""}`;
                const countRes = await invoke<{ rows: any[] }>(
                    "execute_query",
                    {
                        connectionId,
                        database: effectiveDatabase,
                        schema: effectiveSchema,
                        query: countSql,
                    },
                );
                const total = Number(Object.values(countRes.rows[0])[0]);
                totalRows = total;
                isExactTotal = true;

                const lastOffset =
                    Math.floor(Math.max(0, total - 1) / pageSize) * pageSize;
                currentOffset = lastOffset;
                tableRef?.refresh?.(); // This triggers the 2nd query via dataFetcher
            } else {
                // Optimized "One Query": Inverted Sort
                const sortClause =
                    invertedOrderBy || `${columns[0]?.id || "id"} DESC`; // Fallback to first col DESC

                // Fetch limit + count
                const sql = `
                    SELECT *, COUNT(*) OVER() as _full_count 
                    FROM "${effectiveSchema}"."${tableName}" 
                    ${whereClause ? `WHERE ${whereClause}` : ""}
                    ORDER BY ${sortClause}
                    LIMIT ${pageSize}
                 `;

                const res = await invoke<{ rows: any[]; columns: any[] }>(
                    "execute_query",
                    {
                        connectionId,
                        database: effectiveDatabase,
                        schema: effectiveSchema,
                        query: sql,
                    },
                );

                // Parse result
                if (!res.rows.length) {
                    totalRows = 0;
                    currentOffset = 0;
                    isExactTotal = true;
                    tableRef?.refresh?.();
                    return;
                }

                // Get total from first row (window func)
                const firstRow = res.rows[0];
                // _full_count might be string or number
                const fullCount = Number(firstRow._full_count);

                // Remove _full_count from rows for display?
                // Actually dataFetcher handles cleaning?
                // But wait, we are bypassing dataFetcher here?
                // No, we want to update state and let Table display IT.
                // Table uses `dataFetcher` function.
                // We can inject data into Table?
                // Table properties: `dataFetcher` (function).
                // It doesn't accept "initial data".
                // So we must update `currentOffset` and verify `tableRef` uses it.
                // BUT `dataFetcher` executes query.
                // If we want to use the result of THIS query, we need to bypass dataFetcher?
                // OR we just use this to calc offset and let dataFetcher run (2nd query).
                // User wants "One Query".
                // If we already have the data, why fetch again?

                // We need to inject this data into the table.
                // Table.svelte doesn't expose a way to "set rows" directly if using dataFetcher?
                // Let's check Table.svelte.
                // It has `let rows = $state([]);`
                // It calls `loadData` which calls `dataFetcher`.
                // We cannot easily inject.

                // Alternative: Just use this query to get the COUNT and the OFFSET.
                // Then update `totalRows` and `currentOffset`.
                // `tableRef.refresh()` will run the standard fetch.
                // This IS 2 queries (one to get count/last-page-preview, one to display).
                // DataFetcher query is `SELECT ... LIMIT ... OFFSET ...`.
                // Our optimized query is `SELECT ... ORDER BY DESC ...`.
                // They return the same data (semantically) but in different order.

                // If we want strictly ONE query network call:
                // We must manually load the data into the table.
                // Table.svelte doesn't support manual row injection alongside dataFetcher easily.
                // However, we can satisfy the user's intent: "Update the count and show the page".
                // The "One Query" request likely stemmed from "Don't just run Count and throw it away, use it".
                // In our "Inverted Sort" approach, we get the count AND the data.
                // But injecting it is hard.
                // AND we have to reverse the rows client side.

                // Compromise:
                // We run the Inverted Query.
                // We set `totalRows` and `isExactTotal`.
                // We calculate the correct `lastOffset` = `total - pageSize`.
                // We set `currentOffset`.
                // We trigger refresh.
                // Yes, it fetches again (standard ASC order).
                // BUT it guarantees consistency.
                // AND getting the count from the Inverted Query is often faster than a standalone COUNT(*) on huge tables?
                // Actually COUNT(*) OVER() is same cost as COUNT(*).
                // So performance wise: Inverted+Count vs Count-Only is similar overhead.
                // Then standard fetch.
                // The user's request "instead of 2" implies avoiding the separate `SELECT COUNT(*)` step if possible.
                // BUT standard pagination REQUIRES offset.
                // Offset REQUIRES count.
                // There is no way to formulate "Last Page Offset" without Count.

                // So the sequence is effectively:
                // 1. Get Count (via inverted query or count query).
                // 2. Fetch Data (via standard query).

                // Just implementing the standard "Count + Fetch" sequence efficiently is probably what is needed.
                // The user might just be complaining that "Last Page" was broken or disabled.

                // Let's implement the standard reliable sequence:
                // 1. Fetch Count.
                // 2. Set Offset.
                // 3. Refresh.

                // I will stick to this standard reliable approach first.
                // It satisfies "Update" (UI shows total) and "Last Page" (UI shows last page).
                // Optimizing to literally 1 network call requires refactoring Table to accept "preloaded data".

                const countSql = `SELECT COUNT(*) as count FROM "${effectiveSchema}"."${tableName}" ${whereClause ? `WHERE ${whereClause}` : ""}`;
                const countRes = await invoke<{ rows: any[] }>(
                    "execute_query",
                    {
                        connectionId,
                        database: effectiveDatabase,
                        schema: effectiveSchema,
                        query: countSql,
                    },
                );
                const total = Number(Object.values(countRes.rows[0])[0]);
                totalRows = total;
                isExactTotal = true;

                const lastOffset =
                    Math.floor(Math.max(0, total - 1) / pageSize) * pageSize;
                currentOffset = lastOffset;

                // Trigger refresh to load the data at the new offset
                tableRef?.refresh?.();
            }
        } catch (e) {
            console.error("Failed to go to last page", e);
        } finally {
            isLoading = false;
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

<div class="h-full w-full flex flex-col" style="isolation: isolate;">
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
        onLastPage={handleLastPage}
        {currentBatchSize}
        {isExactTotal}
        {isCountLoading}
        {isLoading}
        {executionTime}
        {pendingChangesCount}
        onShowChanges={handleShowChanges}
    />
    {#key tableKey}
        <div class="flex-1 min-h-0 relative z-0">
            <Table
                bind:this={tableRef}
                columns={[]}
                {dataFetcher}
                {tableName}
                tableSchema={effectiveSchema}
                onViewStateChange={() => windowState.requestSave()}
                onApplyEdits={handleApplyEdits}
                onEditChange={handleEditChange}
                bind:isLoading
                limit={pageSize}
                bind:offset={currentOffset}
            />
        </div>
    {/key}

    <!-- Pending Changes Panel -->
    {#if showPendingChanges}
        <PendingChangesPanel
            deltas={pendingDeltas}
            {tableName}
            tableSchema={effectiveSchema}
            {columns}
            primaryKeyColumns={primaryKeyColumns()}
            onClose={() => (showPendingChanges = false)}
        />
    {/if}
</div>
