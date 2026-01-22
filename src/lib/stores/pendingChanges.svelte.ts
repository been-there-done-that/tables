import type { EditDelta } from "$lib/components/table/TableEditManager.svelte";
import type { Column } from "$lib/components/table/types";

class PendingChangesStore {
    deltas = $state<EditDelta[]>([]);
    tableName = $state("");
    tableSchema = $state<string | undefined>(undefined);
    columns = $state<Column[]>([]);
    primaryKeyColumns = $state<string[]>([]);
    
    // Callbacks for actions
    onRevertRow?: (rowId: any) => void;
    onRevertAll?: () => void;

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
        }
    ) {
        this.deltas = deltas;
        this.tableName = tableName;
        this.tableSchema = tableSchema;
        this.columns = columns;
        this.primaryKeyColumns = primaryKeyColumns;
        this.onRevertRow = callbacks?.onRevertRow;
        this.onRevertAll = callbacks?.onRevertAll;
    }

    clear() {
        this.deltas = [];
        this.tableName = "";
        this.tableSchema = undefined;
        this.columns = [];
        this.primaryKeyColumns = [];
        this.onRevertRow = undefined;
        this.onRevertAll = undefined;
    }
}

export const pendingChangesStore = new PendingChangesStore();
