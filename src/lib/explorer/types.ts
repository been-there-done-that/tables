// src/lib/explorer/types.ts

export type NodeKind =
    | "connection"
    | "database"
    | "schema"
    | "group"        // Virtual grouping node (e.g. "Tables", "Views")
    | "table"
    | "view"
    | "column"
    | "index"
    | "foreign_key"
    | "trigger";

export type LoadState =
    | "idle"         // Never requested / initial state
    | "loading"      // Currently fetching or introspecting
    | "ready"        // Fully loaded from cache
    | "partial"      // Children available but deeper levels pending
    | "error";       // Fail state

export interface ExplorerNode {
    id: string;                 // Stable, globally unique ID (e.g. "postgres://conn1/db1/schema1/tables")
    parentId: string | null;

    provider: string;           // "postgres", "sqlite", "s3", etc.
    connectionId: string;

    kind: NodeKind;
    label: string;

    loadState: LoadState;
    disabled?: boolean;         // UI interaction disabled (e.g. during specific operations)
    errorMessage?: string;

    icon?: any;                 // Optional: Override default icon

    // Count of children for group nodes or container nodes
    childCount?: number;

    // Provider-specific identity metadata
    // Used to map back to backend/introspection constructs
    meta?: {
        database?: string;
        schema?: string;
        table?: string;
        column?: string;

        // For groups
        groupType?: string; // "tables", "views", etc.

        [key: string]: any;
    };
}
