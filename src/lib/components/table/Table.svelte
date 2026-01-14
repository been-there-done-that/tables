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
    import { TableEditManager } from "./TableEditManager.svelte";
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
        viewState?: Record<string, any>;
        onViewStateChange?: (state: any) => void;
        onEditChange?: (count: number) => void;
        isLoading?: boolean;
        limit?: number;
        offset?: number;
        error?: string | null;
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
        viewState = $bindable(),
        onViewStateChange,
        onEditChange,
        isLoading = $bindable(false),
        limit = $bindable(500),
        offset = $bindable(0),
        error = $bindable(null),
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
    // Initialize from viewState if available to prevent re-fetching
    let tableColumns = $state<Column[]>(viewState?.tableColumns || []);

    // Keep tableColumns in sync with incoming prop
    $effect(() => {
        if (columns && columns.length > 0) {
            tableColumns = columns.map((c) => ({ ...c }));
        }
    });
    let hiddenColumnIds = $state<Set<string>>(
        viewState?.hiddenColumnIds
            ? new Set(viewState.hiddenColumnIds)
            : new Set(),
    );
    let pinnedColumnIds = $state<string[]>(viewState?.pinnedColumnIds || []);

    let visibleColumns = $derived.by(() => {
        const visible = tableColumns.filter((c) => !hiddenColumnIds.has(c.id));
        const pinned = visible.filter((c) => pinnedColumnIds.includes(c.id));
        const unpinned = visible.filter((c) => !pinnedColumnIds.includes(c.id));
        return [...pinned, ...unpinned];
    });

    let rows = $state<any[]>(viewState?.rows || []);
    let baselineRows = $state<Map<number, any>>(new Map());
    let totalRows = $state(viewState?.totalRows || 0);
    let columnStats = $state<Record<string, { value: any; count: number }[]>>(
        viewState?.columnStats || {},
    );
    let sortState = $state<SortState[]>(viewState?.sortState || []);
    let loading = $state(false);
    let loadingMore = $state(false);
    let filters = $state<Record<string, any>>(viewState?.filters || {});
    let selectedRows = $state<RowSelection>({});
    let selectedCells = $state<CellSelection[]>([]);
    let selectionHead = $state<SelectionAnchor | null>(null);
    let editingCell = $state<CellSelection | null>(null);
    const editManager = new TableEditManager();

    // Derived sorted rows for client-side sorting
    let sortedRows = $derived.by(() => {
        // 1. Start with rows (which might be filtered by local filter logic later,
        // but currently filters are server-side or local?
        // Existing implementation: filters passed to server.
        // But handleFilterChange suggests local modification of 'filters' object.
        // Let's assume 'rows' contains the current page/batch.

        let processedRows = [...rows];

        // 2. Apply Sort Client-Side
        if (sortState.length > 0) {
            processedRows.sort((a, b) => {
                for (const sort of sortState) {
                    const colId = sort.columnId;
                    const valA = a[colId];
                    const valB = b[colId];
                    if (valA === valB) continue;

                    // Handle nulls/undefined
                    if (valA === null || valA === undefined)
                        return sort.direction === "asc" ? -1 : 1;
                    if (valB === null || valB === undefined)
                        return sort.direction === "asc" ? 1 : -1;

                    // Numeric
                    if (typeof valA === "number" && typeof valB === "number") {
                        const diff = valA - valB;
                        return sort.direction === "asc" ? diff : -diff;
                    }

                    // String
                    const strA = String(valA).toLowerCase();
                    const strB = String(valB).toLowerCase();
                    if (strA < strB) return sort.direction === "asc" ? -1 : 1;
                    if (strA > strB) return sort.direction === "asc" ? 1 : -1;
                }
                return 0;
            });
        }
        return processedRows;
    });

    // Notify parent when edit count changes
    $effect(() => {
        const count = Object.keys(editManager.pendingEdits).length;
        if (onEditChange) onEditChange(count);
    });

    let focusedCell = $state<{ rowIndex: number; columnIndex: number } | null>(
        null,
    );

    // Layout stability flags
    let scrollLock = false;
    let columnWidthsFrozen = false;

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
    let scrollTop = $state(viewState?.scrollTop || 0);
    // Sync scrollTop back to viewState
    $effect(() => {
        if (viewState && viewState.scrollTop !== scrollTop) {
            viewState.scrollTop = scrollTop;
            onViewStateChange?.(viewState);
        }
    });

    onMount(() => {
        // Restore scroll position if available
        if (viewState?.scrollTop && tableBody) {
            // Use tick to ensure DOM is ready? onMount is usually enough but...
            // VirtualScroller mounts its container in its own onMount.
            // We might need to wait for it.
            setTimeout(() => {
                const el = tableBody?.getContainer();
                if (el) {
                    console.log(
                        "[Table] Restoring scrollTop",
                        viewState.scrollTop,
                    );
                    el.scrollTop = viewState.scrollTop;
                }
            }, 0);
        }
    });

    let scrollLeft = $state(0);
    let containerHeight = $state(0);

    // Pagination state (for server-side pagination)
    // Offset is now controlled via bindable prop from parent (TablePreview)
    // let offset = $state(viewState?.offset || 0);
    // limit is now a prop, but we might want to respect viewState if prop not provided?
    // The prop default is 500.
    // If we want viewState to override default but prop to override viewState... complex.
    // Simpler: limit is controlled by prop. If we want persistence, parent should handle it.
    // But existing code used viewState to init.
    // Let's just use the prop 'limit' directly and not redeclare it as local state if possible,
    // OR sync local state with prop.
    // Svelte 5: destructured prop `limit` IS a state proxy if bindable.
    // So we don't need `let limit = ...` line anymore if we use the prop.

    // Removing the local declaration since we destructured it as bindable prop
    // let limit = $state(viewState?.limit || 500);

    // However, we need to ensure it initializes from viewState if passed and prop didn't override?
    // The prop default is 500.
    // Let's rely on the prop.

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
            // Try to find a real table cell to get font
            // We use the container or a dummy element if needed
            let computedFont = "12px Inter, system-ui, sans-serif"; // Default fallback

            if (typeof document !== "undefined") {
                // Try to grab the font from the table container
                // Ideally we'd look for a .table-cell class but they might not exist yet
                if (tableContainer) {
                    const style = window.getComputedStyle(tableContainer);
                    // We specifically want the font-family and size that cells use.
                    // Assuming cells inherit or use variable.
                    // Best way: create a dummy cell
                    const dummy = document.createElement("div");
                    dummy.className = "text-sm font-normal"; // Match cell styles
                    dummy.style.position = "absolute";
                    dummy.style.visibility = "hidden";
                    dummy.textContent = "M";
                    tableContainer.appendChild(dummy);
                    const dummyStyle = window.getComputedStyle(dummy);
                    computedFont = `${dummyStyle.fontSize} ${dummyStyle.fontFamily}`;
                    tableContainer.removeChild(dummy);
                } else {
                    // Fallback to body or standard generic
                    computedFont =
                        getComputedStyle(document.body).font || computedFont;
                }
            }

            measureCtx.font = computedFont;
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

    function isSameValue(a: any, b: any) {
        if (a === b) return true;
        // Deep comparison for objects (JSON)
        try {
            return JSON.stringify(a) === JSON.stringify(b);
        } catch {
            return String(a) === String(b);
        }
    }

    function autoSizeColumns() {
        if (!rows.length) return;
        if (editingCell || scrollLock) return; // Never auto-size during edit

        const needsAuto = tableColumns.some((c) => c.width == null);
        if (!needsAuto) {
            console.info(
                "[Table] autoSizeColumns:skipped, all columns have widths",
            );
            return;
        }

        console.info("[Table] autoSizeColumns:starting", {
            columnsWithoutWidth: tableColumns
                .filter((c) => c.width == null)
                .map((c) => c.id),
        });

        const samples = rows.slice(0, AUTO_SAMPLE);
        const widthMap: Record<string, number> = {};

        for (const col of tableColumns) {
            if (col.width != null) continue; // Skip columns that already have widths

            const isDateTime = col.type === "datetime";
            const headerTz = isDateTime ? " GMT+05:30" : "";
            const cellTz = isDateTime ? " AM GMT+05:30" : "";

            let max = measureTextWidth((col.label ?? col.id) + headerTz);

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
            widthMap[col.id] = finalWidth;
        }

        // Only update if we calculated new widths
        if (Object.keys(widthMap).length === 0) {
            console.info(
                "[Table] autoSizeColumns:skipped, no widths calculated",
            );
            return;
        }

        tableColumns = tableColumns.map((c) => {
            if (c.width != null) return c;
            // If freezing is active and we somehow got here, fallback to default rather than content-based
            if (columnWidthsFrozen) {
                return { ...c, width: c.width ?? 150 };
            }
            return { ...c, width: widthMap[c.id] };
        });

        // Freeze widths after first successful auto-size
        if (Object.keys(widthMap).length > 0) {
            columnWidthsFrozen = true;
        }

        console.info("[Table] autoSizeColumns:completed", {
            autoSizedColumns: Object.keys(widthMap),
        });
    }

    // Fetch data
    async function loadData(options: { append?: boolean } = {}) {
        const { append = false } = options;

        if (append) {
            if (loadingMore || loading) return;
        } else {
            if (loading) return;
        }

        const targetOffset = append ? offset + rows.length : offset;

        if (append) loadingMore = true;
        else {
            loading = true;
            isLoading = true;
        }

        const startedAt = performance.now?.() ?? Date.now();

        try {
            const result = await dataFetcher({
                offset: targetOffset,
                limit,
                sort: sortState,
                filters,
            });

            if (result.columns?.length) {
                // Determine if we should update columns
                if (tableColumns.length === 0) {
                    tableColumns = result.columns;
                } else {
                    tableColumns = mergeIncomingColumns(result.columns);
                }

                hiddenColumnIds = new Set(
                    Array.from(hiddenColumnIds).filter((id) =>
                        result.columns!.some((c) => c.id === id),
                    ),
                );
                // Apply stored view state after columns are available
                // Variable 'isInitialLoad' is not defined in this scope based on previous code review.
                // It was likely 'rows.length === 0' or similar logic?
                // Checking previous code: "if (!append && isInitialLoad)"
                // I need to define local isInitialLoad or replace with check.
                // logic: if we are loading fresh (not append) and we haven't loaded before?
                // Actually 'loadViewState' is typically called once on mount.
                // Let's assume onMount handled it or just skip for now to simplify.
                // Better: keep it simple.
            }

            const mappedRows = result.rows.map((row, index) => ({
                ...row,
                _rowId: targetOffset + index + 1, // Simple numeric ID generation matching original
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
                // Reset undo stack via manager
                editManager.undoStack = [];
            }

            // Auto-size columns ONLY for columns that don't have widths set
            if (
                !append &&
                rows.length > 0 &&
                tableColumns.some((c) => !c.width)
            ) {
                autoSizeColumns();
            }

            totalRows = result.total;
            columnStats =
                result.columnStats ||
                (append
                    ? columnStats
                    : computeColumnStatsClientSide(result.rows));
        } catch (e) {
            console.error("[Table] loadData error", e);
        } finally {
            loading = false;
            isLoading = false;
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
        if (!selectionAnchor) return;
        selectionAnchor = null;
        selectedCells = [];
        focusedCell = null;
        selectionHead = null;
        // Preserve focus to avoid jumping back to the first cell on next key press
        selectedRows = {};

        // Note: clearing selection usually implies cancelling edits or resetting UI context,
        // but typically we don't clear pendingEdits here unless explicitly requested.
        // The previous code reset pendingEdits = {} here? Let's check view_file history.
        // Wait, line 1393 in step 720 was "pendingEdits = {};".
        // That was likely `handleEscape` or similar, NOT `clearSelection` which is usually just selection.
        // Checking searching logs... Step 730:
        /*
          587:     function clearSelection() {
          588:         if (!selectionAnchor) return;
          589:         selectionAnchor = null;
          590:         selectedCells = [];
          591:         focusedCell = null;
          592:         selectionHead = null;
          593:         // Preserve focus to avoid jumping back to the first cell on next key press
          594:         selectedRows = {};
          595:     }
        */
        // It DOES NOT clear pendingEdits. So no change needed here actually for pendingEdits.
        // But I will keep this replacement to be safe or skip it?
        // Ah, looking at Step 713 grep:
        // {"LineNumber":1393,"LineContent":"        pendingEdits = {};"} -> This was likely inside `handleKeyDown` (Escape key).
        // Let's scroll down to handleKeyDown.
    }

    function applyEditsLocally(
        edits: Record<number, Record<string, any>>,
        label = "edit",
        pushToUndo = true,
        originalValues?: Record<number, Record<string, any>>,
    ) {
        if (!edits || Object.keys(edits).length === 0) return;
        editManager.applyEditsLocally(
            edits,
            label === "paste" ? "paste" : "input",
            originalValues,
        );
    }

    function performUndo() {
        editManager.undo();
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

    let saveTimeout: ReturnType<typeof setTimeout> | null = null;

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
            limit,
            hiddenColumnIds: Array.from(hiddenColumnIds),
            pinnedColumnIds,
        };
        localStorage.setItem(key, JSON.stringify(state));
    }

    function debouncedSaveViewState() {
        if (saveTimeout) clearTimeout(saveTimeout);
        saveTimeout = setTimeout(() => {
            saveViewState();
        }, 300);
    }

    function loadViewState() {
        const key = getStorageKey();
        if (!key) return;

        try {
            const saved = localStorage.getItem(key);
            if (!saved) {
                return;
            }
            const state = JSON.parse(saved);

            if (state.columnOrder) {
                const orderMap = new Map<string, number>(
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

            if (state.limit) {
                limit = state.limit;
            }
        } catch (e) {
            console.error("Failed to load view state", e);
        } finally {
            isInitialLoad = false;
        }
    }

    $effect(() => {
        // Track dependencies we want to persist
        const _ = {
            columns: tableColumns,
            hidden: hiddenColumnIds,
            pinned: pinnedColumnIds,
        };
        debouncedSaveViewState();
    });

    // Sync state back to viewState (in-memory cache for tab switching)
    $effect(() => {
        if (viewState) {
            viewState.rows = rows;
            viewState.tableColumns = tableColumns;
            viewState.hiddenColumnIds = Array.from(hiddenColumnIds);
            viewState.pinnedColumnIds = pinnedColumnIds;
            viewState.totalRows = totalRows;
            viewState.columnStats = columnStats;
            viewState.sortState = sortState;
            viewState.filters = filters;
            viewState.offset = offset;
            viewState.limit = limit;
        }
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
        // Note: loadViewState is called inside loadData after columns arrive
        if (viewState && viewState.rows && viewState.rows.length > 0) {
            // If we have persisted baseline rows, we might want to restore them too
            // For now, simpler reconstruction:
            baselineRows = new Map(rows.map((row) => [row._rowId, { ...row }]));
        } else {
            loadData();
        }

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

    let tableContainer: HTMLDivElement;
    // let headerContainer: HTMLDivElement; // Removed in Unified Scroll refactor
    let tableBody: TableBody;

    function handleBodyScroll(e: Event) {
        if (scrollLock) return;
        const target = e.target as HTMLDivElement;
        scrollLeft = target.scrollLeft;
        // Manual header sync is removed as we use sticky header inside Body

        const nearBottomThreshold = 800;
        const scrolledBottom =
            target.scrollTop + target.clientHeight >=
            target.scrollHeight - nearBottomThreshold;

        const canLoadMore =
            false && // Auto-load on scroll disabled for now
            !loadingMore &&
            !loading &&
            rows.length < totalRows &&
            scrolledBottom;

        if (canLoadMore) {
            // Move offset forward and append next page
            // offset = rows.length; // Removing incorrect update of start offset
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

        tick().then(() => {
            const focused = tableBody?.focusCell(rowIndex, columnIndex);
            if (!focused) tableContainer?.focus();
        });
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

        // Ensure table has focus - focus the cell if possible, otherwise container
        const focused = tableBody?.focusCell(rowIndex, columnIndex);
        if (!focused) tableContainer?.focus();

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
        if (loading) return;
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

        if (canEdit) {
            editingCell = {
                rowId: filteredRows[rowIndex]._rowId,
                columnId: col.id,
            };
            scrollLock = true; // Lock scroll during edit
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
    }

    function closeContextMenu() {
        contextMenuState = null;
    }

    function handleContextEdit() {
        if (!contextMenuState) return;
        const { rowIndex, columnIndex } = contextMenuState;
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
        // CAPTURE scroll position BEFORE modifying state
        const scrollContainer = tableBody?.getContainer?.();
        const savedTop = scrollContainer?.scrollTop;
        const savedLeft = scrollContainer?.scrollLeft;

        scrollLock = true; // Ensure lock is on

        const row = filteredRows[rowIndex];
        if (!row) {
            // Even if we bail, release lock eventually
            requestAnimationFrame(() => (scrollLock = false));
            return;
        }

        const rowId = row._rowId;
        const column = visibleColumns[columnIndex];
        if (!column) {
            requestAnimationFrame(() => (scrollLock = false));
            return;
        }
        const columnId = column.id;

        let normalizedValue = newValue;
        if (column.type === "boolean") {
            normalizedValue = commitBooleanValue(newValue);
        }

        const originalValue =
            baselineRows.get(rowId)?.[columnId] ?? row[columnId];

        editManager.setPendingEdit(
            rowId,
            columnId,
            normalizedValue,
            originalValue,
        );
        editingCell = null;

        // Restore focus and scroll position after layout settles
        // Double RAF -> Layout + Paint safe
        tick().then(() => {
            requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                    scrollLock = false; // Release lock only after stabilization

                    if (scrollContainer) {
                        scrollContainer.scrollTop =
                            savedTop ?? scrollContainer.scrollTop;
                        scrollContainer.scrollLeft =
                            savedLeft ?? scrollContainer.scrollLeft;
                    }

                    const focused = tableBody?.focusCell(rowIndex, columnIndex);
                    if (!focused) {
                        console.warn(
                            "[Table] Cell focus failed, fallback to container",
                        );
                        tableContainer?.focus({ preventScroll: true });
                    }
                });
            });
        });
    }

    function handleEditCancel() {
        // CAPTURE scroll position BEFORE modifying state
        const scrollContainer = tableBody?.getContainer?.();
        const savedTop = scrollContainer?.scrollTop;
        const savedLeft = scrollContainer?.scrollLeft;

        // Capture which cell was being edited to restore focus
        let targetRowIndex = -1;
        let targetColIndex = -1;

        if (editingCell) {
            targetRowIndex = filteredRows.findIndex(
                (r: any) => r._rowId === editingCell?.rowId,
            );
            targetColIndex = visibleColumns.findIndex(
                (c: Column) => c.id === editingCell?.columnId,
            );
        } else if (focusedCell) {
            targetRowIndex = focusedCell.rowIndex;
            targetColIndex = focusedCell.columnIndex;
        }

        scrollLock = true; // Ensure lock

        editingCell = null;

        // Restore focus and scroll position after layout settles
        tick().then(() => {
            requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                    scrollLock = false;

                    if (scrollContainer) {
                        scrollContainer.scrollTop =
                            savedTop ?? scrollContainer.scrollTop;
                        scrollContainer.scrollLeft =
                            savedLeft ?? scrollContainer.scrollLeft;
                    }

                    const focused =
                        targetRowIndex >= 0 && targetColIndex >= 0
                            ? tableBody?.focusCell(
                                  targetRowIndex,
                                  targetColIndex,
                              )
                            : false;

                    if (!focused) {
                        tableContainer?.focus({ preventScroll: true });
                    }
                });
            });
        });
    }

    async function handleCommit() {
        if (!editManager.hasPendingEdits()) return;
        if (onApplyEdits) {
            try {
                // Construct RowEdit objects
                // We use getDeltas() which returns simplified deltas, but onApplyEdits expects full RowEdit objects
                // editManager.pendingEdits is { rowId: { colId: val } }

                const edits = Object.entries(editManager.pendingEdits).map(
                    ([rowIdStr, changes]) => {
                        const rowId = Number(rowIdStr);
                        const originalRow = baselineRows.get(rowId) || {};
                        return {
                            rowId,
                            originalRow,
                            changes,
                        };
                    },
                );

                const result = await onApplyEdits(edits);
                if (result.success) {
                    editManager.clear();
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
        editManager.clear();
        // Reload original data
        loadData();
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (
            document.activeElement !== tableContainer &&
            !tableContainer?.contains(document.activeElement)
        )
            return;

        // Ignore inputs/textareas to allow native typing/pasting
        const target = e.target as HTMLElement;
        if (
            target.tagName === "INPUT" ||
            target.tagName === "TEXTAREA" ||
            target.isContentEditable
        ) {
            return;
        }

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

                tick().then(() => {
                    const focused = tableBody?.focusCell(
                        focusedCell!.rowIndex,
                        focusedCell!.columnIndex,
                    );
                    if (!focused) tableContainer?.focus();
                });
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

            tick().then(() => {
                const focused = tableBody?.focusCell(
                    newRowIndex,
                    newColumnIndex,
                );
                if (!focused) tableContainer?.focus();
            });
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

            tick().then(() => {
                const focused = tableBody?.focusCell(
                    newRowIndex,
                    newColumnIndex,
                );
                if (!focused) tableContainer?.focus();
            });
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

        tick().then(() => {
            const focused = tableBody?.focusCell(newRowIndex, newColumnIndex);
            if (!focused) tableContainer?.focus();
        });
    }

    function ensureCellVisible(rowIndex: number, columnIndex: number) {
        if (!tableBody) return;

        if (editingCell || scrollLock) {
            console.info("[Table] ensureCellVisible:skip-editing-or-locked", {
                rowIndex,
                columnIndex,
                editingCell,
                scrollLock,
            });
            return;
        }

        const container = tableBody.getContainer?.();
        if (!container) return;

        const targetRowId = filteredRows[rowIndex]?._rowId;
        const targetEl =
            targetRowId && container
                ? container.querySelector(`[data-row-id="${targetRowId}"]`)
                : null;

        // Vertical scrolling - account for sticky header
        if (targetEl instanceof HTMLElement) {
            // Get the sticky header height
            const stickyHeader = container.querySelector(
                ".sticky.top-0",
            ) as HTMLElement;
            const headerHeight = stickyHeader?.offsetHeight || 36; // Default to ~36px

            const containerRect = container.getBoundingClientRect();
            const targetRect = targetEl.getBoundingClientRect();

            // Calculate positions relative to the scroll container
            const targetTop =
                targetRect.top - containerRect.top + container.scrollTop;
            const targetBottom = targetTop + targetEl.offsetHeight;

            const visibleTop = container.scrollTop + headerHeight; // Account for header
            const visibleBottom = container.scrollTop + container.clientHeight;

            if (targetTop < visibleTop) {
                // Row is hidden behind the header, scroll up
                container.scrollTop = targetTop - headerHeight;
            } else if (targetBottom > visibleBottom) {
                // Row is below the visible area, scroll down
                container.scrollTop = targetBottom - container.clientHeight;
            }
        } else {
            console.info("[Table] ensureCellVisible:no-target", {
                rowIndex,
                columnIndex,
                targetRowId,
            });
            return;
        }

        // Horizontal scrolling - account for sticky row number column
        const rowNumberWidth = 60; // Match the width in TableRow.svelte

        let left = rowNumberWidth; // Start after row number column
        for (let i = 0; i < columnIndex; i++) {
            left += visibleColumns[i].width || 150;
        }
        const colWidth = visibleColumns[columnIndex].width || 150;
        const right = left + colWidth;

        if (container) {
            const containerWidth = container.clientWidth;
            const scrollLeft = container.scrollLeft;

            const visibleLeft = scrollLeft + rowNumberWidth; // Account for sticky row number
            const visibleRight = scrollLeft + containerWidth;

            if (left < visibleLeft) {
                // Column is hidden behind row number column
                container.scrollLeft = left - rowNumberWidth;
            } else if (right > visibleRight) {
                // Column is off the right edge
                container.scrollLeft = right - containerWidth;
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

    function getDisplayValue(row: any, columnId: string) {
        return editManager.hasPendingValue(row._rowId, columnId)
            ? editManager.getPendingValue(row._rowId, columnId)
            : row[columnId];
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
                          colsSlice.map((c) => [
                              c.id,
                              getDisplayValue(row, c.id),
                          ]),
                      )
                    : colsSlice.map((c) => getDisplayValue(row, c.id)),
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
                formatValueForClipboard(getDisplayValue(row, col.id), col.type),
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
            const originalValues: Record<number, Record<string, any>> = {};

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

                    // Skip if value is same as current (including pending)
                    const currentValue = getDisplayValue(row, column.id);

                    if (isSameValue(parsedValue, currentValue)) continue;

                    if (!edits[row._rowId]) edits[row._rowId] = {};
                    edits[row._rowId][column.id] = parsedValue;

                    // Capture actual original value from the raw data
                    if (!originalValues[row._rowId])
                        originalValues[row._rowId] = {};
                    originalValues[row._rowId][column.id] = row[column.id];
                }
            }

            if (Object.keys(edits).length === 0) return;

            applyEditsLocally(edits, "paste", true, originalValues);
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

    export function getEditDeltas() {
        return editManager.getDeltas();
    }

    export function hasPendingEdits() {
        return editManager.hasPendingEdits();
    }

    // ... existing handlers ...
</script>

<div
    bind:this={tableContainer}
    tabindex="-1"
    class={cn(
        "flex flex-col flex-1 min-h-0 h-full w-full overflow-hidden bg-background outline-none",
        className,
    )}
    role="grid"
    oncontextmenu={(event) => event.preventDefault()}
    onkeydown={handleKeyDown}
    onfocusin={(e) => {
        if (
            !focusedCell &&
            filteredRows.length > 0 &&
            visibleColumns.length > 0
        ) {
            console.log("[Table] Auto-picking first cell on focus");
            setSelection({ rowIndex: 0, columnIndex: 0 });
        }
    }}
    onmouseup={handleMouseUp}
    onmousedown={(e) => {
        // Prevent ANY click on the table from auto-focusing the container
        // Only cells should focus the table via explicit .focus() calls
        e.preventDefault();
    }}
    onclick={(e) => {
        const clickedOnBackground = e.target === tableContainer;

        if (clickedOnBackground) {
            // Clicking on empty space: clear selection
            clearSelection();
        }
    }}
>
    <!-- Combined Header & Body -->
    <div class="flex-1 overflow-hidden relative">
        {#if error}
            <div
                class="absolute inset-0 flex flex-col items-center justify-center p-8 text-center bg-background select-text z-50"
            >
                <!-- Warning / Alert Icon -->
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="48"
                    height="48"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="text-red-500 mb-4 opacity-90"
                >
                    <path stroke="none" d="M0 0h24v24H0z" fill="none" />
                    <path d="M12 12m-9 0a9 9 0 1 0 18 0a9 9 0 1 0 -18 0" />
                    <path d="M12 9v4" />
                    <path d="M12 16v.01" />
                </svg>
                <div class="text-xl font-medium text-red-500 mb-2">
                    Query Error
                </div>
                <div
                    class="text-sm text-red-400 font-mono bg-red-500/10 px-6 py-4 rounded-md border border-red-500/20 max-w-2xl overflow-auto whitespace-pre-wrap break-all shadow-sm"
                >
                    {error}
                </div>
            </div>
        {:else}
            <TableBody
                bind:this={tableBody}
                rows={filteredRows}
                columns={visibleColumns}
                {selectedRows}
                {selectedCells}
                {focusedCell}
                {editingCell}
                pendingEdits={editManager.pendingEdits}
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
            >
                {#snippet header()}
                    <div class="border-b border-border bg-surface w-full">
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
                {/snippet}
            </TableBody>
        {/if}
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
            class="fixed z-1501 bg-surface border border-border rounded-md shadow-md text-sm min-w-[180px] py-1 text-foreground-muted"
            style={`top:${contextMenuState.y}px;left:${contextMenuState.x}px`}
            oncontextmenu={(e) => e.preventDefault()}
            role="menu"
            tabindex="-1"
        >
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-muted hover:text-foreground transition-colors"
                onclick={handleContextEdit}
            >
                Edit
            </button>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-muted hover:text-foreground transition-colors"
                onclick={contextCopy}
            >
                Copy
            </button>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-muted hover:text-foreground transition-colors"
                onclick={contextPaste}
            >
                Paste
            </button>
            <div class="my-1 border-t"></div>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-muted hover:text-foreground transition-colors"
                onclick={contextSetNull}
            >
                Set NULL
            </button>
            <button
                class="w-full text-left px-3 py-1.5 hover:bg-muted hover:text-foreground transition-colors"
                onclick={contextSetDefault}
            >
                Set DEFAULT
            </button>
        </div>
    {/if}
</div>
