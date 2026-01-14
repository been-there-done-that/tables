import type { EditDelta } from "$lib/components/table/TableEditManager.svelte";
import type { Column } from "$lib/components/table/types";

class PendingChangesStore {
    deltas = $state<EditDelta[]>([]);
    tableName = $state("");
    tableSchema = $state<string | undefined>(undefined);
    columns = $state<Column[]>([]);
    primaryKeyColumns = $state<string[]>([]);

    // Actions
    setContext(
        deltas: EditDelta[],
        tableName: string,
        columns: Column[],
        primaryKeyColumns: string[] = [],
        tableSchema?: string,
    ) {
        this.deltas = deltas;
        this.tableName = tableName;
        this.tableSchema = tableSchema;
        this.columns = columns;
        this.primaryKeyColumns = primaryKeyColumns;
    }

    clear() {
        this.deltas = [];
        this.tableName = "";
        this.tableSchema = undefined;
        this.columns = [];
        this.primaryKeyColumns = [];
    }
}

export const pendingChangesStore = new PendingChangesStore();
