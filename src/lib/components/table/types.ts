
export type ColumnType =
    | "text"
    | "int"
    | "float"
    | "boolean"
    | "date"
    | "time"
    | "datetime"
    | "json"
    | "jsonb"
    | "enum"
    | "blob";

export interface Column {
    id: string;
    label: string;
    type: ColumnType;
    width: number;
    minWidth?: number;
    maxWidth?: number;
    editable?: boolean;
    sortable?: boolean;
    filterable?: boolean;
    // Metadata for specific editors
    enumValues?: string[];
    format?: string;
    // Database specific
    dbType?: string;
}

export interface Row {
    _rowId: number; // Internal stable ID
    [key: string]: any;
}

export interface SortState {
    columnId: string;
    direction: "asc" | "desc";
}

export interface FilterState {
    columnId: string;
    value: any;
    operator: "equals" | "contains" | "gt" | "lt" | "in";
}

export interface SelectionAnchor {
    rowIndex: number;
    columnIndex: number;
}

export interface SelectionBounds {
    top: number;
    bottom: number;
    left: number;
    right: number;
}

export interface CellSelection {
    rowId: number;
    columnId: string;
}

// For clipboard/export
export type CellFormatter = (value: any, column: Column) => string;

export interface TablePlatform {
    readClipboard: () => Promise<string>;
    writeClipboard: (text: string) => Promise<void>;
}
