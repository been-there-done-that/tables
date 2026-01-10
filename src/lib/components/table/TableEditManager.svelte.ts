import { type CellSelection, type RowEdit } from "./types";

export interface EditDelta {
    rowId: any;
    columnId: string;
    oldValue: any;
    newValue: any;
}

export class TableEditManager {
    // Reactive state
    pendingEdits = $state<Record<number, Record<string, any>>>({});
    undoStack = $state<Record<number, Record<string, any>>[]>([]);
    redoStack = $state<Record<number, Record<string, any>>[]>([]);

    // Non-reactive helper for original values to compute deltas against
    // We populate this when an edit starts or when data is loaded if needed
    private originalValues = new Map<string, any>();

    constructor() { }

    // Track original value before first edit
    private ensureOriginalValue(rowId: any, columnId: string, value: any) {
        const key = `${rowId}:${columnId}`;
        if (!this.originalValues.has(key)) {
            this.originalValues.set(key, value);
        }
    }

    setPendingEdit(rowId: number, columnId: string, newValue: any, originalValue: any) {
        // Track original for delta 
        this.ensureOriginalValue(rowId, columnId, originalValue);

        // Logic to update pendingEdits
        const currentEdits = { ...this.pendingEdits };
        if (!currentEdits[rowId]) {
            currentEdits[rowId] = {};
        }

        // Check if new value is essentially same as original, if so, remove edit?
        // For now, let's strictly set it.
        currentEdits[rowId][columnId] = newValue;

        // Push to undo stack (simplified: snapshotting whole state for now, 
        // essentially what the previous implementation did, but we can optimize later)
        this.undoStack.push(this.snapshot());
        this.redoStack = []; // clear redo on new action

        this.pendingEdits = currentEdits;
    }

    applyEditsLocally(edits: Record<number, Record<string, any>>, mode: "paste" | "input" = "input") {
        this.undoStack.push(this.snapshot());

        const next = { ...this.pendingEdits };
        for (const [rowId, cols] of Object.entries(edits)) {
            const rId = Number(rowId);
            if (!next[rId]) next[rId] = {};
            Object.assign(next[rId], cols);
        }
        this.pendingEdits = next;
        this.redoStack = [];
    }

    undo() {
        if (this.undoStack.length === 0) return;
        const current = this.snapshot();
        this.redoStack.push(current);
        const prev = this.undoStack.pop();
        if (prev) {
            this.pendingEdits = prev;
        }
    }

    redo() {
        if (this.redoStack.length === 0) return;
        const current = this.snapshot();
        this.undoStack.push(current);
        const next = this.redoStack.pop();
        if (next) {
            this.pendingEdits = next;
        }
    }

    clear() {
        this.pendingEdits = {};
        this.undoStack = [];
        this.redoStack = [];
        this.originalValues.clear();
    }

    getPendingValue(rowId: number, columnId: string): any | undefined {
        return this.pendingEdits[rowId]?.[columnId];
    }

    hasPendingValue(rowId: number, columnId: string): boolean {
        return this.pendingEdits[rowId] && columnId in this.pendingEdits[rowId];
    }

    hasPendingEdits(): boolean {
        return Object.keys(this.pendingEdits).length > 0;
    }

    // Export current changes as simple deltas
    getDeltas(): EditDelta[] {
        const deltas: EditDelta[] = [];
        for (const [rowIdStr, cols] of Object.entries(this.pendingEdits)) {
            const rowId = Number(rowIdStr);
            for (const [colId, newVal] of Object.entries(cols)) {
                const key = `${rowId}:${colId}`;
                // Usage of original value might be tricky if we didn't capture it.
                // Fallback: The consumer (Table) usually knows the original row.
                // But if we stored it:
                const oldVal = this.originalValues.get(key);
                deltas.push({
                    rowId,
                    columnId: colId,
                    oldValue: oldVal,
                    newValue: newVal
                });
            }
        }
        return deltas;
    }

    private snapshot() {
        return $state.snapshot(this.pendingEdits);
    }
}
