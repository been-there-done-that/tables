import type { EditDelta } from "$lib/components/table/TableEditManager.svelte";
import type { Column } from "$lib/components/table/types";

class PendingChangesStore {
    deltas = $state<EditDelta[]>([]);
    tableName = $state("");
    tableSchema = $state<string | undefined>(undefined);
    columns = $state<Column[]>([]);
    primaryKeyColumns = $state<string[]>([]);
    isSaving = $state(false);

    // Callbacks for actions
    onRevertRow?: (rowId: any) => void;
    onRevertAll?: () => void;
    onSaveChanges?: () => Promise<{ success: boolean; errors?: string[] }>;

    // Actions
    setContext(
        deltas: EditDelta[],
        tableName: string,
        columns: Column[],
        primaryKeyColumns: string[] = [],
        tableSchema?: string,
        callbacks?: {
            onRevertRow?: (rowId: any) => void;
            onRevertAll?: () => void;
            onSaveChanges?: () => Promise<{ success: boolean; errors?: string[] }>;
        }
    ) {
        this.deltas = deltas;
        this.tableName = tableName;
        this.tableSchema = tableSchema;
        this.columns = columns;
        this.primaryKeyColumns = primaryKeyColumns;
        this.onRevertRow = callbacks?.onRevertRow;
        this.onRevertAll = callbacks?.onRevertAll;
        this.onSaveChanges = callbacks?.onSaveChanges;
    }

    clear() {
        this.deltas = [];
        this.tableName = "";
        this.tableSchema = undefined;
        this.columns = [];
        this.primaryKeyColumns = [];
        this.onRevertRow = undefined;
        this.onRevertAll = undefined;
        this.onSaveChanges = undefined;
        this.isSaving = false;
    }
}

export const pendingChangesStore = new PendingChangesStore();
