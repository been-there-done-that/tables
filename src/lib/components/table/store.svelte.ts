
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

    // --- Derived State ---

    // 1. Process data (Sort -> Filter)
    processedRows = $derived.by(() => {
        let rows = [...this.rawRows];

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
            width: c.width || 150
        }));
    }

    handleScroll(scrollTop: number, containerHeight: number) {
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
            // Toggle individual cell (rare in data grids, usually row based, but we support cell)
            // Implementation typically adds to selectedCells
            this.anchorCell = { rowIndex, columnIndex: colIndex };
            /* simplified for now */
            this.selectedCells = [{ rowId: row._rowId, columnId: col.id }];
        } else {
            // Single select
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

    handleKeydown(e: KeyboardEvent) {
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

        // Auto-scroll into view logic would go here or in component effect
    }
}
