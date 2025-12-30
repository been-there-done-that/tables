<script lang="ts">
    import { onMount, tick } from "svelte";
    import type {
        Column,
        DataFetcher,
        OnApplyEdits,
        SortState,
        RowSelection,
        CellSelection,
        SelectionAnchor,
        SelectionBounds,
        ClipboardFormat,
        TableQueryContext,
    } from "./types";
    import TableHeader from "./TableHeader.svelte";
    import TableBody from "./TableBody.svelte";
    import { Button } from "$lib/components/ui/button";
    import { cn } from "$lib/utils";
    import {
        formatValueForClipboard,
        parseClipboardValue,
    } from "./clipboardUtils";
    import {
        DEFAULT_TOKEN,
        NULL_TOKEN,
        commitBooleanValue,
    } from "./valueUtils";

    interface Props {
        columns: Column[];
        dataFetcher: DataFetcher;
        onApplyEdits?: OnApplyEdits;
        class?: string;
        tableName?: string;
        tableSchema?: string;
        onOpenInQueryEditor?: (ctx: TableQueryContext) => void;
        onOpenNewQueryTab?: (ctx: TableQueryContext) => void;
    }

    let {
        columns,
        dataFetcher,
        onApplyEdits,
        class: className,
        tableName,
        tableSchema,
        onOpenInQueryEditor,
        onOpenNewQueryTab,
    }: Props = $props();

    type ClipboardApi = {
        readText: () => Promise<string>;
        writeText: (text: string) => Promise<void>;
    };

    let clipboardApiPromise: Promise<ClipboardApi> | null = null;

    async function resolveClipboardApi(): Promise<ClipboardApi> {
        if (clipboardApiPromise) return clipboardApiPromise;

        clipboardApiPromise = (async () => {
            const canUseNavigator =
                typeof navigator !== "undefined" &&
                typeof navigator.clipboard?.readText === "function" &&
                typeof navigator.clipboard?.writeText === "function";

            if (canUseNavigator) {
                return {
                    readText: () => navigator.clipboard!.readText(),
                    writeText: (text: string) =>
                        navigator.clipboard!.writeText(text),
                };
            }

            try {
                const mod = await import(
                    "@tauri-apps/plugin-clipboard-manager"
                );
                return {
                    readText: mod.readText,
                    writeText: (text: string) => mod.writeText(text),
                };
            } catch (err) {
                console.warn(
                    "Clipboard API unavailable; falling back to noop",
                    err,
                );
                return {
                    readText: async () => "",
                    writeText: async () => {},
                };
            }
        })();

        return clipboardApiPromise;
    }

    async function readClipboardText() {
        const api = await resolveClipboardApi();
        return api.readText();
    }

    async function writeClipboardText(text: string) {
        const api = await resolveClipboardApi();
        return api.writeText(text);
    }

    // State
    let tableColumns = $state<Column[]>([]);

    // Keep tableColumns in sync with incoming prop
    $effect(() => {
        tableColumns = columns.map((c) => ({ ...c }));
    });
    let hiddenColumnIds = $state<Set<string>>(new Set());
    let pinnedColumnIds = $state<string[]>([]);

    let visibleColumns = $derived.by(() => {
        const visible = tableColumns.filter((c) => !hiddenColumnIds.has(c.id));
        const pinned = visible.filter((c) => pinnedColumnIds.includes(c.id));
        const unpinned = visible.filter((c) => !pinnedColumnIds.includes(c.id));
        return [...pinned, ...unpinned];
    });

    let rows = $state<any[]>([]);
    let baselineRows = $state<Map<number, any>>(new Map());
    let totalRows = $state(0);
    let columnStats = $state<Record<string, { value: any; count: number }[]>>(
        {},
    );
    let loading = $state(false);
    let loadingMore = $state(false);
    let sortState = $state<SortState[]>([]);
    let filters = $state<Record<string, any>>({});
    let selectedRows = $state<RowSelection>({});
    let selectedCells = $state<CellSelection[]>([]);
    let selectionHead = $state<SelectionAnchor | null>(null);
    let editingCell = $state<CellSelection | null>(null);
    let pendingEdits = $state<Record<number, any>>({}); // Map of rowId -> partial row data
    let focusedCell = $state<{ rowIndex: number; columnIndex: number } | null>(
        null,
    );
    let undoStack = $state<
        {
            label: string;
            edits: Record<number, Record<string, any>>;
            previous: Record<number, Record<string, any>>;
        }[]
    >([]);
    const MAX_UNDO = 10;
    let clipboardFormat: ClipboardFormat = "tsv";
    let includeHeaders = true;

    // Drag selection state
    let isDragging = $state(false);
    let dragStartCell = $state<SelectionAnchor | null>(null);
    let selectionAnchor = $state<SelectionAnchor | null>(null);
    let contextMenuState = $state<{
        open: boolean;
        x: number;
        y: number;
        rowIndex: number;
        columnIndex: number;
    } | null>(null);

    // Virtualization state (passed to/from Body)
    let scrollTop = $state(0);
    let scrollLeft = $state(0);
    let containerHeight = $state(0);

    // Pagination state (for server-side pagination)
    let offset = $state(0);
    let limit = $state(500); // Initial batch size to handle large datasets without huge payloads

    let totalWidth = $derived(
        visibleColumns.reduce((acc, col) => acc + (col.width || 150), 0),
    );

    // ---------- Auto column width ----------
    const AUTO_SAMPLE = 200;
    const AUTO_MIN = 80;
    const AUTO_MAX = 400;
    const AUTO_PADDING = 24;
    let measureCtx: CanvasRenderingContext2D | null = null;

    function getMeasureCtx() {
        if (measureCtx) return measureCtx;
        const canvas = document.createElement("canvas");
        measureCtx = canvas.getContext("2d");
        if (measureCtx) {
            measureCtx.font =
                "14px Inter, 'Inter var', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
        }
        return measureCtx;
    }

    function measureTextWidth(text: string): number {
        const ctx = getMeasureCtx();
        if (!ctx) return text.length * 8;
        return ctx.measureText(text).width;
    }

    function normalizeCellValue(v: any): string {
        if (v === null || v === undefined) return "";
        if (typeof v === "string") return v;
        if (typeof v === "number" || typeof v === "boolean") return String(v);
        try {
            return JSON.stringify(v);
        } catch {
            return String(v);
        }
    }

    function autoSizeColumns() {
        if (!rows.length) return;
        const needsAuto = tableColumns.some((c) => c.width == null);
        if (!needsAuto) return;

        const samples = rows.slice(0, AUTO_SAMPLE);
        const widthMap: Record<string, number> = {};

        for (const col of tableColumns) {
            const isDateTime = col.type === "datetime";
            const headerTz = isDateTime ? " GMT+05:30" : "";
            const cellTz = isDateTime ? " AM GMT+05:30" : "";

            let max = measureTextWidth((col.label ?? col.id) + headerTz);
            const headerMeasured = max;

            for (const row of samples) {
                const text = normalizeCellValue(row[col.id]);
                const w = measureTextWidth(
                    (text.slice(0, 200) + cellTz).trim(),
                );
                if (w > max) max = w;
            }
            const min = col.minWidth ?? AUTO_MIN;
            const maxClamp = col.maxWidth ?? AUTO_MAX;
            const finalWidth = Math.min(
                Math.max(max + AUTO_PADDING, min),
                maxClamp,
            );
            console.info("[AutoWidth]", {
                column: col.id,
                header: headerMeasured,
                maxMeasured: max,
                finalWidth,
                min,
                maxClamp,
                sampleRows: samples.length,
            });
            widthMap[col.id] = finalWidth;
        }

        tableColumns = tableColumns.map((c) =>
            c.width != null ? c : { ...c, width: widthMap[c.id] },
        );
    }

    // Fetch data
    async function loadData(options: { append?: boolean } = {}) {
        const { append = false } = options;

        if (append) {
            if (loadingMore || loading) return;
        } else {
            if (loading) return;
        }

        const targetOffset = append ? rows.length : offset;

        if (append) loadingMore = true;
        else loading = true;

        const startedAt = performance.now?.() ?? Date.now();

        try {
            const result = await dataFetcher({
                offset: targetOffset,
                limit,
                sort: sortState,
                filters,
            });
            if (result.columns?.length) {
                tableColumns = mergeIncomingColumns(result.columns);
                hiddenColumnIds = new Set(
                    Array.from(hiddenColumnIds).filter((id) =>
                        result.columns!.some((c) => c.id === id),
                    ),
                );
            }

            const mappedRows = result.rows.map((row, index) => ({
                ...row,
                _rowId: targetOffset + index + 1,
            }));

            if (append) {
                rows = [...rows, ...mappedRows];
                mappedRows.forEach((row) => {
                    baselineRows.set(row._rowId, { ...row });
                });
            } else {
                rows = mappedRows;
                baselineRows = new Map(
                    rows.map((row) => [row._rowId, { ...row }]),
                );
                undoStack = [];
            }

            // Auto-size columns once we have data and widths are not set
            if (!append) {
                autoSizeColumns();
            }

            totalRows = result.total;
            columnStats =
                result.columnStats ||
                (append
                    ? columnStats
                    : computeColumnStatsClientSide(result.rows));
        } catch (e) {
            console.error("Failed to load data", e);
        } finally {
            const endedAt = performance.now?.() ?? Date.now();
            const elapsed = Math.round(endedAt - startedAt);
            console.info("[Table] loadData", {
                append,
                offset: targetOffset,
                limit,
                received: append ? rows.length : rows.length,
                totalRows,
                loadingMore,
                loading,
                elapsedMs: elapsed,
            });
            loading = false;
            loadingMore = false;
        }
    }

    function mergeIncomingColumns(incoming: Column[]): Column[] {
        const existing = new Map(tableColumns.map((c) => [c.id, c]));
        return incoming.map((col) => {
            const prev = existing.get(col.id);
            return {
                ...col,
                // Preserve user-resized widths if backend doesn't override
                width: col.width ?? prev?.width,
                minWidth: col.minWidth ?? prev?.minWidth,
                maxWidth: col.maxWidth ?? prev?.maxWidth,
                editable: col.editable ?? prev?.editable ?? false,
                sortable: col.sortable ?? prev?.sortable ?? false,
                filterable: col.filterable ?? prev?.filterable ?? false,
            };
        });
    }

    // Fallback: compute unique values client-side if backend doesn't provide them
    function computeColumnStatsClientSide(currentRows: any[]) {
        const stats: Record<string, { value: any; count: number }[]> = {};
        tableColumns.forEach((col) => {
            const counts = new Map<any, number>();
            currentRows.forEach((row) => {
                const val = row[col.id];
                counts.set(val, (counts.get(val) || 0) + 1);
            });
            stats[col.id] = Array.from(counts.entries()).map(
                ([value, count]) => ({
                    value,
                    count,
                }),
            );
        });
        return stats;
    }

    // Client-side filtering of loaded rows
    let filteredRows = $derived.by(() => {
        let result = [...rows];

        // Apply filters
        Object.entries(filters).forEach(([columnId, filterValue]) => {
            if (!filterValue) return;

            if (filterValue.type === "in" && filterValue.values) {
                result = result.filter((row) => {
                    const cellValue = String(row[columnId]);
                    return filterValue.values.includes(cellValue);
                });
            } else if (filterValue.type === "equals") {
                result = result.filter(
                    (row) => row[columnId] === filterValue.value,
                );
            } else if (filterValue.type === "contains") {
                result = result.filter((row) =>
                    String(row[columnId])
                        .toLowerCase()
                        .includes(String(filterValue.value).toLowerCase()),
                );
            }
            // Add more filter types as needed
        });

        return result;
    });

    // Selection helpers
    function clampAnchor(
        anchor: SelectionAnchor | null,
    ): SelectionAnchor | null {
        if (!anchor) return null;
        if (!filteredRows.length || !visibleColumns.length) return null;

        return {
            rowIndex: Math.min(
                Math.max(anchor.rowIndex, 0),
                filteredRows.length - 1,
            ),
            columnIndex: Math.min(
                Math.max(anchor.columnIndex, 0),
                visibleColumns.length - 1,
            ),
        };
    }

    function buildSelectedCells(bounds: SelectionBounds): CellSelection[] {
        const cells: CellSelection[] = [];
        for (let r = bounds.top; r <= bounds.bottom; r++) {
            const row = filteredRows[r];
            if (!row) continue;
            for (let c = bounds.left; c <= bounds.right; c++) {
                const column = visibleColumns[c];
                if (!column) continue;
                cells.push({ rowId: row._rowId, columnId: column.id });
            }
        }
        return cells;
    }

    function getSelectionBounds(): SelectionBounds | null {
        const anchor = clampAnchor(selectionAnchor ?? focusedCell);
        const head = clampAnchor(
            selectionHead ?? focusedCell ?? selectionAnchor ?? anchor,
        );

        if (!anchor || !head) return null;

        return {
            top: Math.min(anchor.rowIndex, head.rowIndex),
            bottom: Math.max(anchor.rowIndex, head.rowIndex),
            left: Math.min(anchor.columnIndex, head.columnIndex),
            right: Math.max(anchor.columnIndex, head.columnIndex),
        };
    }

    function setSelection(
        anchor: SelectionAnchor,
        head?: SelectionAnchor,
        opts: { preserveAnchor?: boolean; focusHead?: boolean } = {},
    ) {
        const clampedAnchor = clampAnchor(anchor);
        const clampedHead = clampAnchor(head ?? anchor);
        if (!clampedAnchor || !clampedHead) return;

        selectionAnchor = opts.preserveAnchor
            ? (selectionAnchor ?? clampedAnchor)
            : clampedAnchor;
        selectionHead = clampedHead;

        const bounds: SelectionBounds = {
            top: Math.min(selectionAnchor.rowIndex, selectionHead.rowIndex),
            bottom: Math.max(selectionAnchor.rowIndex, selectionHead.rowIndex),
            left: Math.min(
                selectionAnchor.columnIndex,
                selectionHead.columnIndex,
            ),
            right: Math.max(
                selectionAnchor.columnIndex,
                selectionHead.columnIndex,
            ),
        };

        selectedCells = buildSelectedCells(bounds);
        focusedCell =
            opts.focusHead === false
                ? focusedCell
                : {
                      rowIndex: clampedHead.rowIndex,
                      columnIndex: clampedHead.columnIndex,
                  };
        // Clear row selection when cells are manipulated
        selectedRows = {};
    }

    function getActiveBounds(): SelectionBounds | null {
        const bounds = getSelectionBounds();
        if (bounds) return bounds;
        if (focusedCell) {
            const { rowIndex, columnIndex } = focusedCell;
            return {
                top: rowIndex,
                bottom: rowIndex,
                left: columnIndex,
                right: columnIndex,
            };
        }
        return null;
    }

    function clearSelection() {
        selectedCells = [];
        selectionAnchor = null;
        selectionHead = null;
        // Preserve focus to avoid jumping back to the first cell on next key press
        selectedRows = {};
    }

    function applyEditsLocally(
        edits: Record<number, Record<string, any>>,
        label = "edit",
        pushToUndo = true,
    ) {
        if (!edits || Object.keys(edits).length === 0) return;

        const previous: Record<number, Record<string, any>> = {};
        const updatedRows = [...rows];
        const rowIndexMap = new Map<number, number>();
        rows.forEach((r, idx) => rowIndexMap.set(r._rowId, idx));
        const newPending: Record<number, any> = { ...pendingEdits };

        for (const [rowIdKey, changes] of Object.entries(edits)) {
            const rowId = Number(rowIdKey);
            const rowIndex = rowIndexMap.get(rowId);
            if (rowIndex === undefined) continue;

            const rowCopy = { ...updatedRows[rowIndex] };
            for (const [columnId, newValue] of Object.entries(changes)) {
                if (!previous[rowId]) previous[rowId] = {};
                previous[rowId][columnId] = rowCopy[columnId];
                rowCopy[columnId] = newValue;

                const baseline = baselineRows.get(rowId);
                const matchesBaseline =
                    baseline !== undefined && baseline[columnId] === newValue;

                if (matchesBaseline) {
                    if (newPending[rowId]) {
                        delete newPending[rowId][columnId];
                        if (Object.keys(newPending[rowId]).length === 0) {
                            delete newPending[rowId];
                        }
                    }
                } else {
                    if (!newPending[rowId]) newPending[rowId] = {};
                    newPending[rowId][columnId] = newValue;
                }
            }
            updatedRows[rowIndex] = rowCopy;
        }

        rows = updatedRows;
        pendingEdits = newPending;

        if (pushToUndo && Object.keys(previous).length > 0) {
            undoStack = [{ label, edits, previous }, ...undoStack].slice(
                0,
                MAX_UNDO,
            );
        }
    }

    function performUndo() {
        const action = undoStack.shift();
        if (!action) return;
        applyEditsLocally(action.previous, `undo: ${action.label}`, false);
    }
    // Handlers
    function handleSort(
        columnId: string,
        multi: boolean,
        direction?: "asc" | "desc",
    ) {
        const existing = sortState.find((s) => s.columnId === columnId);
        let newSort: SortState[] = [];

        if (direction) {
            if (multi) {
                newSort = [
                    ...sortState.filter((s) => s.columnId !== columnId),
                    { columnId, direction },
                ];
            } else {
                newSort = [{ columnId, direction }];
            }
        } else {
            // Toggle logic
            if (multi) {
                newSort = [...sortState];
                if (existing) {
                    if (existing.direction === "asc")
                        existing.direction = "desc";
                    else
                        newSort = newSort.filter(
                            (s) => s.columnId !== columnId,
                        );
                } else {
                    newSort.push({ columnId, direction: "asc" });
                }
            } else {
                if (existing && existing.direction === "asc") {
                    newSort = [{ columnId, direction: "desc" }];
                } else if (existing && existing.direction === "desc") {
                    newSort = [];
                } else {
                    newSort = [{ columnId, direction: "asc" }];
                }
            }
        }
        sortState = newSort;
    }

    function handleClearSort(columnId: string) {
        sortState = sortState.filter((s) => s.columnId !== columnId);
    }

    function buildQueryContext(columnId?: string): TableQueryContext {
        return {
            tableName,
            tableSchema,
            columnId,
            selectedColumns: columnId
                ? [columnId]
                : visibleColumns.map((c) => c.id),
        };
    }

    function handleOpenInQueryEditor(columnId: string) {
        onOpenInQueryEditor?.(buildQueryContext(columnId));
    }

    function handleOpenNewQueryTab(columnId: string) {
        onOpenNewQueryTab?.(buildQueryContext(columnId));
    }

    function handleFilterChange(columnId: string, value: any) {
        if (value === undefined || value === null || value === "") {
            const newFilters = { ...filters };
            delete newFilters[columnId];
            filters = newFilters;
        } else {
            filters = { ...filters, [columnId]: value };
        }
    }

    function handleResize(columnId: string, newWidth: number) {
        const colIndex = tableColumns.findIndex((c) => c.id === columnId);
        if (colIndex !== -1) {
            // Create a new array to trigger reactivity if needed, though $state with mutation works in Svelte 5
            // but for array items sometimes explicit assignment helps.
            tableColumns[colIndex].width = newWidth;
        }
    }

    function handleHideColumn(columnId: string) {
        hiddenColumnIds.add(columnId);
        hiddenColumnIds = new Set(hiddenColumnIds);
        pinnedColumnIds = pinnedColumnIds.filter((id) => id !== columnId);
    }

    function handleMoveColumn(fromId: string, toId: string) {
        const fromIndex = tableColumns.findIndex((c) => c.id === fromId);
        const toIndex = tableColumns.findIndex((c) => c.id === toId);
        if (fromIndex === -1 || toIndex === -1) return;

        const newColumns = [...tableColumns];
        const [moved] = newColumns.splice(fromIndex, 1);
        newColumns.splice(toIndex, 0, moved);
        tableColumns = newColumns;
    }

    function handlePinColumn(columnId: string) {
        if (pinnedColumnIds.includes(columnId)) return;
        if (pinnedColumnIds.length >= 3) {
            console.warn("Max 3 pinned columns allowed");
            return;
        }
        const col = tableColumns.find((c) => c.id === columnId);
        if (
            col?.type === "json" ||
            col?.type === "jsonb" ||
            col?.type === "JSON"
        ) {
            console.warn("Pinned columns cannot be wide JSON types");
            return;
        }
        pinnedColumnIds = [...pinnedColumnIds, columnId];
    }

    function handleUnpinColumn(columnId: string) {
        pinnedColumnIds = pinnedColumnIds.filter((id) => id !== columnId);
    }

    // ---------- Persistence ----------
    const STORAGE_VERSION = "v1";
    let isInitialLoad = true;

    function getStorageKey() {
        if (!tableName) return null;
        return `table_view_state_${tableSchema || "default"}_${tableName}_${STORAGE_VERSION}`;
    }

    function saveViewState() {
        const key = getStorageKey();
        if (!key || isInitialLoad) return;

        const state = {
            columnOrder: tableColumns.map((c) => c.id),
            columnWidths: tableColumns.reduce(
                (acc, c) => {
                    if (c.width) acc[c.id] = c.width;
                    return acc;
                },
                {} as Record<string, number>,
            ),
            hiddenColumnIds: Array.from(hiddenColumnIds),
            pinnedColumnIds,
        };
        localStorage.setItem(key, JSON.stringify(state));
        console.info("[Table] saveViewState", key, state);
    }

    function loadViewState() {
        const key = getStorageKey();
        if (!key) return;

        try {
            const saved = localStorage.getItem(key);
            if (!saved) return;
            const state = JSON.parse(saved);

            if (state.columnOrder) {
                const orderMap = new Map(
                    state.columnOrder.map((id: string, i: number) => [id, i]),
                );
                tableColumns = [...tableColumns].sort((a, b) => {
                    const aIdx = orderMap.get(a.id) ?? 999;
                    const bIdx = orderMap.get(b.id) ?? 999;
                    return aIdx - bIdx;
                });
            }

            if (state.columnWidths) {
                tableColumns = tableColumns.map((c) => ({
                    ...c,
                    width: state.columnWidths[c.id] ?? c.width,
                }));
            }

            if (state.hiddenColumnIds) {
                hiddenColumnIds = new Set(state.hiddenColumnIds);
            }

            if (state.pinnedColumnIds) {
                pinnedColumnIds = state.pinnedColumnIds;
            }
            console.info("[Table] loadViewState", key, state);
        } catch (e) {
            console.error("Failed to load view state", e);
        } finally {
            isInitialLoad = false;
        }
    }

    // Persist changes
    $effect(() => {
        // Track dependencies we want to persist
        const _ = {
            columns: tableColumns,
            hidden: hiddenColumnIds,
            pinned: pinnedColumnIds,
        };
        saveViewState();
    });

    function handleResetColumnWidth(columnId: string) {
        tableColumns = tableColumns.map((c) => {
            if (c.id === columnId) {
                return { ...c, width: undefined };
            }
            return c;
        });
        // Let auto-size recalculate on next render if needed
        tick().then(() => autoSizeColumns());
    }

    function handleResetColumnOrder() {
        // Reset to original column order from props
        const originalOrder = new Map(columns.map((c, i) => [c.id, i]));
        tableColumns = [...tableColumns].sort((a, b) => {
            const aIdx = originalOrder.get(a.id) ?? 999;
            const bIdx = originalOrder.get(b.id) ?? 999;
            return aIdx - bIdx;
        });
    }

    function handleResetAll() {
        hiddenColumnIds = new Set();
        pinnedColumnIds = [];
        tableColumns = columns.map((c) => ({ ...c, width: undefined }));
        const key = getStorageKey();
        if (key) localStorage.removeItem(key);
        tick().then(() => autoSizeColumns());
    }

    function handleHideOtherColumns(columnId: string) {
        const toHide = tableColumns
            .filter((c) => c.id !== columnId)
            .map((c) => c.id);
        hiddenColumnIds = new Set(toHide);
    }

    function handleShowColumnList() {
        hiddenColumnIds = new Set();
    }

    function getUniqueValues(columnId: string) {
        return columnStats[columnId] || [];
    }

    function handleRowSelect(rowId: number, multi: boolean, range: boolean) {
        // TODO: Implement complex selection logic (shift/ctrl click)
        if (multi) {
            selectedRows = { ...selectedRows, [rowId]: !selectedRows[rowId] };
        } else {
            selectedRows = { [rowId]: true };
        }
        // Clear cell selection when selecting rows
        selectedCells = [];
    }

    onMount(() => {
        loadViewState();
        loadData();

        // Prevent Cmd+A from selecting page text (desktop app behavior)
        const handleGlobalKeydown = (e: KeyboardEvent) => {
            if ((e.metaKey || e.ctrlKey) && e.key === "a") {
                const target = e.target as HTMLElement;
                // Allow Cmd+A in inputs, textareas, contenteditable, and the table
                if (
                    target.tagName !== "INPUT" &&
                    target.tagName !== "TEXTAREA" &&
                    !target.isContentEditable &&
                    target !== tableContainer &&
                    !tableContainer?.contains(target)
                ) {
                    e.preventDefault();
                }
            }
        };

        document.addEventListener("keydown", handleGlobalKeydown);

        return () => {
            document.removeEventListener("keydown", handleGlobalKeydown);
        };
    });

    let headerContainer: HTMLDivElement;
    let tableContainer: HTMLDivElement;
    let tableBody: TableBody;

    function handleBodyScroll(e: Event) {
        const target = e.target as HTMLDivElement;
        scrollLeft = target.scrollLeft;
        if (headerContainer) {
            headerContainer.scrollLeft = scrollLeft;
        }

        const nearBottomThreshold = 800;
        const scrolledBottom =
            target.scrollTop + target.clientHeight >=
            target.scrollHeight - nearBottomThreshold;

        const canLoadMore =
            !loadingMore &&
            !loading &&
            rows.length < totalRows &&
            scrolledBottom;

        if (canLoadMore) {
            console.info("[Table] scroll:near-bottom", {
                scrollTop: target.scrollTop,
                scrollHeight: target.scrollHeight,
                clientHeight: target.clientHeight,
                rows: rows.length,
                totalRows,
            });
            // Move offset forward and append next page
            offset = rows.length;
            loadData({ append: true });
        }
    }

    function handleCellClick(
        rowIndex: number,
        columnIndex: number,
        event: MouseEvent,
    ) {
        if (event.button !== 0) return; // only left click should change selection
        if (loading) return;

        const head: SelectionAnchor = { rowIndex, columnIndex };

        if (event.shiftKey) {
            const anchor = selectionAnchor ??
                focusedCell ?? {
                    rowIndex,
                    columnIndex,
                };
            setSelection(anchor, head, { preserveAnchor: true });
        } else if (event.metaKey || event.ctrlKey) {
            // Toggle individual cell without losing prior range selection
            const cellId = {
                rowId: filteredRows[rowIndex]._rowId,
                columnId: visibleColumns[columnIndex].id,
            };
            const exists = selectedCells.findIndex(
                (c) =>
                    c.rowId === cellId.rowId && c.columnId === cellId.columnId,
            );
            if (exists >= 0) {
                const next = [...selectedCells];
                next.splice(exists, 1);
                selectedCells = next;
                if (next.length === 0) {
                    selectionAnchor = null;
                    selectionHead = null;
                } else {
                    selectionHead = head;
                }
            } else {
                selectedCells = [...selectedCells, cellId];
                selectionAnchor = selectionAnchor ?? head;
                selectionHead = head;
            }
            focusedCell = head;
        } else {
            if (selectionIncludes(rowIndex, columnIndex)) {
                focusedCell = head;
            } else {
                setSelection(head, head);
            }
        }

        tick().then(() => tableContainer?.focus());
    }

    function handleCellMouseDown(
        rowIndex: number,
        columnIndex: number,
        event: MouseEvent,
    ) {
        if (event.button !== 0) return; // ignore right/middle for drag selection
        // Don't start drag if using modifier keys (let click handlers deal with it)
        if (event.shiftKey || event.metaKey || event.ctrlKey) {
            return;
        }

        if (loading) return;

        // Prevent default to avoid text selection
        event.preventDefault();

        // Ensure table has focus
        tableContainer?.focus();

        isDragging = true;
        dragStartCell = { rowIndex, columnIndex };
        setSelection({ rowIndex, columnIndex }, { rowIndex, columnIndex });
    }

    function handleCellMouseEnter(rowIndex: number, columnIndex: number) {
        if (!isDragging || !dragStartCell) return;

        // Ensure we're within bounds
        if (rowIndex < 0 || rowIndex >= filteredRows.length) return;
        if (columnIndex < 0 || columnIndex >= visibleColumns.length) return;

        setSelection(
            dragStartCell,
            { rowIndex, columnIndex },
            { preserveAnchor: true },
        );
    }

    function handleMouseUp() {
        isDragging = false;
        dragStartCell = null;
    }

    function selectionIncludes(rowIndex: number, columnIndex: number) {
        const bounds = getSelectionBounds();
        if (!bounds) return false;
        return (
            rowIndex >= bounds.top &&
            rowIndex <= bounds.bottom &&
            columnIndex >= bounds.left &&
            columnIndex <= bounds.right
        );
    }

    function ensureSelection(rowIndex: number, columnIndex: number) {
        if (selectionIncludes(rowIndex, columnIndex)) return;
        setSelection({ rowIndex, columnIndex }, { rowIndex, columnIndex });
    }

    function handleCellDoubleClick(rowIndex: number, columnIndex: number) {
        console.log("handleCellDoubleClick called", {
            rowIndex,
            columnIndex,
            hasOnApplyEdits: !!onApplyEdits,
        });
        if (!onApplyEdits) {
            console.warn(
                "handleCellDoubleClick: Read-only mode (no onApplyEdits)",
            );
            return;
        }
        const col = visibleColumns[columnIndex];
        const alwaysEditableTypes = [
            "boolean",
            "date",
            "datetime",
            "json",
            "enum",
        ];
        const canEdit =
            col.editable || alwaysEditableTypes.includes(col.type as any);
        console.info("[Table] edit:request", {
            reason: "doubleClick-or-enter",
            rowIndex,
            columnIndex,
            rowId: filteredRows[rowIndex]?._rowId,
            columnId: col.id,
            type: col.type,
            editableFlag: col.editable,
            canEdit,
            loading,
        });

        if (canEdit) {
            editingCell = {
                rowId: filteredRows[rowIndex]._rowId,
                columnId: col.id,
            };
            console.info("[Table] edit:state-set", { editingCell });
        } else {
            console.warn("[Table] edit:refused - not editable");
        }
    }

    function handleCellContextMenu(
        rowIndex: number,
        columnIndex: number,
        event: MouseEvent,
    ) {
        if (loading) return;
        event.preventDefault();
        event.stopPropagation();
        console.log("handleCellContextMenu", { rowIndex, columnIndex });

        // Keep existing selection if it already includes this cell; otherwise select this cell
        if (!selectionIncludes(rowIndex, columnIndex)) {
            setSelection({ rowIndex, columnIndex }, { rowIndex, columnIndex });
        }

        const x = Math.min(window.innerWidth - 180, Math.max(8, event.clientX));
        const y = Math.min(
            window.innerHeight - 200,
            Math.max(8, event.clientY),
        );

        contextMenuState = { open: true, x, y, rowIndex, columnIndex };
        console.log("contextMenuState set", contextMenuState);
    }

    function closeContextMenu() {
        console.log("closeContextMenu");
        contextMenuState = null;
    }

    function contextEdit() {
        console.log("contextEdit called", contextMenuState);
        if (!contextMenuState) {
            console.error("contextEdit: contextMenuState is null");
            return;
        }
        const { rowIndex, columnIndex } = contextMenuState;
        console.log("contextEdit extracting", { rowIndex, columnIndex });
        closeContextMenu();
        handleCellDoubleClick(rowIndex, columnIndex);
    }

    function contextCopy() {
        closeContextMenu();
        handleCopy();
    }

    function contextPaste() {
        closeContextMenu();
        handlePaste();
    }

    function applyValueToSelection(value: any, label: string) {
        const bounds =
            getSelectionBounds() ??
            ({
                top: focusedCell?.rowIndex ?? 0,
                bottom: focusedCell?.rowIndex ?? 0,
                left: focusedCell?.columnIndex ?? 0,
                right: focusedCell?.columnIndex ?? 0,
            } as SelectionBounds);

        const targetRowEnd = Math.min(bounds.bottom, filteredRows.length - 1);
        const targetColEnd = Math.min(bounds.right, visibleColumns.length - 1);

        const edits: Record<number, Record<string, any>> = {};

        for (let r = bounds.top; r <= targetRowEnd; r++) {
            const row = filteredRows[r];
            if (!row) continue;
            for (let c = bounds.left; c <= targetColEnd; c++) {
                const column = visibleColumns[c];
                if (!column?.editable) continue;
                if (!edits[row._rowId]) edits[row._rowId] = {};
                edits[row._rowId][column.id] = value;
            }
        }

        if (Object.keys(edits).length === 0) return;
        applyEditsLocally(edits, label, true);
    }

    function contextSetNull() {
        closeContextMenu();
        applyValueToSelection(NULL_TOKEN, "context-set-null");
    }

    function contextSetDefault() {
        closeContextMenu();
        applyValueToSelection(DEFAULT_TOKEN, "context-set-default");
    }

    function handleEditComplete(
        rowIndex: number,
        columnIndex: number,
        newValue: any,
    ) {
        // Ensure grid regains focus after committing edit
        const refocus = () => tableContainer?.focus();
        const rowId = filteredRows[rowIndex]._rowId;
        const columnId = visibleColumns[columnIndex].id;
        let normalizedValue = newValue;
        if (visibleColumns[columnIndex]?.type === "boolean") {
            normalizedValue = commitBooleanValue(newValue);
        }
        const oldValue = filteredRows[rowIndex][columnId];

        console.info("[Table] edit:commit", {
            rowIndex,
            columnIndex,
            rowId,
            columnId,
            oldValue,
            newValue: normalizedValue,
        });

        if (oldValue !== normalizedValue) {
            applyEditsLocally(
                { [rowId]: { [columnId]: normalizedValue } },
                "edit",
                true,
            );
        }
        editingCell = null;
        tick().then(refocus);
    }

    function handleEditCancel() {
        editingCell = null;
        tick().then(() => tableContainer?.focus());
    }

    async function handleCommit() {
        if (Object.keys(pendingEdits).length === 0) return;
        if (onApplyEdits) {
            try {
                const result = await onApplyEdits(pendingEdits);
                if (result.success) {
                    pendingEdits = {};
                    // Optionally reload data
                    await loadData();
                } else {
                    alert(
                        "Failed to save changes: " +
                            (result.conflicts || "Unknown error"),
                    );
                }
            } catch (e) {
                console.error("Commit failed", e);
            }
        }
    }

    function handleRollback() {
        pendingEdits = {};
        undoStack = [];
        // Reload original data
        loadData();
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (document.activeElement !== tableContainer) return;
        if (editingCell) return;
        if (loading) return;

        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "z") {
            e.preventDefault();
            performUndo();
            return;
        }

        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "c") {
            e.preventDefault();
            handleCopy();
            return;
        }

        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "v") {
            e.preventDefault();
            handlePaste();
            return;
        }

        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "a") {
            e.preventDefault();
            handleSelectAll();
            return;
        }

        if (e.key === "Escape") {
            e.preventDefault();
            clearSelection();
            return;
        }

        if (
            e.altKey &&
            (e.key === "ArrowLeft" || e.key === "ArrowRight") &&
            focusedCell
        ) {
            e.preventDefault();
            const currentColId = visibleColumns[focusedCell.columnIndex].id;
            const targetIndex =
                e.key === "ArrowLeft"
                    ? focusedCell.columnIndex - 1
                    : focusedCell.columnIndex + 1;

            if (targetIndex >= 0 && targetIndex < visibleColumns.length) {
                const targetColId = visibleColumns[targetIndex].id;
                handleMoveColumn(currentColId, targetColId);
                // Update focused cell to follow the column
                focusedCell = { ...focusedCell, columnIndex: targetIndex };
                // Also update selection to match new position if needed
                if (!e.shiftKey) {
                    setSelection(focusedCell, focusedCell);
                }
                ensureCellVisible(
                    focusedCell.rowIndex,
                    focusedCell.columnIndex,
                );
            }
            return;
        }

        if (!focusedCell) {
            if (filteredRows.length && visibleColumns.length) {
                setSelection({ rowIndex: 0, columnIndex: 0 });
            }
            return;
        }

        const { rowIndex, columnIndex } = focusedCell;
        let newRowIndex = rowIndex;
        let newColumnIndex = columnIndex;

        const clampRow = (r: number) =>
            Math.min(Math.max(r, 0), filteredRows.length - 1);
        const clampCol = (c: number) =>
            Math.min(Math.max(c, 0), visibleColumns.length - 1);

        if (e.shiftKey && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            if (e.key === "ArrowUp") newRowIndex = clampRow(0);
            else if (e.key === "ArrowDown")
                newRowIndex = clampRow(filteredRows.length - 1);
            else if (e.key === "ArrowLeft") newColumnIndex = clampCol(0);
            else if (e.key === "ArrowRight")
                newColumnIndex = clampCol(visibleColumns.length - 1);
            else return;

            setSelection(
                selectionAnchor ?? focusedCell,
                { rowIndex: newRowIndex, columnIndex: newColumnIndex },
                { preserveAnchor: true },
            );
            ensureCellVisible(newRowIndex, newColumnIndex);
            return;
        }

        if (e.key === "ArrowUp" && rowIndex > 0) {
            e.preventDefault();
            newRowIndex = rowIndex - 1;
        } else if (
            e.key === "ArrowDown" &&
            rowIndex < filteredRows.length - 1
        ) {
            e.preventDefault();
            newRowIndex = rowIndex + 1;
        } else if (e.key === "ArrowLeft" && columnIndex > 0) {
            e.preventDefault();
            newColumnIndex = columnIndex - 1;
        } else if (
            e.key === "ArrowRight" &&
            columnIndex < visibleColumns.length - 1
        ) {
            e.preventDefault();
            newColumnIndex = columnIndex + 1;
        } else if (e.key === "Enter") {
            e.preventDefault();
            handleCellDoubleClick(rowIndex, columnIndex);
            return;
        } else if (
            e.ctrlKey &&
            ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.key)
        ) {
            e.preventDefault();
            if (e.key === "ArrowUp" && rowIndex > 0) newRowIndex = rowIndex - 1;
            else if (
                e.key === "ArrowDown" &&
                rowIndex < filteredRows.length - 1
            )
                newRowIndex = rowIndex + 1;
            else if (e.key === "ArrowLeft" && columnIndex > 0)
                newColumnIndex = columnIndex - 1;
            else if (
                e.key === "ArrowRight" &&
                columnIndex < visibleColumns.length - 1
            )
                newColumnIndex = columnIndex + 1;
            else return;

            focusedCell = {
                rowIndex: newRowIndex,
                columnIndex: newColumnIndex,
            };

            const cellId = {
                rowId: filteredRows[newRowIndex]._rowId,
                columnId: visibleColumns[newColumnIndex].id,
            };
            const existing = selectedCells.findIndex(
                (c) =>
                    c.rowId === cellId.rowId && c.columnId === cellId.columnId,
            );
            if (existing >= 0) {
                const next = [...selectedCells];
                next.splice(existing, 1);
                selectedCells = next;
            } else {
                selectedCells = [...selectedCells, cellId];
            }
            selectionHead = {
                rowIndex: newRowIndex,
                columnIndex: newColumnIndex,
            };
            selectionAnchor = selectionAnchor ?? focusedCell;
            ensureCellVisible(newRowIndex, newColumnIndex);
            return;
        } else {
            return;
        }

        focusedCell = { rowIndex: newRowIndex, columnIndex: newColumnIndex };

        if (e.shiftKey) {
            setSelection(
                selectionAnchor ?? { rowIndex, columnIndex },
                { rowIndex: newRowIndex, columnIndex: newColumnIndex },
                { preserveAnchor: true },
            );
        } else {
            setSelection(
                { rowIndex: newRowIndex, columnIndex: newColumnIndex },
                { rowIndex: newRowIndex, columnIndex: newColumnIndex },
            );
        }

        ensureCellVisible(newRowIndex, newColumnIndex);
    }

    function ensureCellVisible(rowIndex: number, columnIndex: number) {
        if (!tableBody) return;
        if (editingCell) {
            console.info("[Table] ensureCellVisible:skip-editing", {
                rowIndex,
                columnIndex,
                editingCell,
            });
            return;
        }

        // Prefer DOM-based scroll to avoid height drift
        const container = tableBody.getContainer?.();
        const targetRowId = filteredRows[rowIndex]?._rowId;
        const targetEl =
            targetRowId && container
                ? container.querySelector(`[data-row-id="${targetRowId}"]`)
                : null;

        if (targetEl instanceof HTMLElement) {
            targetEl.scrollIntoView({ block: "nearest" });
        } else {
            console.info("[Table] ensureCellVisible:no-target", {
                rowIndex,
                columnIndex,
                targetRowId,
            });
            // Avoid fallback scrollToIndex to prevent drift if heights mismatch
            return;
        }

        // Horizontal scrolling
        // We need to calculate the left offset of the column
        // Since we don't have direct access to column DOM elements easily here without more state,
        // we can approximate or use the column widths if they are fixed/known.
        // But wait, we have `tableColumns` which might have width info if we track it.
        // The `visibleColumns` have a width property? Let's check `Column` type.
        // Assuming `width` is on `Column`.

        // A simpler approach for horizontal scroll if we don't want to track strict pixel widths:
        // We can't easily do "auto" without knowing current scrollLeft and container width.
        // But we can try to guess.

        // Let's rely on the fact that we can get the container from `TableBody` if we exposed it,
        // or just use `tableContainer` (which wraps the header, but the body has its own scroll).
        // Actually `TableBody` has the scrollable div.

        // Let's implement a best-effort horizontal scroll based on column index and estimated width
        // OR, better, let's just scroll if it's the first or last column for now?
        // No, that's not enough.

        // Let's calculate cumulative width
        let left = 0;
        for (let i = 0; i < columnIndex; i++) {
            left += visibleColumns[i].width || 150; // Default 150 if undefined
        }
        const colWidth = visibleColumns[columnIndex].width || 150;
        const right = left + colWidth;

        // We need the scrollable container's width and scrollLeft.
        // We don't have direct access to the VirtualScroller's container DOM from here easily
        // unless we expose getters.
        // But `handleBodyScroll` updates `scrollLeft`.
        // And `headerContainer` width is roughly the viewport width (minus scrollbar).

        if (headerContainer) {
            const containerWidth = headerContainer.clientWidth;

            if (left < scrollLeft) {
                console.info("[Table] ensureCellVisible:left", {
                    rowIndex,
                    columnIndex,
                    left,
                    scrollLeft,
                    containerWidth,
                });
                tableBody.scrollToLeft(left);
            } else if (right > scrollLeft + containerWidth) {
                console.info("[Table] ensureCellVisible:right", {
                    rowIndex,
                    columnIndex,
                    right,
                    scrollLeft,
                    containerWidth,
                });
                tableBody.scrollToLeft(right - containerWidth);
            }
        }
    }

    function parseClipboardGrid(text: string): any[][] {
        const trimmed = text.trimEnd();
        if (!trimmed) return [];

        // JSON first
        if (trimmed.startsWith("[") || trimmed.startsWith("{")) {
            try {
                const parsed = JSON.parse(trimmed);
                if (Array.isArray(parsed)) {
                    if (parsed.length === 0) return [];
                    if (Array.isArray(parsed[0])) {
                        return parsed as any[][];
                    }
                    if (typeof parsed[0] === "object") {
                        const keys = Object.keys(parsed[0]);
                        return (parsed as Record<string, any>[]).map((row) =>
                            keys.map((k) => row[k]),
                        );
                    }
                }
            } catch {
                // fall through
            }
        }

        const rowsRaw = trimmed.split(/\r?\n/);
        const delimiter = trimmed.includes("\t")
            ? "\t"
            : trimmed.includes(",")
              ? ","
              : "\t";
        return rowsRaw
            .map((r) => r.split(delimiter))
            .filter((r) => !(r.length === 1 && r[0] === ""));
    }

    function handleSelectAll() {
        if (!filteredRows.length || !visibleColumns.length) return;
        setSelection(
            { rowIndex: 0, columnIndex: 0 },
            {
                rowIndex: filteredRows.length - 1,
                columnIndex: visibleColumns.length - 1,
            },
        );
    }

    function handleCopy() {
        const bounds = getActiveBounds();
        if (!bounds) return;

        const rowsSlice = filteredRows.slice(bounds.top, bounds.bottom + 1);
        const colsSlice = visibleColumns.slice(bounds.left, bounds.right + 1);
        if (!rowsSlice.length || !colsSlice.length) return;

        if (clipboardFormat === "json") {
            const payload = rowsSlice.map((row) =>
                includeHeaders
                    ? Object.fromEntries(
                          colsSlice.map((c) => [c.id, row[c.id]]),
                      )
                    : colsSlice.map((c) => row[c.id]),
            );
            writeClipboardText(JSON.stringify(payload, null, 2));
            return;
        }

        const delimiter = clipboardFormat === "csv" ? "," : "\t";
        const lines: string[] = [];

        if (includeHeaders) {
            lines.push(colsSlice.map((c) => c.label ?? c.id).join(delimiter));
        }

        for (const row of rowsSlice) {
            const cells = colsSlice.map((col) =>
                formatValueForClipboard(row[col.id], col.type),
            );
            lines.push(cells.join(delimiter));
        }

        writeClipboardText(lines.join("\n"));
    }

    async function handlePaste() {
        if (!focusedCell) return;
        try {
            const text = await readClipboardText();
            if (!text) return;

            const grid = parseClipboardGrid(text);
            if (!grid.length) return;

            const selection = getSelectionBounds();
            const anchor = selection ?? {
                top: focusedCell.rowIndex,
                bottom: focusedCell.rowIndex,
                left: focusedCell.columnIndex,
                right: focusedCell.columnIndex,
            };

            // Drop header row if multiple rows copied (as before)
            const rowsData = grid.length > 1 ? grid.slice(1) : grid;
            const dataRows = rowsData.length;
            const dataCols = Math.max(...rowsData.map((r) => r.length), 1);

            const selRowCount = anchor.bottom - anchor.top + 1;
            const selColCount = anchor.right - anchor.left + 1;

            const targetRowCount = selection
                ? Math.max(selRowCount, dataRows)
                : dataRows;
            const targetColCount = selection
                ? Math.max(selColCount, dataCols)
                : dataCols;

            const targetBounds: SelectionBounds = {
                top: anchor.top,
                left: anchor.left,
                bottom: anchor.top + targetRowCount - 1,
                right: anchor.left + targetColCount - 1,
            };

            const targetRowEnd = Math.min(
                targetBounds.bottom,
                filteredRows.length - 1,
            );
            const targetColEnd = Math.min(
                targetBounds.right,
                visibleColumns.length - 1,
            );

            const edits: Record<number, Record<string, any>> = {};

            for (let r = targetBounds.top; r <= targetRowEnd; r++) {
                const row = filteredRows[r];
                if (!row) continue;
                const relativeR = r - anchor.top;
                const sourceRow = rowsData[relativeR % rowsData.length] ?? [];

                for (let c = targetBounds.left; c <= targetColEnd; c++) {
                    const column = visibleColumns[c];
                    if (!column?.editable) continue;

                    const relativeC = c - anchor.left;
                    const rawValue = sourceRow[relativeC % dataCols] ?? "";
                    const parsedValue = parseClipboardValue(
                        rawValue,
                        column.type,
                    );

                    if (!edits[row._rowId]) edits[row._rowId] = {};
                    edits[row._rowId][column.id] = parsedValue;
                }
            }

            if (Object.keys(edits).length === 0) return;

            applyEditsLocally(edits, "paste", true);
        } catch (e) {
            console.error("Paste failed", e);
        }
    }
    export function refresh() {
        loadData();
    }

    // Expose helpers for toolbar integration
    export function copySelection() {
        handleCopy();
    }

    export function getState() {
        return {
            sortState,
            filters,
            offset,
            limit,
            totalRows,
        };
    }

    export function setFilters(nextFilters: Record<string, any> = {}) {
        filters = { ...nextFilters };
        offset = 0;
        loadData();
    }

    export function setSort(nextSort: SortState[] = []) {
        sortState = [...nextSort];
        offset = 0;
        loadData();
    }

    // ... existing handlers ...
</script>

<div
    bind:this={tableContainer}
    tabindex="-1"
    class={cn(
        "flex flex-col flex-1 min-h-0 h-full w-full border border-[var(--theme-border-default)] overflow-hidden bg-[var(--theme-bg-primary)]",
        className,
    )}
    role="grid"
    oncontextmenu={(event) => event.preventDefault()}
    onkeydown={handleKeyDown}
    onmouseup={handleMouseUp}
    onmousedown={(e) => {
        // Prevent ANY click on the table from auto-focusing the container
        // Only cells should focus the table via explicit .focus() calls
        e.preventDefault();
    }}
    onclick={(e) => {
        const clickedOnBackground =
            e.target === tableContainer || e.target === headerContainer;

        if (clickedOnBackground) {
            // Clicking on empty space: clear selection
            clearSelection();
        }
    }}
>
    <!-- Header -->
    <div bind:this={headerContainer} class="flex-none overflow-hidden">
        <div
            class="border-b border-[var(--theme-border-default)] bg-[var(--theme-bg-secondary)]"
            style="width: {totalWidth + 60}px;"
        >
            <TableHeader
                columns={visibleColumns}
                {sortState}
                {filters}
                {pinnedColumnIds}
                onSort={handleSort}
                onClearSort={handleClearSort}
                onOpenInQueryEditor={handleOpenInQueryEditor}
                onOpenNewQueryTab={handleOpenNewQueryTab}
                onFilter={handleFilterChange}
                onResize={handleResize}
                onResetColumnWidth={handleResetColumnWidth}
                onMoveColumn={handleMoveColumn}
                onPinColumn={handlePinColumn}
                onUnpinColumn={handleUnpinColumn}
                onResetColumnOrder={handleResetColumnOrder}
                onResetAll={handleResetAll}
                onHideColumn={handleHideColumn}
                onHideOtherColumns={handleHideOtherColumns}
                onShowColumnList={handleShowColumnList}
                {getUniqueValues}
            />
        </div>
    </div>

    <!-- Body -->
    <div class="flex-1 overflow-hidden relative">
        <TableBody
            bind:this={tableBody}
            rows={filteredRows}
            columns={visibleColumns}
            {selectedRows}
            {selectedCells}
            {focusedCell}
            {editingCell}
            {pendingEdits}
            {loading}
            onRowSelect={handleRowSelect}
            onScroll={handleBodyScroll}
            onCellClick={handleCellClick}
            onCellMouseDown={handleCellMouseDown}
            onCellMouseEnter={handleCellMouseEnter}
            onCellDoubleClick={handleCellDoubleClick}
            onCellContextMenu={handleCellContextMenu}
            onEditComplete={handleEditComplete}
            onEditCancel={handleEditCancel}
        />
    </div>

    {#if contextMenuState?.open}
        <div
            class="fixed inset-0 z-1500"
            onclick={closeContextMenu}
            oncontextmenu={(e) => e.preventDefault()}
            role="presentation"
            tabindex="-1"
            onkeydown={(e) => {
                if (e.key === "Escape") {
                    e.preventDefault();
                    closeContextMenu();
                }
            }}
        ></div>
        <div
            class="fixed z-1501 bg-[var(--theme-bg-secondary)] border border-[var(--theme-border-default)] rounded-md shadow-md text-sm min-w-[180px] py-1 text-[var(--theme-fg-secondary)]"
            style={`top:${contextMenuState.y}px;left:${contextMenuState.x}px`}
            oncontextmenu={(e) => e.preventDefault()}
            role="menu"
            tabindex="-1"
        >
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--theme-bg-hover)] hover:text-[var(--theme-fg-primary)] transition-colors"
                onclick={contextEdit}
            >
                Edit
            </button>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--theme-bg-hover)] hover:text-[var(--theme-fg-primary)] transition-colors"
                onclick={contextCopy}
            >
                Copy
            </button>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--theme-bg-hover)] hover:text-[var(--theme-fg-primary)] transition-colors"
                onclick={contextPaste}
            >
                Paste
            </button>
            <div class="my-1 border-t"></div>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--theme-bg-hover)] hover:text-[var(--theme-fg-primary)] transition-colors"
                onclick={contextSetNull}
            >
                Set NULL
            </button>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--theme-bg-hover)] hover:text-[var(--theme-fg-primary)] transition-colors"
                onclick={contextSetDefault}
            >
                Set DEFAULT
            </button>
        </div>
    {/if}
</div>
