/**
 * Driver Types
 * 
 * Core interfaces for database-agnostic tree rendering.
 * The UI only understands ExplorerNode, drivers handle engine-specific logic.
 */

import type { MetaColumn, MetaForeignKey, MetaIndex, MetaTable, MetaTrigger } from '$lib/query/schemaQueries';

// ============================================================================
// Node Types
// ============================================================================

export type NodeType = 'root' | 'database' | 'schema' | 'table' | 'column';

export type IconType =
    | 'database'
    | 'schema'
    | 'table'
    | 'column'
    | 'key'
    | 'folder'
    | 'file';

/**
 * Unified node structure for the explorer tree.
 * The UI renders this type exclusively - completely engine-agnostic.
 */
export interface ExplorerNode {
    /** Unique identifier, format: "type:name" or "type:parent:name" */
    id: string;
    /** Display label */
    label: string;
    /** Semantic node type */
    type: NodeType;
    /** Whether this node can be expanded */
    hasChildren: boolean;
    /** Whether children have been fetched */
    isLoaded: boolean;
    /** Current expansion state (managed by UI) */
    isExpanded: boolean;
    /** Icon identifier for rendering */
    icon: IconType;
    /** Additional data type info (e.g., column type) */
    secondaryLabel?: string;
    /** Engine-specific payload (OID, file path, etc.) */
    metadata: ExplorerNodeMetadata;
}

/**
 * Metadata attached to each node for context menu actions and navigation.
 */
export interface ExplorerNodeMetadata {
    connectionId: string;
    database?: string;
    schema?: string;
    tableName?: string;
    columnName?: string;
    /** Original backend data */
    raw?: MetaTable | MetaColumn | Record<string, unknown>;
}

// ============================================================================
// Driver Interface
// ============================================================================

/**
 * Database driver interface.
 * All database engines implement this to provide their specific hierarchy.
 */
export interface DatabaseDriver {
    /** Engine identifier (e.g., 'postgres', 'sqlite') */
    readonly engineType: string;
    /** Connection ID this driver is bound to */
    readonly connectionId: string;
    /** Database name this driver is scoped to */
    readonly database: string;

    /**
     * Get root-level nodes for the explorer tree.
     * - Postgres: Returns schemas
     * - SQLite: Returns tables directly
     */
    getRoots(): Promise<ExplorerNode[]>;

    /**
     * Get child nodes for a given parent.
     * @param parentId - The parent node's ID
     * @param parentType - The parent node's type
     */
    getChildren(parentId: string, parentType: NodeType): Promise<ExplorerNode[]>;

    /**
     * Resolve a path into nodes for deep-linking/breadcrumbs.
     * @param path - Array of path segments (e.g., ["public", "users"])
     */
    resolveBreadcrumbs(path: string[]): Promise<ExplorerNode[]>;
}

// ============================================================================
// Re-export Meta types for convenience
// ============================================================================

export type { MetaColumn, MetaTable, MetaSchema, MetaForeignKey, MetaIndex, MetaTrigger } from '$lib/query/schemaQueries';

// ============================================================================
// Icon mapping helper
// ============================================================================

export function getIconForNodeType(type: NodeType, metadata?: ExplorerNodeMetadata): IconType {
    if (type === 'column' && metadata?.raw) {
        const column = metadata.raw as MetaColumn;
        if (column.is_primary_key) return 'key';
    }

    switch (type) {
        case 'root':
        case 'database':
            return 'database';
        case 'schema':
            return 'schema';
        case 'table':
            return 'table';
        case 'column':
            return 'column';
        default:
            return 'folder';
    }
}
