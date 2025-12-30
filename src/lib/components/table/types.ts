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
    | "JSON"
    | "enum"
    | "blob"
    | "bytea"
    | "binary";

export type EditorMode = "inline" | "popover" | "modal";

export interface EditorConfig {
    mode: EditorMode;
    renderer: string;
    props?: Record<string, any>;
}

export interface Column {
    id: string;
    label: string;
    type: ColumnType;
    editable: boolean;
    sortable: boolean;
    filterable: boolean;
    width?: number;
    minWidth?: number;
    maxWidth?: number;
    enumValues?: string[];
    format?: string;
    foreignKey?: {
        refTable: string;
        refColumn: string;
    };

    // New architecture
    renderer?: string;
    editor?: Partial<EditorConfig>;

    /**
     * Backend-provided metadata to keep rendering/clipboard aware of DB-native types.
     * Keeps the component "plug and play": the backend can set these and the table
     * will render appropriately without hardcoded client mappings.
     */
    dbType?: string;
    dbSchema?: string;
    dbTable?: string;
    rawType?: string;
}

export interface EditingState {
    rowId: any;
    columnId: string;
    value: any;
    mode: EditorMode;
    renderer: string;
    props?: Record<string, any>;
    anchorRect?: DOMRect;
}

export interface SortState {
    columnId: string;
    direction: "asc" | "desc";
}

export interface DataFetcherParams {
    offset: number;
    limit: number;
    sort: SortState[];
    filters: Record<string, any>;
}

export interface DataFetcherResult {
    rows: any[];
    total: number;
    columnStats?: Record<string, { value: any; count: number }[]>;
    /**
     * Optional column metadata supplied by the backend. When present, the table
     * will adopt these definitions (merging width/visibility where possible).
     */
    columns?: Column[];
}

export type DataFetcher = (params: DataFetcherParams) => Promise<DataFetcherResult>;

export interface EditResult {
    success: boolean;
    conflicts?: any[];
}

export type OnApplyEdits = (editedRows: Record<string, any>) => Promise<EditResult>;

export interface RowSelection {
    [rowId: number]: boolean;
}

export interface CellSelection {
    rowId: number;
    columnId: string;
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

export type ClipboardFormat = "tsv" | "csv" | "json";

export interface TableQueryContext {
    tableName?: string;
    tableSchema?: string;
    columnId?: string;
    selectedColumns?: string[];
}
