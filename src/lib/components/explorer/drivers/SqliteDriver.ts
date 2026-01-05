/**
 * SQLite Driver
 * 
 * Hierarchy: Table → Column (SQLite has implicit single schema)
 * Uses lazy loading via Tauri invoke commands.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
    DatabaseDriver,
    ExplorerNode,
    NodeType,
    MetaTable,
    MetaColumn,
} from './driver.types';
import { getIconForNodeType } from './driver.types';

/**
 * Default schema name for SQLite.
 * SQLite uses "main" as the default schema for the primary database file.
 */
const SQLITE_DEFAULT_SCHEMA = 'main';

export class SqliteDriver implements DatabaseDriver {
    readonly engineType = 'sqlite';

    constructor(
        readonly connectionId: string,
        readonly database: string
    ) { }

    /**
     * Get tables as root nodes (SQLite has no schema hierarchy in UI).
     */
    async getRoots(): Promise<ExplorerNode[]> {
        const tables = await invoke<MetaTable[]>('get_tables_lazy', {
            connectionId: this.connectionId,
            database: this.database,
            schema: SQLITE_DEFAULT_SCHEMA,
        });

        return tables.map((table) => this.tableToNode(table));
    }

    /**
     * Get children for a given parent node.
     */
    async getChildren(parentId: string, parentType: NodeType): Promise<ExplorerNode[]> {
        switch (parentType) {
            case 'table': {
                const tableName = this.extractTableName(parentId);
                return this.getColumns(tableName);
            }
            default:
                return [];
        }
    }

    /**
     * Resolve breadcrumbs for deep-linking.
     */
    async resolveBreadcrumbs(path: string[]): Promise<ExplorerNode[]> {
        const nodes: ExplorerNode[] = [];

        if (path.length >= 1) {
            // First segment is table
            const tableName = path[0];
            nodes.push({
                id: `table:${tableName}`,
                label: tableName,
                type: 'table',
                hasChildren: true,
                isLoaded: false,
                isExpanded: true,
                icon: 'table',
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: SQLITE_DEFAULT_SCHEMA,
                    tableName,
                },
            });
        }

        return nodes;
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    private tableToNode(table: MetaTable): ExplorerNode {
        return {
            id: `table:${table.table_name}`,
            label: table.table_name,
            type: 'table',
            hasChildren: true,
            isLoaded: false,
            isExpanded: false,
            icon: 'table',
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: SQLITE_DEFAULT_SCHEMA,
                tableName: table.table_name,
                raw: table,
            },
        };
    }

    private async getColumns(tableName: string): Promise<ExplorerNode[]> {
        const columns = await invoke<MetaColumn[]>('get_columns_lazy', {
            connectionId: this.connectionId,
            database: this.database,
            schema: SQLITE_DEFAULT_SCHEMA,
            table: tableName,
        });

        return columns.map((column) => this.columnToNode(column, tableName));
    }

    private columnToNode(column: MetaColumn, tableName: string): ExplorerNode {
        const metadata = {
            connectionId: this.connectionId,
            database: this.database,
            schema: SQLITE_DEFAULT_SCHEMA,
            tableName,
            columnName: column.column_name,
            raw: column,
        };

        return {
            id: `column:${tableName}:${column.column_name}`,
            label: column.column_name,
            type: 'column',
            hasChildren: false,
            isLoaded: true,
            isExpanded: false,
            icon: getIconForNodeType('column', metadata),
            secondaryLabel: column.logical_type,
            metadata,
        };
    }

    private extractTableName(id: string): string {
        // Format: "table:tableName"
        return id.replace('table:', '');
    }
}
