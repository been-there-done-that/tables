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

    import {
        normalizeColumnType,
        ensureUniqueColumnIds,
    } from "$lib/components/table/columnUtils";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import type { EditDelta } from "$lib/components/table/TableEditManager.svelte";
    import { pendingChangesStore } from "$lib/stores/pendingChanges.svelte";
    import { tick } from "svelte";

    let { id = "playground", context = $bindable({}) } = $props<{
        id?: string;
        context?: any;
    }>();

    // Ensure context structure exists
    if (!context.results) {
        context.results = {
            rows: [],
            columns: [],
            total: 0,
            visible: false,
            loading: false,
            pageSize: 100,
            offset: 0,
            whereClause: "",
            orderByClause: "",
            executionTime: 0,
            executedQueryText: "",
            currentBatchSize: 0,
            isExactTotal: true,
            isCountLoading: false,
            bottomTabVisible: true,
        };
    }
    if (!context.controller) {
        context.controller = {};
    }

    // Proxy properties for easier access within this component.
    const results = $derived(context.results);
    const controller = $derived(context.controller);

    // We don't initialize context here anymore as it's done in Session.openView.
    // This ensures consistency and initial reactivity.

    let editorContainer: HTMLElement;
    let editorHandle = $state<EditorHandle | null>(null);
    let logs: string[] = $state([]);
    let isLoadingSession = $state(true);
    let isRunning = $state(false);

    let headerController = $state<QueryHeaderController | null>(null);

    let tableRef: any = $state(null); // Still need a ref for potential actions

    // Update controller handlers
    $effect(() => {
        controller.execute = handleExecute;
        controller.refresh = handleRefresh;
        controller.cancel = handleCancel;
        controller.saveChanges = handleToolbarSave;
        controller.addRow = handleAddRow;
        controller.showChanges = handleShowChanges;
        controller.pageChange = handlePageChange;
        controller.pageSizeChange = handlePageSizeChange;
        controller.whereChange = handleWhereChange;
        controller.orderByChange = handleOrderByChange;
        controller.export = handleExport;
        controller.showDdl = handleShowDdl;
        controller.countUpdate = handleCountUpdate;
        controller.editChange = handleEditChange;
        controller.applyEdits = handleApplyEdits;
    });

    // Derived DataFetcher for Table component
    const resultDataFetcher: DataFetcher = $derived(async (params) => {
        const { offset, limit } = params;

        if (!schemaStore.activeConnection || !results.executedQueryText) {
            return {
                rows: [],
                total: 0,
                columns: results.columns,
            };
        }

        try {
            results.loading = true;
            const result = await invoke<any>("execute_query", {
                connectionId: schemaStore.activeConnection.id,
                sessionId: id,
                database: schemaStore.selectedDatabase,
                schema: schemaStore.activeSchema || "public",
                query: results.executedQueryText,
                component: "editor",
                limit: limit || results.pageSize,
                offset: offset || 0,
            });

            if (result.rows && result.columns) {
                const columnsMetadata = result.columns.map((c: any) => ({
                    name: c.name,
                    label: c.name,
                    type: normalizeColumnType(c.type),
                    rawType: c.type,
                    sortable: true,
                    filterable: true,
                    pinnable: true,
                    editable: true,
                    isPrimaryKey: c.is_primary_key,
                    sourceTable: c.source_table,
                    sourceSchema: c.source_schema,
                }));

                const processed = ensureUniqueColumnIds(
                    columnsMetadata,
                    result.rows,
                );

                results.total = result.total ?? results.total;
                results.currentBatchSize = processed.rows.length;

                // Add rowId
                const rowsWithId = processed.rows.map((r, i) => ({
                    ...r,
                    _rowId: (offset || 0) + i,
                }));

                return {
                    rows: rowsWithId,
                    total: results.total,
                    columns: processed.columns,
                };
            }
        } catch (e) {
            console.error("Data fetcher failed:", e);
            log(`Background fetch failed: ${e}`);
        } finally {
            results.loading = false;
        }

        return {
            rows: [],
            total: results.total,
            columns: results.columns,
        };
    });

    $effect(() => {
        controller.dataFetcher = resultDataFetcher;
    });

    // Sync run button visibility setting
    $effect(() => {
        if (headerController) {
            headerController.showAll = settingsStore.editorShowAllRunButtons;
        }
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

            // Auto-open bottom panel and ensure tab is visible
            settingsStore.sidebarBottomVisible = true;
            results.bottomTabVisible = true;

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
                results.loading = true;
                const result = await invoke<any>("execute_query", {
                    connectionId: schemaStore.activeConnection.id,
                    sessionId: id,
                    database: schemaStore.selectedDatabase,
                    schema: schemaStore.activeSchema || "public",
                    query: query,
                    component: "editor",
                    limit: results.pageSize,
                    offset: 0,
                });
                isRunning = false;
                results.executionTime =
                    result.duration_ms ?? performance.now() - startTime;

                const duration = results.executionTime;

                if (startLine && headerController) {
                    headerController.updateStatus(startLine, query, {
                        state: "success",
                        duration,
                    });
                }

                console.log("Query Result:", result);

                // Populate Result Table
                if (result.rows && result.columns) {
                    const columnsMetadata = result.columns.map((c: any) => ({
                        name: c.name,
                        label: c.name,
                        type: normalizeColumnType(c.type),
                        rawType: c.type,
                        sortable: true,
                        filterable: true,
                        pinnable: true,
                        editable: true,
                        width: undefined, // Auto-size
                        isPrimaryKey: c.is_primary_key,
                        sourceTable: c.source_table,
                        sourceSchema: c.source_schema,
                    }));

                    const processed = ensureUniqueColumnIds(
                        columnsMetadata,
                        result.rows,
                    );
                    results.rows = processed.rows;
                    results.columns = processed.columns;
                    results.total = result.total ?? result.rows.length;
                    results.isExactTotal = result.total !== undefined;
                    results.visible = true;
                    results.executedQueryText = query; // User feedback: attach query
                    if (controller.refreshTable) controller.refreshTable();
                } else if (
                    Array.isArray(result) &&
                    result.length > 0 &&
                    result[0].rows
                ) {
                    // Handle multi-result (batch)? Just take last for now
                    const last = result[result.length - 1];
                    if (last.rows && last.columns) {
                        const columnsMetadata = last.columns.map((c: any) => ({
                            name: c.name,
                            label: c.name,
                            type: normalizeColumnType(c.type),
                            rawType: c.type,
                            sortable: true,
                            filterable: true,
                            pinnable: true,
                            editable: true,
                            isPrimaryKey: c.is_primary_key,
                            sourceTable: c.source_table,
                            sourceSchema: c.source_schema,
                        }));

                        const processed = ensureUniqueColumnIds(
                            columnsMetadata,
                            last.rows,
                        );
                        results.rows = processed.rows;
                        results.columns = processed.columns;
                        results.total = last.total ?? last.rows.length;
                        results.isExactTotal = last.total !== undefined;
                        results.visible = true;
                        results.executedQueryText = query; // User feedback: attach query
                        if (controller.refreshTable) controller.refreshTable();
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
                results.loading = false;
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

        // Auto-open bottom panel and ensure tab is visible
        settingsStore.sidebarBottomVisible = true;
        results.bottomTabVisible = true;

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
            results.loading = true;
            const result = await invoke<any>("execute_query", {
                connectionId: schemaStore.activeConnection.id,
                sessionId: id,
                database: schemaStore.selectedDatabase,
                schema: schemaStore.activeSchema || "public",
                query: queryText,
                component: "editor",
                limit: results.pageSize,
                offset: 0,
            });
            isRunning = false;

            // Use backend duration if available (more accurate), else fallback to frontend measure
            results.executionTime =
                result.duration_ms ?? performance.now() - startTime;
            const duration = results.executionTime;

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
                const columnsMetadata = result.columns.map((c: any) => ({
                    name: c.name,
                    label: c.name,
                    type: normalizeColumnType(c.type),
                    rawType: c.type,
                    sortable: true,
                    filterable: true,
                    pinnable: true,
                    editable: true,
                    isPrimaryKey: c.is_primary_key,
                    sourceTable: c.source_table,
                    sourceSchema: c.source_schema,
                }));

                const processed = ensureUniqueColumnIds(
                    columnsMetadata,
                    result.rows,
                );
                results.rows = processed.rows;
                results.columns = processed.columns;
                results.total = result.total ?? result.rows.length;
                results.isExactTotal = result.total !== undefined;
                results.visible = true;
                results.executedQueryText = queryText;
                if (controller.refreshTable) controller.refreshTable();
            } else if (
                Array.isArray(result) &&
                result.length > 0 &&
                result[0].rows
            ) {
                const last = result[result.length - 1];
                if (last.rows && last.columns) {
                    const columnsMetadata = last.columns.map((c: any) => ({
                        name: c.name,
                        label: c.name,
                        type: normalizeColumnType(c.type),
                        rawType: c.type,
                        sortable: true,
                        filterable: true,
                        pinnable: true,
                        editable: true,
                        isPrimaryKey: c.is_primary_key,
                        sourceTable: c.source_table,
                        sourceSchema: c.source_schema,
                    }));

                    const processed = ensureUniqueColumnIds(
                        columnsMetadata,
                        last.rows,
                    );
                    results.rows = processed.rows;
                    results.columns = processed.columns;
                    results.total = last.total ?? last.rows.length;
                    results.isExactTotal = last.total !== undefined;
                    results.visible = true;
                    results.executedQueryText = queryText;
                    if (controller.refreshTable) controller.refreshTable();
                }
            }

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
            results.loading = false;
        }
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
        if (results.executedQueryText) {
            executeQueryText(results.executedQueryText);
        } else {
            executeCurrent();
        }
    }

    function handlePageChange(newOffset: number) {
        results.offset = newOffset;
        if (controller.refreshTable) controller.refreshTable();
    }

    function handlePageSizeChange(newSize: number) {
        results.pageSize = newSize;
        results.offset = 0;
        if (controller.refreshTable) controller.refreshTable();
    }

    function handleWhereChange(value: string) {
        results.whereClause = value;
    }

    function handleOrderByChange(value: string) {
        results.orderByClause = value;
    }

    async function handleCountUpdate() {
        // For arbitrary results, we don't have a reliable way to get total count
        // unless we wrapped the query in a CTE or subquery.
        // For now, we use resultTotal from the fetched buffer.
        results.isCountLoading = true;
        await new Promise((r) => setTimeout(r, 200));
        results.isCountLoading = false;
    }

    function handleExport(format: string) {
        console.log("Export requested:", format);
        log(`Exporting results to ${format} (stub)`);
    }

    function handleShowDdl() {
        log("Show DDL not supported for arbitrary queries.");
    }

    async function handleApplyEdits(
        _edits: any[],
    ): Promise<{ success: boolean; conflicts?: string[] }> {
        const deltas = tableRef?.getEditDeltas?.() ?? [];
        if (deltas.length === 0) return { success: true };

        const errors: string[] = [];
        for (const delta of deltas) {
            const { type, rowId, columnId, newValue, pkValues } = delta;

            // Resolve column to find source table
            const colDef = results.columns.find((c: any) => c.id === columnId);
            if (!colDef) {
                errors.push(`Column ${columnId} not found in results.`);
                continue;
            }

            const sourceTable = colDef.sourceTable;
            const sourceSchema =
                colDef.sourceSchema || schemaStore.activeSchema || "public";

            if (!sourceTable) {
                errors.push(
                    `Column ${columnId} does not have a known source table. Cannot edit.`,
                );
                continue;
            }

            // Find PKs for this source table from the available result columns
            // We look for columns that belong to the same source table and are marked as PK
            const targetPkCols = results.columns.filter(
                (c: any) => c.sourceTable === sourceTable && c.isPrimaryKey,
            );

            if (targetPkCols.length === 0) {
                // Try fallback to 'id' if present and unclaimed? No, strict mode requested.
                errors.push(
                    `No primary key found in results for table ${sourceTable}. Cannot edit safely.`,
                );
                continue;
            }

            // Check if we have values for all these PKs in the row (via pkValues or original row data?)
            // tableRef.getEditDeltas provides pkValues map "colId" -> value.
            // We assume table component populated this correctly using primaryKeyColumns,
            // BUT primaryKeyColumns passes ALL PKs from ALL tables mixed.
            // This is fine, as long as we pick the ones for THIS table.

            const missingPks = targetPkCols.filter(
                (pk: any) => pkValues[pk.id] === undefined,
            );
            if (missingPks.length > 0) {
                errors.push(
                    `Missing primary key value for ${missingPks.map((c: any) => c.id).join(", ")} for table ${sourceTable}.`,
                );
                continue;
            }

            let sql = "";

            if (type === "U") {
                const setClause = `"${columnId}" = ${formatSqlValue(newValue)}`;
                const whereClauses = targetPkCols.map(
                    (pk: any) =>
                        `"${pk.id}" = ${formatSqlValue(pkValues[pk.id])}`,
                );
                sql = `UPDATE "${sourceSchema}"."${sourceTable}" SET ${setClause} WHERE ${whereClauses.join(" AND ")};`;
            } else if (type === "D") {
                const whereClauses = targetPkCols.map(
                    (pk: any) =>
                        `"${pk.id}" = ${formatSqlValue(pkValues[pk.id])}`,
                );
                sql = `DELETE FROM "${sourceSchema}"."${sourceTable}" WHERE ${whereClauses.join(" AND ")};`;
            } else if (type === "I") {
                errors.push("INSERT not yet supported in playground results.");
                continue;
            }

            try {
                log(`Applying edit: ${sql}`);
                await invoke("execute_query", {
                    connectionId: schemaStore.activeConnection?.id,
                    sessionId: id,
                    database: schemaStore.selectedDatabase,
                    schema: sourceSchema,
                    query: sql,
                });
            } catch (e) {
                errors.push(`Failed to apply edit for row ${rowId}: ${e}`);
            }
        }

        if (errors.length > 0) return { success: false, conflicts: errors };

        // Re-run original query
        handleRefresh();
        return { success: true };
    }

    async function handleSaveChanges(): Promise<{
        success: boolean;
        errors?: string[];
    }> {
        results.isSaving = true;
        const res = await handleApplyEdits([]);
        results.isSaving = false;
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
            results.pendingDeltas = tableRef.getEditDeltas();
        }
        tick().then(() => {
            pendingChangesStore.setContext(
                results.pendingDeltas,
                "Result Set", // Generic name for multi-table/query results
                results.columns,
                [],
                "", // Schema agnostic
                {
                    onRevertRow: (rid: any) => tableRef?.revertRow?.(rid),
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
            results.pendingDeltas = tableRef.getEditDeltas();
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
        executionTime={results?.executionTime}
        activeSchema={schemaStore.activeSchema || "public"}
        showAll={settingsStore.editorShowAllRunButtons}
        onToggleShowAll={() =>
            (settingsStore.editorShowAllRunButtons =
                !settingsStore.editorShowAllRunButtons)}
        onExecute={executeCurrent}
        onStop={handleStop}
        onFormat={handleFormat}
        onClear={handleClear}
        onUndo={handleUndo}
        onRedo={handleRedo}
        onExplain={handleExplain}
        onSchemaChange={(v) => (schemaStore.activeSchema = v)}
    />

    <div class="flex-1 relative overflow-hidden">
        <!-- Editor Area -->
        <div class="absolute inset-0 w-full h-full">
            <div
                bind:this={editorContainer}
                class="absolute inset-0 w-full h-full sql-editor-container"
            ></div>
        </div>
    </div>
</div>

<style>
    /* No custom CSS padding on .view-lines as it breaks cursor coordinates. */
    /* Monaco's native 'padding' and 'lineDecorationsWidth' handle this correctly. */
</style>
