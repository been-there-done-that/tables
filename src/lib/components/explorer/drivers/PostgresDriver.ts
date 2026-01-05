/**
 * PostgreSQL Driver
 * 
 * Hierarchy: Schema → Table → Column
 * Uses lazy loading via Tauri invoke commands.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
    DatabaseDriver,
    ExplorerNode,
    NodeType,
    MetaSchema,
    MetaTable,
    MetaColumn,
} from './driver.types';
import { getIconForNodeType } from './driver.types';

export class PostgresDriver implements DatabaseDriver {
    readonly engineType = 'postgres';

    constructor(
        readonly connectionId: string,
        readonly database: string
    ) { }

    /**
     * Get schemas as root nodes.
     */
    async getRoots(): Promise<ExplorerNode[]> {
        const schemas = await invoke<MetaSchema[]>('get_schemas_lazy', {
            connectionId: this.connectionId,
            database: this.database,
        });

        return schemas.map((schema) => this.schemaToNode(schema));
    }

    /**
     * Get children for a given parent node.
     */
    async getChildren(parentId: string, parentType: NodeType): Promise<ExplorerNode[]> {
        switch (parentType) {
            case 'schema': {
                const schemaName = this.extractName(parentId, 'schema');
                return this.getTables(schemaName);
            }
            case 'table': {
                const { schema, table } = this.extractTableInfo(parentId);
                return this.getColumns(schema, table);
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
            // First segment is schema
            const schemaName = path[0];
            nodes.push({
                id: `schema:${schemaName}`,
                label: schemaName,
                type: 'schema',
                hasChildren: true,
                isLoaded: false,
                isExpanded: true,
                icon: 'schema',
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: schemaName,
                },
            });
        }

        if (path.length >= 2) {
            // Second segment is table
            const [schemaName, tableName] = path;
            nodes.push({
                id: `table:${schemaName}:${tableName}`,
                label: tableName,
                type: 'table',
                hasChildren: true,
                isLoaded: false,
                isExpanded: true,
                icon: 'table',
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: schemaName,
                    tableName,
                },
            });
        }

        return nodes;
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    private schemaToNode(schema: MetaSchema): ExplorerNode {
        return {
            id: `schema:${schema.name}`,
            label: schema.name,
            type: 'schema',
            hasChildren: true,
            isLoaded: false,
            isExpanded: false,
            icon: 'schema',
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: schema.name,
                raw: schema as unknown as Record<string, unknown>,
            },
        };
    }

    private async getTables(schemaName: string): Promise<ExplorerNode[]> {
        const tables = await invoke<MetaTable[]>('get_tables_lazy', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
        });

        return tables.map((table) => this.tableToNode(table, schemaName));
    }

    private tableToNode(table: MetaTable, schemaName: string): ExplorerNode {
        return {
            id: `table:${schemaName}:${table.table_name}`,
            label: table.table_name,
            type: 'table',
            hasChildren: true,
            isLoaded: false,
            isExpanded: false,
            icon: 'table',
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: schemaName,
                tableName: table.table_name,
                raw: table,
            },
        };
    }

    private async getColumns(schemaName: string, tableName: string): Promise<ExplorerNode[]> {
        const columns = await invoke<MetaColumn[]>('get_columns_lazy', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
            table: tableName,
        });

        return columns.map((column) => this.columnToNode(column, schemaName, tableName));
    }

    private columnToNode(column: MetaColumn, schemaName: string, tableName: string): ExplorerNode {
        const metadata = {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
            tableName,
            columnName: column.column_name,
            raw: column,
        };

        return {
            id: `column:${schemaName}:${tableName}:${column.column_name}`,
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

    private extractName(id: string, prefix: string): string {
        // Format: "prefix:name" -> extract "name"
        return id.replace(`${prefix}:`, '');
    }

    private extractTableInfo(id: string): { schema: string; table: string } {
        // Format: "table:schemaName:tableName"
        const parts = id.split(':');
        return {
            schema: parts[1],
            table: parts[2],
        };
    }
}
