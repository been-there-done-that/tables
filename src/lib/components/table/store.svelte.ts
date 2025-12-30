
import { type Column, type Row, type SortState, type SelectionAnchor, type CellSelection } from "./types";

const ROW_HEIGHT = 32; // Fixed row height for now, matching most dense tables
const OVERSCAN = 5;

export class TableController {
    // --- State (Runes) ---
    // Core Data
    rawRows = $state<Row[]>([]);
    columns = $state<Column[]>([]);

    // Viewport / Virtualization
    scrollTop = $state(0);
    containerHeight = $state(0);

    // Sorting & Filtering
    sortState = $state<SortState[]>([]);
    filterQuery = $state(""); // Simple global search for now, can extend later

    // Selection
    selectedRowIds = $state<Set<number>>(new Set());
    selectedCells = $state<CellSelection[]>([]); // Array of specific cells
    focusedCell = $state<SelectionAnchor | null>(null);
    anchorCell = $state<SelectionAnchor | null>(null); // For range selection

    // Editing
    editingCell = $state<CellSelection | null>(null);
    editingRect = $state<{ top: number; left: number; width: number; height: number } | null>(null);

    // --- Derived State ---

    selectedCellSet = $derived.by(() => {
        const set = new Set<string>();
        for (const cell of this.selectedCells) {
            set.add(`${cell.rowId}:${cell.columnId}`);
        }
        return set;
    });

    // 1. Process data (Sort -> Filter)
    processedRows = $derived.by(() => {
        let rows = [...this.rawRows]; // Create shallow copy

        // Sorting
        if (this.sortState.length > 0) {
            rows.sort((a, b) => {
                for (const sort of this.sortState) {
                    const valA = a[sort.columnId];
                    const valB = b[sort.columnId];
                    if (valA === valB) continue;

                    const dir = sort.direction === "asc" ? 1 : -1;
                    if (valA == null) return 1; // nulls last
                    if (valB == null) return -1;

                    if (valA < valB) return -1 * dir;
                    if (valA > valB) return 1 * dir;
                }
                return 0;
            });
        }

        return rows;
    });

    totalHeight = $derived(this.processedRows.length * ROW_HEIGHT);

    // 2. Virtualization
    virtualState = $derived.by(() => {
        const total = this.processedRows.length;
        if (total === 0 || this.containerHeight === 0) {
            return { startIndex: 0, endIndex: 0, visibleItems: [], offsetY: 0 };
        }

        const startIndex = Math.max(0, Math.floor(this.scrollTop / ROW_HEIGHT) - OVERSCAN);
        const endIndex = Math.min(
            total,
            Math.ceil((this.scrollTop + this.containerHeight) / ROW_HEIGHT) + OVERSCAN
        );

        const offsetY = startIndex * ROW_HEIGHT;
        const visibleItems = this.processedRows.slice(startIndex, endIndex);

        return { startIndex, endIndex, visibleItems, offsetY };
    });

    // --- Actions ---

    constructor() {
        // Init logic if needed
    }

    loadData(rows: Row[], cols: Column[]) {
        this.rawRows = rows;
        // Ensure columns have width
        this.columns = cols.map(c => ({
            ...c,
            // Ensure width, potentially from saved state or default
            width: c.width || 150
        }));
    }

    handleScroll(scrollTop: number, containerHeight: number) {
        if (this.scrollTop === scrollTop && this.containerHeight === containerHeight) return;
        this.scrollTop = scrollTop;
        this.containerHeight = containerHeight;
    }

    toggleSort(columnId: string, multi: boolean) {
        const existing = this.sortState.find(s => s.columnId === columnId);
        let newDir: "asc" | "desc" | null = "asc";

        if (existing) {
            newDir = existing.direction === "asc" ? "desc" : null;
        }

        if (!multi) {
            if (newDir) {
                this.sortState = [{ columnId, direction: newDir }];
            } else {
                this.sortState = [];
            }
        } else {
            // Multi-sort logic could verify keeping others
            if (newDir) {
                this.sortState = [
                    ...this.sortState.filter(s => s.columnId !== columnId),
                    { columnId, direction: newDir }
                ];
            } else {
                this.sortState = this.sortState.filter(s => s.columnId !== columnId);
            }
        }
    }

    resizeColumn(columnId: string, width: number) {
        const idx = this.columns.findIndex(c => c.id === columnId);
        if (idx !== -1) {
            this.columns[idx].width = Math.max(width, this.columns[idx].minWidth || 50);
        }
    }

    // Selection Logic
    selectCell(rowIndex: number, colIndex: number, multi: boolean, range: boolean) {
        if (rowIndex < 0 || rowIndex >= this.processedRows.length) return;
        if (colIndex < 0 || colIndex >= this.columns.length) return;

        const row = this.processedRows[rowIndex];
        const col = this.columns[colIndex];

        this.focusedCell = { rowIndex, columnIndex: colIndex };

        if (range && this.anchorCell) {
            // Range selection logic
            this.selectRange(this.anchorCell, { rowIndex, columnIndex: colIndex });
        } else if (multi) {
            // Toggle individual cell
            this.anchorCell = { rowIndex, columnIndex: colIndex };
            const cellId = { rowId: row._rowId, columnId: col.id };

            const existsIdx = this.selectedCells.findIndex(c => c.rowId === cellId.rowId && c.columnId === cellId.columnId);
            if (existsIdx >= 0) {
                // Remove
                const next = [...this.selectedCells];
                next.splice(existsIdx, 1);
                this.selectedCells = next;

                // Update selectedRowIds? Complex if mixed selection. 
                // For now, we only sync rowIds if full row is selected or for styling hinting
            } else {
                this.selectedCells = [...this.selectedCells, cellId];
            }
        } else {
            // Single select / Reset
            this.anchorCell = { rowIndex, columnIndex: colIndex };
            this.selectedCells = [{ rowId: row._rowId, columnId: col.id }];
            this.selectedRowIds = new Set([row._rowId]);
        }
    }

    selectRange(start: SelectionAnchor, end: SelectionAnchor) {
        const top = Math.min(start.rowIndex, end.rowIndex);
        const bottom = Math.max(start.rowIndex, end.rowIndex);
        const left = Math.min(start.columnIndex, end.columnIndex);
        const right = Math.max(start.columnIndex, end.columnIndex);

        const cells: CellSelection[] = [];
        const rows = new Set<number>();

        for (let r = top; r <= bottom; r++) {
            const row = this.processedRows[r];
            rows.add(row._rowId);
            for (let c = left; c <= right; c++) {
                const col = this.columns[c];
                cells.push({ rowId: row._rowId, columnId: col.id });
            }
        }

        this.selectedCells = cells;
        this.selectedRowIds = rows;
    }

    async copySelection() {
        if (this.selectedCells.length === 0) return;

        // Group by row to form a grid for text format
        // Optimization: Sort selected cells by row index then col index
        // We need a map to check existence quickly during iteration or just iterate selection if sparse

        // 1. Identify bounds of selection to create a rect text
        // or just list them? usually TSV is rect.

        // Let's assume standard behavior: Copy unique selected cells. 
        // If it's a sparse selection, TSV might differ.
        // For simplicity: Iterate all cells, check if selected.

        // Better: Collect values
        const rowsMap = new Map<number, Map<string, any>>();

        for (const cell of this.selectedCells) {
            if (!rowsMap.has(cell.rowId)) rowsMap.set(cell.rowId, new Map());
            // We need to find the value. Expensive if we don't have direct access.
            // We have processedRows. We can look up by ID? No, processedRows is array.
            // We generally select visually.
            // Let's rely on the fact that selectedCells usually comes from SelectRange (dense).

            // ... actually, looking up row by ID for 100k rows is O(N).
            // We should store rowRef if possible or index? 
            // Indices change on sort. IDs are stable.
            // Map lookup is O(1). We don't have a rowID -> Row map.
            // Let's create one on data load? Or linear scan if selection is small.
        }

        // For 100k rows, we SHOULD have an id->row map.
        const rowMap = new Map(this.processedRows.map(r => [r._rowId, r]));

        // Helper to get column index for sorting
        const colIdxMap = new Map(this.columns.map((c, i) => [c.id, i]));

        // We want to format as a grid.
        // 1. Group by Row Index (visual order) ? Or Row ID?
        // Visual copy usually expects visual order.
        // So we need to intersect `processedRows` with `selectedCells`.

        const selectedSet = new Set(this.selectedCells.map(c => `${c.rowId}:${c.columnId}`));

        const outputRows: string[] = [];
        let currentRow: string[] = [];
        let collectingRow = false;

        // Iterate visible order
        for (const row of this.processedRows) {
            const rowId = row._rowId;
            let hasSelection = false;
            const rowValues: string[] = [];

            for (const col of this.columns) {
                if (selectedSet.has(`${rowId}:${col.id}`)) {
                    hasSelection = true;
                    // Format value
                    let val = row[col.id];
                    if (val === null || val === undefined) val = "";
                    rowValues.push(String(val)); // Simple stringify for now
                }
            }

            if (hasSelection) {
                outputRows.push(rowValues.join("\t"));
            }
        }

        if (outputRows.length > 0) {
            await navigator.clipboard.writeText(outputRows.join("\n"));
        }
    }

    startEditing(rowIndex: number, colIndex: number, rect: { top: number; left: number; width: number; height: number }) {
        const row = this.processedRows[rowIndex];
        const col = this.columns[colIndex];
        // Check if editable? (Assuming all editable for now or check col.editable)

        this.editingCell = { rowId: row._rowId, columnId: col.id };
        this.editingRect = rect;
    }

    commitEditing(val: any) {
        if (!this.editingCell) return;
        const { rowId, columnId } = this.editingCell;

        // Find row in rawRows to update it persistently
        // Optimization: Use a map if frequent. For now linear scan is ok for 10k arrays? 
        // 10k linear scan is fast (microsecond).
        const rawIndex = this.rawRows.findIndex(r => r._rowId === rowId);
        if (rawIndex !== -1) {
            // Update the array item carefully to trigger reactivity
            const newRow = { ...this.rawRows[rawIndex], [columnId]: val };
            const nextRaw = [...this.rawRows];
            nextRaw[rawIndex] = newRow;
            this.rawRows = nextRaw;
        }

        this.editingCell = null;
        this.editingRect = null;
    }

    cancelEditing() {
        this.editingCell = null;
        this.editingRect = null;
    }

    handleKeydown(e: KeyboardEvent) {
        // Copy: Mod+C
        if ((e.metaKey || e.ctrlKey) && e.key === "c") {
            e.preventDefault();
            this.copySelection();
            return;
        }

        if (!this.focusedCell) return;

        const { rowIndex, columnIndex } = this.focusedCell;
        let nextRow = rowIndex;
        let nextCol = columnIndex;

        switch (e.key) {
            case "ArrowUp": nextRow--; break;
            case "ArrowDown": nextRow++; break;
            case "ArrowLeft": nextCol--; break;
            case "ArrowRight": nextCol++; break;
            default: return;
        }

        e.preventDefault();

        // Clamp
        nextRow = Math.max(0, Math.min(nextRow, this.processedRows.length - 1));
        nextCol = Math.max(0, Math.min(nextCol, this.columns.length - 1));

        this.selectCell(nextRow, nextCol, e.metaKey || e.ctrlKey, e.shiftKey);

        // Auto-scroll logic: simple version
        // We need to know if nextRow is visible.
        const { startIndex, endIndex } = this.virtualState;
        if (nextRow < startIndex) {
            this.scrollTop = nextRow * ROW_HEIGHT;
        } else if (nextRow >= endIndex - 1) { // -1 because endIndex is exclusive or close
            this.scrollTop = (nextRow - (this.containerHeight / ROW_HEIGHT) + 2) * ROW_HEIGHT;
        }
    }
}

