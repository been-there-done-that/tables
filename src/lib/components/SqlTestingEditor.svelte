<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import { cn } from "$lib/utils";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import IconAlertTriangle from "@tabler/icons-svelte/icons/alert-triangle";
    import * as monaco from "monaco-editor";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconSchema from "@tabler/icons-svelte/icons/table";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";

    import { settingsStore } from "$lib/stores/settings.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { composerStore } from "$lib/stores/composer.svelte";

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
    import { toast } from "svelte-sonner";

    import type { ViewState } from "$lib/stores/session.svelte";

    let { id = "playground", context = $bindable({}), view = null } = $props<{
        id?: string;
        context?: any;
        view?: ViewState | null;
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

    // Dangerous query confirmation
    let pendingRunAction = $state<(() => void) | null>(null);
    let dangerousWarningMsg = $state("");
    let runConfirmedOnce = false; // non-reactive bypass flag

    function detectDangerousPattern(query: string): string | null {
        const q = query.trim().toUpperCase();
        if (/^DROP\s/.test(q)) return "DROP statement cannot be undone.";
        if (/^TRUNCATE\s/.test(q)) return "TRUNCATE will delete all rows.";
        if (/^DELETE\s/.test(q) && !q.includes("WHERE")) return "DELETE without WHERE will erase every row.";
        if (/^UPDATE\s/.test(q) && !q.includes("WHERE")) return "UPDATE without WHERE will modify every row.";
        return null;
    }

    let headerController = $state<QueryHeaderController | null>(null);
    let selectionButton = $state<{ visible: boolean; x: number; y: number }>({ visible: false, x: 0, y: 0 });

    let tableRef: any = $state(null); // Still need a ref for potential actions

    // Auto-run when pendingRun flag is set (e.g. from agent run_query)
    $effect(() => {
        if (context.pendingRun && !isLoadingSession) {
            context.pendingRun = false;
            tick().then(() => handleExecute());
        }
    });

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

    // Holds rows already fetched by handleExecute so the subsequent refreshTable()
    // call can return them directly instead of hitting the DB a second time.
    let _pendingFetchResult: { rows: any[]; total: number; columns: any[] } | null = null;

    // Derived DataFetcher for Table component
    const resultDataFetcher: DataFetcher = $derived(async (params) => {
        const { offset, limit } = params;

        // Consume the cached result from the most-recent handleExecute/handleRefresh
        // so we don't re-query immediately after a successful execute.
        if (_pendingFetchResult && (offset ?? 0) === 0) {
            const cached = _pendingFetchResult;
            _pendingFetchResult = null;
            return cached;
        }

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
                database: context.databaseContext || schemaStore.selectedDatabase,
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

    // Stream write_file content into the editor as the agent generates it
    $effect(() => {
        const streaming = view?.streamingContent;
        if (streaming !== undefined && editorHandle?.editor) {
            const ed = editorHandle.editor;
            if (ed.getModel() && ed.getValue() !== streaming) {
                ed.setValue(streaming);
                const lineCount = ed.getModel()?.getLineCount() ?? 1;
                ed.setPosition({ lineNumber: lineCount, column: 9999 });
            }
        }
    });

    // Sync final write_file content once streaming completes (data.content updated, streamingContent cleared)
    $effect(() => {
        const content = view?.data?.content as string | undefined;
        const streaming = view?.streamingContent;
        // Only sync when streaming is not active (undefined = not streaming)
        if (content !== undefined && streaming === undefined && editorHandle?.editor) {
            const ed = editorHandle.editor;
            if (ed.getModel() && ed.getValue() !== content) {
                ed.setValue(content);
            }
        }
    });

    // Jump to line range when agent clicks a read_file/write_file chip
    $effect(() => {
        const revealAt = view?.data?.revealAt as { start: number; end: number; seq: number } | undefined;
        if (!revealAt || !editorHandle?.editor) return;
        const ed = editorHandle.editor;
        // seq dependency ensures re-execution even when revealing same line again
        void revealAt.seq;
        ed.revealLineInCenter(revealAt.start);
        ed.setSelection(new monaco.Range(revealAt.start, 1, revealAt.end, ed.getModel()?.getLineMaxColumn(revealAt.end) ?? 9999));
        ed.focus();
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
            const danger = !runConfirmedOnce && detectDangerousPattern(query);
            runConfirmedOnce = false;
            if (danger) {
                dangerousWarningMsg = danger;
                pendingRunAction = () => { runConfirmedOnce = true; executeCurrent(); };
                return;
            }

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
                    results.executedQueryText = query;
                    results.fetchedAt = new Date();
                    _pendingFetchResult = { rows: processed.rows, total: results.total, columns: processed.columns };
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
                        results.executedQueryText = query;
                        results.fetchedAt = new Date();
                        _pendingFetchResult = { rows: processed.rows, total: results.total, columns: processed.columns };
                        if (controller.refreshTable) controller.refreshTable();
                    }
                }

                log("Query completed successfully.");
                toast.success("Query executed", {
                    description: `${results.currentBatchSize} rows · ${results.executionTime.toFixed(0)}ms`,
                });
            } catch (e) {
                if (startLine && headerController) {
                    headerController.updateStatus(startLine, query, {
                        state: "error",
                        errorMessage: String(e),
                    });
                }
                console.error("Query execution failed:", e);
                log(`Query failed: ${e}`);
                toast.error("Query failed", { description: String(e) });
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

        const danger = !runConfirmedOnce && detectDangerousPattern(queryText);
        runConfirmedOnce = false;
        if (danger) {
            dangerousWarningMsg = danger;
            pendingRunAction = () => { runConfirmedOnce = true; executeQueryText(queryText, startLine, endLine); };
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
                database: context.databaseContext || schemaStore.selectedDatabase,
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
                results.fetchedAt = new Date();
                _pendingFetchResult = { rows: processed.rows, total: results.total, columns: processed.columns };
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
                    results.fetchedAt = new Date();
                    _pendingFetchResult = { rows: processed.rows, total: results.total, columns: processed.columns };
                    if (controller.refreshTable) controller.refreshTable();
                }
            }

            log("Query completed successfully.");
            toast.success("Query executed", {
                description: `${results.currentBatchSize} rows · ${results.executionTime.toFixed(0)}ms`,
            });
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
            toast.error("Query failed", { description: String(e) });
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
        toast.info("Export coming soon", { description: `${format.toUpperCase()} export is not yet implemented` });
    }

    function handleShowDdl() {
        log("Show DDL not supported for arbitrary queries.");
    }

    async function handleApplyEdits(
        _edits: any[],
    ): Promise<{ success: boolean; conflicts?: string[] }> {
        const deltas = controller.getEditDeltas?.() ?? [];
        if (deltas.length === 0) return { success: true };

        const errors: string[] = [];
        const deleteQueries: string[] = [];
        const insertQueries: string[] = [];
        const updateQueries: string[] = [];

        function resolveTarget(columnId: string, pkValues: Record<string, any>) {
            const colDef = results.columns.find((c: any) => c.id === columnId);
            if (!colDef?.sourceTable) {
                errors.push(`Column "${columnId}" has no known source table — cannot edit.`);
                return null;
            }
            const sourceTable = colDef.sourceTable as string;
            const sourceSchema: string = (colDef as any).sourceSchema ?? schemaStore.activeSchema ?? "public";
            const pkCols = results.columns.filter(
                (c: any) => c.sourceTable === sourceTable && c.isPrimaryKey,
            );
            if (pkCols.length === 0) {
                errors.push(`Table "${sourceTable}" has no primary key in the result set — cannot edit safely.`);
                return null;
            }
            const missing = pkCols.filter((pk: any) => pkValues[pk.id] === undefined);
            if (missing.length > 0) {
                errors.push(`Missing PK value(s): ${missing.map((c: any) => c.id).join(", ")}.`);
                return null;
            }
            const whereClause = pkCols
                .map((pk: any) => `"${pk.id}" = ${formatSqlValue(pkValues[pk.id])}`)
                .join(" AND ");
            return { sourceTable, sourceSchema, whereClause };
        }

        // Group INSERT deltas by rowId
        const insertGroups = new Map<any, typeof deltas>();
        for (const d of deltas.filter((d: any) => d.type === "I")) {
            if (!insertGroups.has(d.rowId)) insertGroups.set(d.rowId, []);
            insertGroups.get(d.rowId)!.push(d);
        }

        // Build DELETE queries
        const deletedRowIds = new Set(deltas.filter((d: any) => d.type === "D").map((d: any) => d.rowId));
        for (const rowId of deletedRowIds) {
            const d = deltas.find((x: any) => x.rowId === rowId && x.type === "D")!;
            const t = resolveTarget(d.columnId, d.pkValues ?? {});
            if (!t) continue;
            deleteQueries.push(`DELETE FROM "${t.sourceSchema}"."${t.sourceTable}" WHERE ${t.whereClause};`);
        }

        // Build INSERT queries
        for (const [, rowDeltas] of insertGroups) {
            const firstD = rowDeltas[0];
            const colDef = results.columns.find((c: any) => c.id === firstD?.columnId);
            if (!(colDef as any)?.sourceTable) { errors.push("INSERT: no source table."); continue; }
            const sourceSchema: string = (colDef as any).sourceSchema ?? schemaStore.activeSchema ?? "public";
            const insertCols = rowDeltas.filter((d: any) => d.newValue !== undefined).map((d: any) => `"${d.columnId}"`);
            const insertVals = rowDeltas.filter((d: any) => d.newValue !== undefined).map((d: any) => formatSqlValue(d.newValue));
            if (insertCols.length > 0) {
                insertQueries.push(
                    `INSERT INTO "${sourceSchema}"."${(colDef as any).sourceTable}" (${insertCols.join(", ")}) VALUES (${insertVals.join(", ")});`
                );
            }
        }

        // Build UPDATE queries
        for (const d of deltas.filter((x: any) => x.type === "U")) {
            const t = resolveTarget(d.columnId, d.pkValues ?? {});
            if (!t) continue;
            updateQueries.push(
                `UPDATE "${t.sourceSchema}"."${t.sourceTable}" SET "${d.columnId}" = ${formatSqlValue(d.newValue)} WHERE ${t.whereClause};`
            );
        }

        if (errors.length > 0) return { success: false, conflicts: errors };

        // Order: DELETE → INSERT → UPDATE (prevents FK/PK conflicts)
        const queries = [...deleteQueries, ...insertQueries, ...updateQueries];
        if (queries.length === 0) return { success: true };

        try {
            log(`Applying ${queries.length} mutations in transaction`);
            await invoke("execute_mutation_batch", {
                params: {
                    connectionId: schemaStore.activeConnection?.id,
                    sessionId: id,
                    database: schemaStore.selectedDatabase,
                    queries,
                },
            });
            handleRefresh();
            return { success: true };
        } catch (e) {
            return { success: false, conflicts: [String(e)] };
        }
    }

    async function handleSaveChanges(): Promise<{
        success: boolean;
        errors?: string[];
    }> {
        results.isSaving = true;
        const res = await handleApplyEdits([]);
        results.isSaving = false;
        if (res.success) {
            controller.revertAll?.();
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
        results.pendingDeltas = controller.getEditDeltas?.() ?? [];
        tick().then(() => {
            const firstDelta = results.pendingDeltas[0];
            const firstCol = results.columns.find((c: any) => c.id === firstDelta?.columnId);
            const displayName: string = (firstCol as any)?.sourceTable ?? "Query Result";
            const displaySchema: string = (firstCol as any)?.sourceSchema ?? "";

            pendingChangesStore.setContext(
                results.pendingDeltas,
                displayName,
                results.columns,
                [],
                displaySchema,
                {
                    onRevertRow: (rid: any) => controller.revertRow?.(rid),
                    onRevertAll: () => controller.revertAll?.(),
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
        results.pendingDeltas = controller.getEditDeltas?.() ?? [];
    }

    async function handleCancel() {
        if (!schemaStore.activeConnection) return;
        try {
            await invoke("cancel_query", {
                connectionId: schemaStore.activeConnection.id,
            });
            log("Query cancellation requested.");
            toast.info("Query cancelled");
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

    function addSelectionToChat(ed: import("monaco-editor").editor.ICodeEditor) {
        const selection = ed.getSelection();
        if (!selection) return;
        const isEmpty =
            selection.startLineNumber === selection.endLineNumber &&
            selection.startColumn === selection.endColumn;
        if (isEmpty) return;

        const session = windowState.activeSession;
        const activeView = session?.views.find(v => v.id === session.activeViewId);
        const path = activeView?.title ?? "query.sql";

        composerStore.pendingChip = {
            path,
            lineStart: selection.startLineNumber,
            lineEnd: selection.endLineNumber,
        };
        selectionButton = { visible: false, x: 0, y: 0 };
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
                readOnly: context?.readOnly === true,
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
                        if (context?.initialValue !== undefined) {
                            console.log(
                                "[EDITOR-DEBUG] Setting content from initialValue:",
                                context.initialValue.substring(0, 100),
                            );
                            handle.editor.setValue(context.initialValue);
                        } else if (context?.content) {
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

            // Selection → floating "Add to chat" button
            const selectionChangeDisposable = handle.editor.onDidChangeCursorSelection((e) => {
                const sel = e.selection;
                const isEmpty =
                    sel.startLineNumber === sel.endLineNumber &&
                    sel.startColumn === sel.endColumn;
                if (isEmpty) {
                    selectionButton = { visible: false, x: 0, y: 0 };
                    return;
                }
                const endPos = handle.editor.getScrolledVisiblePosition({
                    lineNumber: sel.endLineNumber,
                    column: sel.endColumn,
                });
                if (!endPos) {
                    selectionButton = { visible: false, x: 0, y: 0 };
                    return;
                }
                const containerRect = editorContainer.getBoundingClientRect();
                selectionButton = {
                    visible: true,
                    x: containerRect.left + endPos.left + 4,
                    y: containerRect.top + endPos.top - 30,
                };
            });

            // Cmd+L / Ctrl+L keybinding to add selection to chat
            handle.editor.addAction({
                id: "add-to-agent-chat",
                label: "Add to Agent Chat",
                keybindings: [
                    monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyL,
                ],
                run(ed) {
                    addSelectionToChat(ed);
                },
            });

            // Store disposables for cleanup on unmount
            editorDisposables = [
                contentChangeDisposable,
                cursorChangeDisposable,
                mouseDownDisposable,
                selectionChangeDisposable,
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

{#if pendingRunAction}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
        <div class="bg-background border border-border rounded-xl shadow-2xl p-5 max-w-sm w-full mx-4 space-y-4">
            <div class="flex items-start gap-3">
                <IconAlertTriangle class="size-5 text-amber-500 mt-0.5 shrink-0" />
                <div>
                    <h3 class="text-sm font-bold text-foreground">Dangerous query</h3>
                    <p class="text-xs text-muted-foreground mt-1">{dangerousWarningMsg}</p>
                </div>
            </div>
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    class="h-7 px-3 rounded text-[11px] font-bold hover:bg-muted transition-colors text-muted-foreground"
                    onclick={() => { pendingRunAction = null; dangerousWarningMsg = ""; }}
                >
                    Cancel
                </button>
                <button
                    type="button"
                    class="h-7 px-3 rounded text-[11px] font-bold bg-red-600 text-white hover:bg-red-700 transition-colors"
                    onclick={() => { const action = pendingRunAction; pendingRunAction = null; dangerousWarningMsg = ""; action?.(); }}
                >
                    Run Anyway
                </button>
            </div>
        </div>
    </div>
{/if}

<div class="flex h-full w-full flex-col bg-background">
    <QueryEditorToolbar
        {isRunning}
        executionTime={results?.executionTime}
        activeSchema={schemaStore.activeSchema || "public"}
        viewData={context}
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

{#if selectionButton.visible}
    <button
        class="fixed z-50 flex items-center gap-1 rounded border border-blue-500/50 bg-popover px-2 py-1 text-xs text-blue-400 shadow-md hover:bg-accent"
        style="left:{selectionButton.x}px; top:{selectionButton.y}px"
        onclick={() => { if (editorHandle?.editor) addSelectionToChat(editorHandle.editor); }}
    >
        + Add to chat
    </button>
{/if}

<style>
    /* No custom CSS padding on .view-lines as it breaks cursor coordinates. */
    /* Monaco's native 'padding' and 'lineDecorationsWidth' handle this correctly. */

</style>
