import { type CellSelection, type RowEdit } from "./types";

export interface EditDelta {
    rowId: any;
    columnId: string;
    oldValue: any;
    newValue: any;
    pkValues?: Record<string, any>; // Original values of PK columns for this row
}

export interface EditDelta {
    rowId: any;
    columnId: string;
    oldValue: any;
    newValue: any;
    pkValues?: Record<string, any>;
    type?: "U" | "I" | "D";
}

export class TableEditManager {
    // Reactive state
    pendingEdits = $state<Record<string, Record<string, any>>>({});
    deletedRowIds = $state<Set<string>>(new Set());
    insertedRows = $state<any[]>([]);

    undoStack = $state<any[]>([]);
    redoStack = $state<any[]>([]);

    // Non-reactive helper for original values to compute deltas against
    private originalValues = new Map<string, any>();

    constructor() { }

    private ensureOriginalValue(rowId: any, columnId: string, value: any) {
        const key = `${rowId}:${columnId}`;
        if (!this.originalValues.has(key)) {
            this.originalValues.set(key, value);
        }
    }

    private isEqual(a: any, b: any): boolean {
        if (a === b) return true;
        try {
            return JSON.stringify(a) === JSON.stringify(b);
        } catch {
            return String(a) === String(b);
        }
    }

    setPendingEdit(rowId: any, columnId: string, newValue: any, originalValue: any) {
        const rId = String(rowId);
        this.ensureOriginalValue(rId, columnId, originalValue);

        const currentEdits = { ...this.pendingEdits };
        const baseline = this.originalValues.get(`${rId}:${columnId}`);

        if (this.isEqual(newValue, baseline)) {
            if (currentEdits[rId]) {
                delete currentEdits[rId][columnId];
                if (Object.keys(currentEdits[rId]).length === 0) {
                    delete currentEdits[rId];
                }
            }
        } else {
            if (!currentEdits[rId]) {
                currentEdits[rId] = {};
            }
            currentEdits[rId][columnId] = newValue;
        }

        this.recordHistory();
        this.pendingEdits = currentEdits;
    }

    trackDeletion(rowId: any) {
        const rId = String(rowId);
        const next = new Set(this.deletedRowIds);
        if (next.has(rId)) return;

        next.add(rId);
        this.recordHistory();
        this.deletedRowIds = next;
    }

    untrackDeletion(rowId: any) {
        const rId = String(rowId);
        const next = new Set(this.deletedRowIds);
        if (!next.has(rId)) return;

        next.delete(rId);
        this.recordHistory();
        this.deletedRowIds = next;
    }

    insertRow(row: any) {
        this.recordHistory();
        this.insertedRows = [...this.insertedRows, row];
    }

    removeInsertedRow(tempId: string) {
        this.recordHistory();
        this.insertedRows = this.insertedRows.filter(r => r._tempId !== tempId);
    }

    applyEditsLocally(
        edits: Record<string, Record<string, any>>,
        mode: "paste" | "input" = "input",
        originalValues?: Record<string, Record<string, any>>
    ) {
        this.recordHistory();

        if (originalValues) {
            for (const [rowId, cols] of Object.entries(originalValues)) {
                for (const [colId, val] of Object.entries(cols)) {
                    this.ensureOriginalValue(rowId, colId, val);
                }
            }
        }

        const next = { ...this.pendingEdits };
        for (const [rowId, cols] of Object.entries(edits)) {
            if (!next[rowId]) next[rowId] = {};

            for (const [colId, val] of Object.entries(cols)) {
                const baseline = this.originalValues.get(`${rowId}:${colId}`);
                if (this.isEqual(val, baseline)) {
                    delete next[rowId][colId];
                } else {
                    next[rowId][colId] = val;
                }
            }

            if (Object.keys(next[rowId]).length === 0) {
                delete next[rowId];
            }
        }
        this.pendingEdits = next;
    }

    undo() {
        if (this.undoStack.length === 0) return;
        this.redoStack.push(this.snapshot());
        const prev = this.undoStack.pop();
        this.restore(prev);
    }

    redo() {
        if (this.redoStack.length === 0) return;
        this.undoStack.push(this.snapshot());
        const next = this.redoStack.pop();
        this.restore(next);
    }

    private recordHistory() {
        this.undoStack.push(this.snapshot());
        this.redoStack = [];
    }

    private snapshot() {
        return {
            pendingEdits: $state.snapshot(this.pendingEdits),
            deletedRowIds: Array.from(this.deletedRowIds),
            insertedRows: $state.snapshot(this.insertedRows)
        };
    }

    private restore(state: any) {
        if (!state) return;
        this.pendingEdits = state.pendingEdits;
        this.deletedRowIds = new Set(state.deletedRowIds);
        this.insertedRows = state.insertedRows;
    }

    clear() {
        this.pendingEdits = {};
        this.deletedRowIds = new Set();
        this.insertedRows = [];
        this.undoStack = [];
        this.redoStack = [];
        this.originalValues.clear();
    }

    revertRow(rowId: any) {
        const rId = String(rowId);
        const next = { ...this.pendingEdits };
        let changed = false;

        if (next[rId]) {
            delete next[rId];
            changed = true;
        }

        if (this.deletedRowIds.has(rId)) {
            const nextDel = new Set(this.deletedRowIds);
            nextDel.delete(rId);
            this.deletedRowIds = nextDel;
            changed = true;
        }

        if (changed) {
            this.recordHistory();
            this.pendingEdits = next;
        }
    }

    getPendingValue(rowId: any, columnId: string): any | undefined {
        return this.pendingEdits[String(rowId)]?.[columnId];
    }

    hasPendingValue(rowId: any, columnId: string): boolean {
        const rId = String(rowId);
        return this.pendingEdits[rId] && columnId in this.pendingEdits[rId];
    }

    isDeleted(rowId: any): boolean {
        return this.deletedRowIds.has(String(rowId));
    }

    hasPendingEdits(): boolean {
        return (
            Object.keys(this.pendingEdits).length > 0 ||
            this.deletedRowIds.size > 0 ||
            this.insertedRows.length > 0
        );
    }

    getDeltas(getPkValues?: (rowId: any) => Record<string, any>): EditDelta[] {
        const deltas: EditDelta[] = [];

        // Updates
        for (const [rowId, cols] of Object.entries(this.pendingEdits)) {
            const pkValues = getPkValues?.(rowId);
            for (const [colId, newVal] of Object.entries(cols)) {
                const oldVal = this.originalValues.get(`${rowId}:${colId}`);
                deltas.push({
                    rowId,
                    columnId: colId,
                    oldValue: oldVal,
                    newValue: newVal,
                    pkValues,
                    type: "U"
                });
            }
        }

        // Deletions
        for (const rowId of this.deletedRowIds) {
            deltas.push({
                rowId,
                columnId: "*",
                oldValue: null,
                newValue: null,
                pkValues: getPkValues?.(rowId),
                type: "D"
            });
        }

        // Insertions
        for (const row of this.insertedRows) {
            for (const [colId, val] of Object.entries(row)) {
                if (colId === "_tempId") continue;
                deltas.push({
                    rowId: row._tempId,
                    columnId: colId,
                    oldValue: null,
                    newValue: val,
                    type: "I"
                });
            }
        }

        return deltas;
    }
}
