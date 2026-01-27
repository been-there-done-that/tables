<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import { cn } from "$lib/utils";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import * as monaco from "monaco-editor";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconSchema from "@tabler/icons-svelte/icons/table";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";

    import { settingsStore } from "$lib/stores/settings.svelte";
    import { windowState } from "$lib/stores/window.svelte";

    import { invoke } from "@tauri-apps/api/core";
    import {
        saveEditorSession,
        loadEditorSession,
        createDebouncedSave,
    } from "$lib/services/editor-persistence";
    import {
        enableQueryHeaders,
        type QueryHeaderController,
    } from "$lib/monaco/query-headers";
    import QueryEditorToolbar from "./editor/QueryEditorToolbar.svelte";
    import ResizableSplitPane from "./ResizableSplitPane.svelte";

    import Table from "$lib/components/table/Table.svelte";
    import TableToolbar from "$lib/components/table/TableToolbar.svelte";
    import { normalizeColumnType } from "$lib/components/table/columnUtils";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import type { EditDelta } from "$lib/components/table/TableEditManager.svelte";
    import { pendingChangesStore } from "$lib/stores/pendingChanges.svelte";
    import { tick } from "svelte";

    let { id = "playground", context = $bindable({}) } = $props<{
        id?: string;
        context?: any;
    }>();

    let editorContainer: HTMLElement;
    let editorHandle = $state<EditorHandle | null>(null);
    let logs: string[] = $state([]);
    let isLoadingSession = $state(true);
    let isRunning = $state(false);
    let executionTime = $state(0);

    let headerController: QueryHeaderController | null = null;

    // Result Table State
    let resultRows = $state<any[]>([]);
    let resultColumns = $state<Column[]>([]);
    let resultTotal = $state(0);
    let showResultTable = $state(false);
    let tableRef: any = $state(null);
    let resultLoading = $state(false);
    let resultSplitRatio = $state(0.6);

    // Toolbar & Pagination State
    let pageSize = $state(100);
    let currentOffset = $state(0);
    let currentBatchSize = $state(0);
    let isExactTotal = $state(true);
    let isCountLoading = $state(false);
    let whereClause = $state("");
    let orderByClause = $state("");
    let lastExecutedQuery = $state("");
    let detectedTable = $state<{ schema: string; table: string } | null>(null);

    // Editing state
    let pendingDeltas = $state<EditDelta[]>([]);
    let isSaving = $state(false);
    const pendingChangesCount = $derived(pendingDeltas.length);

    // Derived DataFetcher for Table component
    const resultDataFetcher: DataFetcher = $derived(async (params) => {
        const { offset, limit, sort, filters } = params;

        let processed = [...resultRows];

        // 1. Client-side Sort
        if (sort && sort.length > 0) {
            processed.sort((a, b) => {
                for (const s of sort) {
                    const colId = s.columnId;
                    const valA = a[colId];
                    const valB = b[colId];
                    if (valA === valB) continue;

                    // Handle nulls
                    if (valA === null || valA === undefined)
                        return s.direction === "asc" ? -1 : 1;
                    if (valB === null || valB === undefined)
                        return s.direction === "asc" ? 1 : -1;

                    if (typeof valA === "number" && typeof valB === "number") {
                        const diff = valA - valB;
                        return s.direction === "asc" ? diff : -diff;
                    }

                    const strA = String(valA).toLowerCase();
                    const strB = String(valB).toLowerCase();
                    if (strA < strB) return s.direction === "asc" ? -1 : 1;
                    if (strA > strB) return s.direction === "asc" ? 1 : -1;
                }
                return 0;
            });
        }

        // 2. Client-side Filter
        if (filters && Object.keys(filters).length > 0) {
            // Note: Table component handles filtering efficiently too,
            // but if Table calls fetcher with filters, we must apply them here for correctness if we return a subset.
            // Since we're client-side, we can just return the filtered page.

            // Simplified filter implemention matching Table.svelte's logic
            Object.entries(filters).forEach(([columnId, filterValue]) => {
                if (!filterValue) return;

                if (filterValue.type === "in" && filterValue.values) {
                    processed = processed.filter((row) => {
                        const cellValue = String(row[columnId]);
                        return filterValue.values.includes(cellValue);
                    });
                } else if (filterValue.type === "equals") {
                    processed = processed.filter(
                        (row) => row[columnId] === filterValue.value,
                    );
                } else if (filterValue.type === "contains") {
                    processed = processed.filter((row) =>
                        String(row[columnId])
                            .toLowerCase()
                            .includes(String(filterValue.value).toLowerCase()),
                    );
                }
            });
        }

        const filteredTotal = processed.length;
        const page = processed.slice(offset, offset + (limit || 1000));

        currentBatchSize = page.length;
        resultTotal = filteredTotal;

        // Add rowId if missing
        const rowsWithId = page.map((r, i) => ({
            ...r,
            _rowId: (offset || 0) + i,
        }));

        return {
            rows: rowsWithId,
            total: filteredTotal,
            columns: resultColumns,
            columnStats: undefined, // Let table compute stats
        };
    });

    // Debounced save for editor content
    const debouncedSave = createDebouncedSave(2000);

    // Stable context and URI derived from ID
    const stableContextId = $derived(`sql-playground-${id}`);
    const stableModelUri = $derived(
        context?.modelUri || `file:///playground-${id}.sql`,
    );

    // Toolbar state
    // Use schemaStore.activeSchema instead of local state
    // We synchronize it with context if provided
    $effect(() => {
        if (context?.schemaName) {
            schemaStore.activeSchema = context.schemaName;
        }
    });

    // Reactive font settings
    $effect(() => {
        if (editorHandle?.editor) {
            const family = settingsStore.editorFontFamily.includes(" ")
                ? `"${settingsStore.editorFontFamily}"`
                : settingsStore.editorFontFamily;

            editorHandle.editor.updateOptions({
                fontFamily: family,
                fontSize: settingsStore.editorFontSize,
            });
        }
    });

    const currentSchemas = $derived.by(() => {
        const dbName = schemaStore.selectedDatabase;
        if (!dbName) return [];
        const db = schemaStore.databases.find((d) => d.name === dbName);
        return db?.schemas || [];
    });

    function log(msg: string) {
        logs = [
            `${new Date().toISOString().split("T")[1].substring(0, 12)} - ${msg}`,
            ...logs,
        ];
    }

    async function executeCurrent() {
        if (!editorHandle) return;
        const editor = editorHandle.editor;
        const model = editor.getModel();
        if (!model) return;

        let query = "";
        let source = "";
        let startLine: number | undefined;

        // 1. Check for manual selection first
        const selection = editor.getSelection();
        if (selection && !selection.isEmpty()) {
            query = model.getValueInRange(selection);
            source = "manual selection";
            startLine = selection.startLineNumber;
        } else {
            // 2. Fallback to auto-highlighted statement
            const decorations = editor.getDecorationsInRange(
                new monaco.Range(1, 1, model.getLineCount(), 1),
            );
            const highlight = decorations?.find(
                (d) => d.options.className === "current-query-highlight-bg",
            );

            if (highlight) {
                // Ensure we get the full text of the last line
                const fullRange = new monaco.Range(
                    highlight.range.startLineNumber,
                    1,
                    highlight.range.endLineNumber,
                    model.getLineMaxColumn(highlight.range.endLineNumber),
                );
                query = model.getValueInRange(fullRange);
                source = "auto-highlighted statement";
                startLine = highlight.range.startLineNumber;
            } else {
                // 3. Fallback to full text
                query = editor.getValue();
                source = "full text";
            }
        }

        if (query.trim()) {
            console.log(`[Execute] Running query from ${source}:`, query);
            log(
                `Executing (${source}) in ${schemaStore.selectedDatabase}.${schemaStore.activeSchema}:\n${query}`,
            );

            if (!schemaStore.activeConnection) {
                log("No active connection selected.");
                return;
            }

            if (startLine && headerController) {
                headerController.updateStatus(startLine, query, {
                    state: "running",
                });
            }

            const startTime = performance.now();

            try {
                isRunning = true;
                resultLoading = true;
                const result = await invoke<any>("execute_query", {
                    connectionId: schemaStore.activeConnection.id,
                    sessionId: id,
                    database: schemaStore.selectedDatabase,
                    schema: schemaStore.activeSchema || "public",
                    query: query,
                    component: "editor",
                });
                isRunning = false;
                executionTime =
                    result.duration_ms ?? performance.now() - startTime;

                const duration = executionTime;

                if (startLine && headerController) {
                    headerController.updateStatus(startLine, query, {
                        state: "success",
                        duration,
                    });
                }

                console.log("Query Result:", result);

                // Populate Result Table
                if (result.rows && result.columns) {
                    resultRows = result.rows;
                    resultColumns = result.columns.map((c: any) => ({
                        id: c.name,
                        label: c.name,
                        type: normalizeColumnType(c.type),
                        rawType: c.type,
                        sortable: true,
                        filterable: true,
                        pinnable: true,
                        editable: true,
                        width: undefined, // Auto-size
                    }));
                    resultTotal = result.rows.length;
                    showResultTable = true;
                    // Force refresh table logic if needed, usually dataFetcher updates handle it
                    if (tableRef) tableRef.refresh();
                } else if (
                    Array.isArray(result) &&
                    result.length > 0 &&
                    result[0].rows
                ) {
                    // Handle multi-result (batch)? Just take last for now
                    const last = result[result.length - 1];
                    if (last.rows && last.columns) {
                        resultRows = last.rows;
                        resultColumns = last.columns.map((c: any) => ({
                            id: c.name,
                            label: c.name,
                            type: normalizeColumnType(c.type),
                            rawType: c.type,
                            sortable: true,
                            filterable: true,
                            pinnable: true,
                            editable: true,
                        }));
                        resultTotal = last.rows.length;
                        showResultTable = true;
                        if (tableRef) tableRef.refresh();
                    }
                }

                log("Query completed successfully.");
            } catch (e) {
                if (startLine && headerController) {
                    headerController.updateStatus(startLine, query, {
                        state: "error",
                        errorMessage: String(e),
                    });
                }
                console.error("Query execution failed:", e);
                log(`Query failed: ${e}`);
            } finally {
                isRunning = false;
                resultLoading = false;
            }
        } else {
            log("No query to execute");
        }
    }

    // Execute a specific query text (used by inline run buttons)
    // Accept line ranges to track status
    async function executeQueryText(
        queryText: string,
        startLine?: number,
        endLine?: number,
    ) {
        if (!queryText.trim()) {
            log("No query to execute");
            return;
        }

        log(
            `Executing (inline button) in ${schemaStore.selectedDatabase}.${schemaStore.activeSchema}:\n${queryText}`,
        );

        if (!schemaStore.activeConnection) {
            log("No active connection selected.");
            return;
        }

        // Mark as running
        if (startLine && endLine && headerController) {
            headerController.updateStatus(startLine, queryText, {
                state: "running",
            });
        }

        const startTime = performance.now();

        try {
            isRunning = true;
            resultLoading = true;
            const result = await invoke<any>("execute_query", {
                connectionId: schemaStore.activeConnection.id,
                sessionId: id,
                database: schemaStore.selectedDatabase,
                schema: schemaStore.activeSchema || "public",
                query: queryText,
                component: "editor",
            });
            isRunning = false;

            // Use backend duration if available (more accurate), else fallback to frontend measure
            executionTime = result.duration_ms ?? performance.now() - startTime;
            const duration = executionTime;

            // Mark success
            if (startLine && endLine && headerController) {
                headerController.updateStatus(startLine, queryText, {
                    state: "success",
                    duration,
                });
            }

            console.log("Query Result:", result);

            // Populate Result Table
            if (result.rows && result.columns) {
                resultRows = result.rows;
                resultColumns = result.columns.map((c: any) => ({
                    id: c.name,
                    label: c.name,
                    type: normalizeColumnType(c.type),
                    rawType: c.type,
                    sortable: true,
                    filterable: true,
                    pinnable: true,
                    editable: true,
                }));
                resultTotal = result.rows.length;
                showResultTable = true;
                if (tableRef) tableRef.refresh();
            } else if (
                Array.isArray(result) &&
                result.length > 0 &&
                result[0].rows
            ) {
                const last = result[result.length - 1];
                if (last.rows && last.columns) {
                    resultRows = last.rows;
                    resultColumns = last.columns.map((c: any) => ({
                        id: c.name,
                        label: c.name,
                        type: normalizeColumnType(c.type),
                        rawType: c.type,
                        sortable: true,
                        filterable: true,
                        pinnable: true,
                        editable: true,
                    }));
                    resultTotal = last.rows.length;
                    showResultTable = true;
                    if (tableRef) tableRef.refresh();
                }
            }

            lastExecutedQuery = queryText;
            detectedTable = detectTableFromQuery(queryText);

            log("Query completed successfully.");
        } catch (e) {
            // Mark error
            if (startLine && endLine && headerController) {
                headerController.updateStatus(startLine, queryText, {
                    state: "error",
                    errorMessage: String(e),
                });
            }

            console.error("Query execution failed:", e);
            log(`Query failed: ${e}`);
        } finally {
            isRunning = false;
            resultLoading = false;
        }
    }

    // Detect if a query is editable (simple SELECT * FROM <table>)
    function detectTableFromQuery(
        queryStr: string,
    ): { schema: string; table: string } | null {
        const match = queryStr.match(
            /^\s*SELECT\s+\*\s+FROM\s+(?:"?(\w+)"?\.)?"?(\w+)"?\s*(?:WHERE|LIMIT|ORDER|;|$)/i,
        );
        if (match) {
            const schema = match[1] || schemaStore.activeSchema || "public";
            const table = match[2];
            return { schema, table };
        }
        return null;
    }

    async function fetchPkColumns(
        schema: string,
        table: string,
    ): Promise<string[]> {
        try {
            const tableDetails = await invoke<any>("get_schema_table_details", {
                connectionId: schemaStore.activeConnection?.id,
                database: schemaStore.selectedDatabase,
                schema: schema,
                tableName: table,
            });
            if (tableDetails && tableDetails.columns) {
                return tableDetails.columns
                    .filter((c: any) => c.is_primary_key)
                    .map((c: any) => c.column_name);
            }
        } catch (e) {
            console.warn(
                "[SqlEditor] Failed to fetch table details for PK detection:",
                e,
            );
        }
        return [];
    }

    function formatSqlValue(val: any, rawType?: string): string {
        if (val === null || val === undefined) return "NULL";
        if (typeof val === "boolean") return val ? "TRUE" : "FALSE";
        if (typeof val === "number") return String(val);
        const strVal = String(val).replace(/'/g, "''");
        return `'${strVal}'`;
    }

    // Toolbar & Editing Handlers
    function handleExecute() {
        executeCurrent();
    }

    function handleRefresh() {
        executeCurrent();
    }

    function handlePageChange(newOffset: number) {
        currentOffset = newOffset;
        if (tableRef) tableRef.refresh();
    }

    function handlePageSizeChange(newSize: number) {
        pageSize = newSize;
        currentOffset = 0;
        if (tableRef) tableRef.refresh();
    }

    function handleWhereChange(value: string) {
        whereClause = value;
    }

    function handleOrderByChange(value: string) {
        orderByClause = value;
    }

    async function handleCountUpdate() {
        // For arbitrary results, we don't have a reliable way to get total count
        // unless we wrapped the query in a CTE or subquery.
        // For now, we use resultTotal from the fetched buffer.
        isCountLoading = true;
        await new Promise((r) => setTimeout(r, 200));
        isCountLoading = false;
    }

    function handleExport(format: string) {
        console.log("Export requested:", format);
        log(`Exporting results to ${format} (stub)`);
    }

    function handleShowDdl() {
        log("Show DDL not supported for arbitrary queries.");
    }

    async function handleApplyEdits(
        edits: any[],
    ): Promise<{ success: boolean; conflicts?: string[] }> {
        if (!detectedTable) {
            return {
                success: false,
                conflicts: [
                    "Editing is only supported for 'SELECT * FROM table' queries.",
                ],
            };
        }

        const deltas = tableRef?.getEditDeltas?.() ?? [];
        if (deltas.length === 0) return { success: true };

        const pkCols = await fetchPkColumns(
            detectedTable.schema,
            detectedTable.table,
        );
        if (pkCols.length === 0) {
            // Check if there is an 'id' column anyway as an anchor point per user suggestion
            if (resultColumns.find((c) => c.id.toLowerCase() === "id")) {
                pkCols.push(
                    resultColumns.find((c) => c.id.toLowerCase() === "id")!.id,
                );
            } else {
                return {
                    success: false,
                    conflicts: [
                        "No primary keys found for table. Please refresh schema or add an 'id' column.",
                    ],
                };
            }
        }

        const errors: string[] = [];
        for (const delta of deltas) {
            let sql = "";
            const { type, rowId, columnId, newValue, pkValues } = delta;

            if (type === "U") {
                const setClause = `"${columnId}" = ${formatSqlValue(newValue)}`;
                const whereClauses = pkCols.map(
                    (pk) => `"${pk}" = ${formatSqlValue(pkValues[pk])}`,
                );
                sql = `UPDATE "${detectedTable.schema}"."${detectedTable.table}" SET ${setClause} WHERE ${whereClauses.join(" AND ")};`;
            } else if (type === "D") {
                const whereClauses = pkCols.map(
                    (pk) => `"${pk}" = ${formatSqlValue(pkValues[pk])}`,
                );
                sql = `DELETE FROM "${detectedTable.schema}"."${detectedTable.table}" WHERE ${whereClauses.join(" AND ")};`;
            } else if (type === "I") {
                // Bulk insert handled as a single row for simplicity here
                // We'd need to gather all columns for a full insert if we support 'Add Row' properly
                errors.push("INSERT not yet supported in playground results.");
                continue;
            }

            try {
                log(`Applying edit: ${sql}`);
                await invoke("execute_query", {
                    connectionId: schemaStore.activeConnection?.id,
                    sessionId: id,
                    database: schemaStore.selectedDatabase,
                    schema: detectedTable.schema,
                    query: sql,
                });
            } catch (e) {
                errors.push(`Failed to apply edit for row ${rowId}: ${e}`);
            }
        }

        if (errors.length > 0) return { success: false, conflicts: errors };

        // Re-run original query to see changes
        executeCurrent();
        return { success: true };
    }

    async function handleSaveChanges(): Promise<{
        success: boolean;
        errors?: string[];
    }> {
        isSaving = true;
        const res = await handleApplyEdits([]);
        isSaving = false;
        if (res.success) {
            tableRef?.revertAll?.();
        }
        return {
            success: res.success,
            errors: res.conflicts,
        };
    }

    async function handleToolbarSave(): Promise<void> {
        await handleSaveChanges();
    }

    function handleShowChanges() {
        if (tableRef?.getEditDeltas) {
            pendingDeltas = tableRef.getEditDeltas();
        }
        tick().then(() => {
            pendingChangesStore.setContext(
                pendingDeltas,
                detectedTable?.table || "query_result",
                resultColumns,
                [], // PKs not strictly tracked in store here
                detectedTable?.schema || "",
                {
                    onRevertRow: (rid) => tableRef?.revertRow?.(rid),
                    onRevertAll: () => tableRef?.revertAll?.(),
                    onSaveChanges: handleSaveChanges,
                },
            );
            windowState.openRightPanel("pending-changes");
        });
    }

    function handleAddRow() {
        log("Manual Add Row not yet supported in playground.");
    }

    function handleEditChange() {
        if (tableRef?.getEditDeltas) {
            pendingDeltas = tableRef.getEditDeltas();
        }
    }

    async function handleCancel() {
        if (!schemaStore.activeConnection) return;
        try {
            await invoke("cancel_query", {
                connectionId: schemaStore.activeConnection.id,
            });
            log("Query cancellation requested.");
        } catch (e) {
            console.error("Cancel failed:", e);
        }
    }

    function handleExplain(raw: boolean = false) {
        log(`Executing Explain ${raw ? "(Raw)" : ""}...`);
        const editor = editorHandle?.editor;
        if (!editor) return;

        const selection = editor.getSelection();
        const model = editor.getModel();
        if (!model) return;

        let query = "";
        if (selection && !selection.isEmpty()) {
            query = model.getValueInRange(selection);
        } else {
            query = editor.getValue();
        }

        if (!query.trim()) return;

        const explainQuery = raw
            ? `EXPLAIN (FORMAT JSON) ${query}`
            : `EXPLAIN ${query}`;

        executeQueryText(explainQuery);
    }

    async function handleFormat() {
        if (!editorHandle) return;

        // Clear headers before format to prevent visual glitches
        if (headerController) {
            headerController.clearAll();
        }

        const sql = editorHandle.editor.getValue();
        try {
            const formatted = await invoke<string>("format_sql", { sql });

            // Apply formatting via edit operation to maintain undo stack
            const model = editorHandle.editor.getModel();
            if (model) {
                editorHandle.editor.executeEdits("formatter", [
                    {
                        range: model.getFullModelRange(),
                        text: formatted,
                        forceMoveMarkers: true,
                    },
                ]);
            }

            log("Code formatted (backend)");
        } catch (e) {
            log(`Formatting failed: ${e}`);
            // Fallback to monaco built-in just in case
            editorHandle.editor
                .getAction("editor.action.formatDocument")
                ?.run();
        }
    }

    function handleClear() {
        if (!editorHandle) return;

        // Clear headers
        if (headerController) {
            headerController.clearAll();
        }

        editorHandle.editor.setValue("");
        log("Editor cleared");
    }

    async function handleStop() {
        if (!schemaStore.activeConnection) return;
        log("Requesting query cancellation...");
        try {
            await invoke("cancel_query", {
                connectionId: schemaStore.activeConnection.id,
            });
            // Optimization: clear headers on stop to force fresh state
            if (headerController) {
                headerController.clearAll();
            }
        } catch (e) {
            log(`Failed to cancel query: ${e}`);
            console.error("Cancel failed:", e);
        }
    }

    function handleUndo() {
        if (!editorHandle?.editor) return;
        editorHandle.editor.trigger("toolbar", "undo", null);
        editorHandle.editor.focus();
    }

    function handleRedo() {
        if (!editorHandle?.editor) return;
        editorHandle.editor.trigger("toolbar", "redo", null);
        editorHandle.editor.focus();
    }

    useMonacoEditor(
        {
            contextId: stableContextId,
            windowId: windowState.label,
            kind: "sql",
            modelUri: stableModelUri,
            container: () => editorContainer,
            options: {
                theme: MONACO_THEME_NAME,
                minimap: { enabled: false },
                padding: { top: 4, bottom: 16, left: 16 } as any,
                lineNumbersMinChars: 3,
                lineDecorationsWidth: 8,
                glyphMargin: true,
            },
        },
        (handle) => {
            console.log("[EDITOR-DEBUG] ========== CALLBACK START ==========");
            console.log("[EDITOR-DEBUG] Editor callback received for:", {
                id,
                stableModelUri,
                stableContextId,
                editorId: handle.editorId,
                contentOnCallback: handle.editor.getValue().substring(0, 100),
            });

            editorHandle = handle;
            log("Editor initialized");

            // Add Command+Enter / Ctrl+Enter shortcut
            handle.editor.addCommand(
                monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
                () => {
                    executeCurrent();
                },
            );

            // Load session from backend
            console.log("[EDITOR-DEBUG] Loading session for id:", id);
            loadEditorSession(id)
                .then((session) => {
                    isLoadingSession = false;
                    console.log("[EDITOR-DEBUG] Session loaded:", {
                        hasSession: !!session,
                        sessionContent:
                            session?.content?.substring(0, 100) || "null",
                        currentEditorContent: handle.editor
                            .getValue()
                            .substring(0, 100),
                    });

                    if (session) {
                        log(
                            `Restored session from ${new Date(session.lastOpenedAt * 1000).toLocaleString()}`,
                        );
                        console.log(
                            "[EDITOR-DEBUG] Setting content from session",
                        );
                        handle.editor.setValue(session.content);
                        handle.editor.setPosition({
                            lineNumber: session.cursorLine,
                            column: session.cursorColumn,
                        });
                        if (context) {
                            context.content = session.content;
                        }
                        console.log(
                            "[EDITOR-DEBUG] After setValue from session:",
                            handle.editor.getValue().substring(0, 100),
                        );
                    } else {
                        // No saved session - ALWAYS set content to clear stale pooled content
                        log(
                            "No saved session, initializing with default content",
                        );
                        if (context?.content) {
                            console.log(
                                "[EDITOR-DEBUG] Setting content from context:",
                                context.content.substring(0, 100),
                            );
                            handle.editor.setValue(context.content);
                        } else {
                            const defaultContent = `-- SQL Auto-Completion Playground\n-- Context: ${schemaStore.selectedDatabase}.${schemaStore.activeSchema}\n-- Type 'SELECT' or table names from your active connection\n\nSELECT * FROM `;
                            console.log(
                                "[EDITOR-DEBUG] Setting default content",
                            );
                            handle.editor.setValue(defaultContent);
                            handle.editor.setPosition({
                                lineNumber: 4,
                                column: 15,
                            });
                        }
                        console.log(
                            "[EDITOR-DEBUG] After setValue default:",
                            handle.editor.getValue().substring(0, 100),
                        );
                    }
                    handle.editor.focus();
                    console.log(
                        "[EDITOR-DEBUG] ========== CALLBACK COMPLETE ==========",
                    );
                })
                .catch((e) => {
                    isLoadingSession = false;
                    console.error("[EDITOR-DEBUG] Failed to load session:", e);
                    handle.editor.focus();
                });

            // Store disposables for cleanup - CRITICAL to prevent event listener leaks!
            // Monaco subscriptions return IDisposable objects that MUST be disposed
            // when the component unmounts, otherwise they accumulate across pool reuse.
            const contentChangeDisposable =
                handle.editor.onDidChangeModelContent(() => {
                    const val = handle.editor.getValue();
                    const capturedId = id; // Capture current id value NOW
                    if (context) {
                        context.content = val;
                    }
                    // Trigger frontend state save
                    windowState.requestSave();
                    // Debounced backend save
                    const pos = handle.editor.getPosition();
                    console.log(
                        "[SAVE-DEBUG] Scheduling save for id:",
                        capturedId,
                        "content preview:",
                        val.substring(0, 50),
                    );
                    debouncedSave.save(() => {
                        console.log(
                            "[SAVE-DEBUG] Executing save for id:",
                            capturedId,
                        );
                        return saveEditorSession(
                            capturedId,
                            windowState.label,
                            val,
                            pos?.lineNumber ?? 1,
                            pos?.column ?? 1,
                            schemaStore.activeConnection?.id,
                            schemaStore.activeSchema,
                        );
                    });
                });

            // Also save cursor position changes (debounced)
            const cursorChangeDisposable =
                handle.editor.onDidChangeCursorPosition((e) => {
                    const pos = handle.editor.getPosition();
                    if (pos && headerController) {
                        headerController.onCursor(pos.lineNumber);
                    }

                    const val = handle.editor.getValue();
                    const capturedId = id;
                    debouncedSave.save(() => {
                        return saveEditorSession(
                            capturedId,
                            windowState.label,
                            val,
                            pos?.lineNumber ?? 1,
                            pos?.column ?? 1,
                            schemaStore.activeConnection?.id,
                            schemaStore.activeSchema,
                        );
                    });
                });

            // Listen for mouse down to trigger header update immediately on click
            // (Cursor position event fires too, but this can feel snappier for UI toggles)
            const mouseDownDisposable = handle.editor.onMouseDown((e) => {
                if (e.target.position && headerController) {
                    headerController.onCursor(e.target.position.lineNumber);
                }
            });

            // Store disposables for cleanup on unmount
            editorDisposables = [
                contentChangeDisposable,
                cursorChangeDisposable,
                mouseDownDisposable,
            ];

            // Enable Rich Headers (ViewZones above queries)
            const executeQuery = (
                queryText: string,
                startLine: number,
                endLine: number,
            ) => {
                executeQueryText(queryText, startLine, endLine);
            };

            const stopQuery = async (startLine: number, endLine: number) => {
                await handleStop();
            };

            headerController = enableQueryHeaders(
                handle.editor,
                executeQuery,
                stopQuery,
            );

            // Add ResizeObserver for robust layout updates
            const observer = new ResizeObserver(() => {
                handle.editor.layout();
            });
            observer.observe(editorContainer);
            editorDisposables.push({ dispose: () => observer.disconnect() });
        },
    );

    // Track disposables for cleanup
    let editorDisposables: { dispose: () => void }[] = [];

    // Flush pending saves and dispose event listeners on destroy
    onDestroy(() => {
        console.log(
            "[EDITOR-DEBUG] Component destroying, disposing",
            editorDisposables.length,
            "listeners",
        );
        debouncedSave.flush();
        // CRITICAL: Dispose all Monaco event subscriptions to prevent leaks
        editorDisposables.forEach((d) => d.dispose());
        editorDisposables = [];
        // Dispose headers
        headerController?.dispose();
        headerController = null;
    });
</script>

<div class="flex h-full w-full flex-col bg-background">
    <QueryEditorToolbar
        {isRunning}
        {executionTime}
        activeSchema={schemaStore.activeSchema || "public"}
        onExecute={executeCurrent}
        onStop={handleStop}
        onFormat={handleFormat}
        onClear={handleClear}
        onUndo={handleUndo}
        onRedo={handleRedo}
        onExplain={handleExplain}
        onSchemaChange={(v) => (schemaStore.activeSchema = v)}
    />

    <div class="flex-1 overflow-hidden">
        <ResizableSplitPane
            orientation="vertical"
            defaultRatio={0.6}
            bind:controlledRatio={resultSplitRatio}
            minLeft="100px"
            minRight="100px"
            rightVisible={showResultTable}
        >
            {#snippet left()}
                <!-- Editor Area -->
                <div class="h-full relative overflow-hidden">
                    <div
                        bind:this={editorContainer}
                        class="absolute inset-0 w-full h-full sql-editor-container"
                    ></div>
                </div>
            {/snippet}

            {#snippet right()}
                <!-- Result Table Area -->
                <div
                    class="h-full flex flex-col border-t border-border bg-background"
                >
                    <TableToolbar
                        bind:tableRef
                        columns={resultColumns}
                        {currentOffset}
                        totalRows={resultTotal}
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
                        isLoading={resultLoading}
                        {executionTime}
                        {pendingChangesCount}
                        onShowChanges={handleShowChanges}
                        onAddRow={handleAddRow}
                        onSaveChanges={handleToolbarSave}
                        {isSaving}
                    />
                    <div class="flex-1 relative">
                        <Table
                            bind:this={tableRef}
                            columns={resultColumns}
                            dataFetcher={resultDataFetcher}
                            isLoading={resultLoading}
                            onEditChange={handleEditChange}
                            onApplyEdits={handleApplyEdits}
                        />
                    </div>
                </div>
            {/snippet}
        </ResizableSplitPane>
    </div>
</div>

<style>
    /* No custom CSS padding on .view-lines as it breaks cursor coordinates. */
    /* Monaco's native 'padding' and 'lineDecorationsWidth' handle this correctly. */
</style>
