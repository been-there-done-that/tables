/**
 * PostgreSQL Driver
 * 
 * Hierarchy: Schema → Tables/Views folders → Table/View → Columns/FKs/Indexes/Triggers folders
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
    MetaForeignKey,
    MetaIndex,
    MetaTrigger,
} from './driver.types';
import { getIconForNodeType } from './driver.types';

// Folder sub-types for parsing
type FolderSubType = 'tables' | 'views' | 'columns' | 'foreign_keys' | 'indexes' | 'triggers';

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
                // Schema → Tables + Views folders
                const schemaName = this.extractName(parentId, 'schema');
                return this.getSchemaFolders(schemaName);
            }
            case 'folder': {
                // Folder → actual items
                return this.getFolderContents(parentId);
            }
            case 'table':
            case 'view': {
                // Table/View → Columns + FKs + Indexes + Triggers folders
                const { schema, table } = this.extractTableInfo(parentId);
                return this.getTableFolders(schema, table);
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
    // Schema children: Tables + Views folders
    // =========================================================================

    private async getSchemaFolders(schemaName: string): Promise<ExplorerNode[]> {
        // Fetch tables to get counts
        const tables = await invoke<MetaTable[]>('get_tables_lazy', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
        });

        const tableCount = tables.filter(t => t.table_type === 'table' || t.table_type === 'BASE TABLE').length;
        const viewCount = tables.filter(t => t.table_type === 'view' || t.table_type === 'VIEW').length;

        const folders: ExplorerNode[] = [];

        // Tables folder
        if (tableCount > 0) {
            folders.push({
                id: `folder:tables:${schemaName}`,
                label: 'Tables',
                type: 'folder',
                hasChildren: true,
                isLoaded: false,
                isExpanded: false,
                icon: 'folder',
                secondaryLabel: String(tableCount),
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: schemaName,
                },
            });
        }

        // Views folder
        if (viewCount > 0) {
            folders.push({
                id: `folder:views:${schemaName}`,
                label: 'Views',
                type: 'folder',
                hasChildren: true,
                isLoaded: false,
                isExpanded: false,
                icon: 'folder',
                secondaryLabel: String(viewCount),
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: schemaName,
                },
            });
        }

        return folders;
    }

    // =========================================================================
    // Folder contents
    // =========================================================================

    private async getFolderContents(folderId: string): Promise<ExplorerNode[]> {
        const { folderType, schema, table } = this.parseFolderId(folderId);

        switch (folderType) {
            case 'tables':
                return this.getTables(schema!, 'table');
            case 'views':
                return this.getTables(schema!, 'view');
            case 'columns':
                return this.getColumns(schema!, table!);
            case 'foreign_keys':
                return this.getForeignKeys(schema!, table!);
            case 'indexes':
                return this.getIndexes(schema!, table!);
            case 'triggers':
                return this.getTriggers(schema!, table!);
            default:
                return [];
        }
    }

    // =========================================================================
    // Table children: Columns + FKs + Indexes + Triggers folders
    // =========================================================================

    private async getTableFolders(schemaName: string, tableName: string): Promise<ExplorerNode[]> {
        // Fetch table details to get counts
        const details = await invoke<{
            columns: MetaColumn[];
            foreign_keys: MetaForeignKey[];
            indexes: MetaIndex[];
            triggers: MetaTrigger[];
        }>('get_cached_table_details', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
            tableName: tableName,
        });

        const folders: ExplorerNode[] = [];

        // Columns folder (always present)
        folders.push({
            id: `folder:columns:${schemaName}:${tableName}`,
            label: 'Columns',
            type: 'folder',
            hasChildren: details.columns.length > 0,
            isLoaded: false,
            isExpanded: false,
            icon: 'folder',
            secondaryLabel: String(details.columns.length),
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: schemaName,
                tableName,
            },
        });

        // Foreign Keys folder
        if (details.foreign_keys.length > 0) {
            folders.push({
                id: `folder:foreign_keys:${schemaName}:${tableName}`,
                label: 'Foreign Keys',
                type: 'folder',
                hasChildren: true,
                isLoaded: false,
                isExpanded: false,
                icon: 'folder',
                secondaryLabel: String(details.foreign_keys.length),
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: schemaName,
                    tableName,
                },
            });
        }

        // Indexes folder
        if (details.indexes.length > 0) {
            folders.push({
                id: `folder:indexes:${schemaName}:${tableName}`,
                label: 'Indexes',
                type: 'folder',
                hasChildren: true,
                isLoaded: false,
                isExpanded: false,
                icon: 'folder',
                secondaryLabel: String(details.indexes.length),
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: schemaName,
                    tableName,
                },
            });
        }

        // Triggers folder
        if (details.triggers.length > 0) {
            folders.push({
                id: `folder:triggers:${schemaName}:${tableName}`,
                label: 'Triggers',
                type: 'folder',
                hasChildren: true,
                isLoaded: false,
                isExpanded: false,
                icon: 'folder',
                secondaryLabel: String(details.triggers.length),
                metadata: {
                    connectionId: this.connectionId,
                    database: this.database,
                    schema: schemaName,
                    tableName,
                },
            });
        }

        return folders;
    }

    // =========================================================================
    // Leaf data fetchers
    // =========================================================================

    private async getTables(schemaName: string, tableType: 'table' | 'view'): Promise<ExplorerNode[]> {
        const tables = await invoke<MetaTable[]>('get_tables_lazy', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
        });

        const filtered = tables.filter(t => {
            const type = t.table_type?.toLowerCase();
            if (tableType === 'table') {
                return type === 'table' || type === 'base table';
            } else {
                return type === 'view';
            }
        });

        return filtered.map((table) => this.tableToNode(table, schemaName, tableType));
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

    private async getForeignKeys(schemaName: string, tableName: string): Promise<ExplorerNode[]> {
        const details = await invoke<{
            foreign_keys: MetaForeignKey[];
        }>('get_cached_table_details', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
            tableName: tableName,
        });

        return details.foreign_keys.map((fk) => this.fkToNode(fk, schemaName, tableName));
    }

    private async getIndexes(schemaName: string, tableName: string): Promise<ExplorerNode[]> {
        const details = await invoke<{
            indexes: MetaIndex[];
        }>('get_cached_table_details', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
            tableName: tableName,
        });

        return details.indexes.map((idx) => this.indexToNode(idx, schemaName, tableName));
    }

    private async getTriggers(schemaName: string, tableName: string): Promise<ExplorerNode[]> {
        const details = await invoke<{
            triggers: MetaTrigger[];
        }>('get_cached_table_details', {
            connectionId: this.connectionId,
            database: this.database,
            schema: schemaName,
            tableName: tableName,
        });

        return details.triggers.map((trigger) => this.triggerToNode(trigger, schemaName, tableName));
    }

    // =========================================================================
    // Node builders
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

    private tableToNode(table: MetaTable, schemaName: string, tableType: 'table' | 'view'): ExplorerNode {
        const nodeType: NodeType = tableType === 'view' ? 'view' : 'table';
        return {
            id: `${nodeType}:${schemaName}:${table.table_name}`,
            label: table.table_name,
            type: nodeType,
            hasChildren: true,
            isLoaded: false,
            isExpanded: false,
            icon: nodeType,
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: schemaName,
                tableName: table.table_name,
                raw: table,
            },
        };
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

    private fkToNode(fk: MetaForeignKey, schemaName: string, tableName: string): ExplorerNode {
        const label = fk.constraint_name || `${fk.column_name} → ${fk.ref_table}.${fk.ref_column}`;
        return {
            id: `foreign_key:${schemaName}:${tableName}:${fk.constraint_name || fk.column_name}`,
            label,
            type: 'foreign_key',
            hasChildren: false,
            isLoaded: true,
            isExpanded: false,
            icon: 'foreign_key',
            secondaryLabel: `→ ${fk.ref_table}`,
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: schemaName,
                tableName,
                raw: fk as unknown as Record<string, unknown>,
            },
        };
    }

    private indexToNode(idx: MetaIndex, schemaName: string, tableName: string): ExplorerNode {
        return {
            id: `index:${schemaName}:${tableName}:${idx.index_name}`,
            label: idx.index_name,
            type: 'index',
            hasChildren: false,
            isLoaded: true,
            isExpanded: false,
            icon: 'index',
            secondaryLabel: idx.is_unique ? 'UNIQUE' : undefined,
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: schemaName,
                tableName,
                raw: idx as unknown as Record<string, unknown>,
            },
        };
    }

    private triggerToNode(trigger: MetaTrigger, schemaName: string, tableName: string): ExplorerNode {
        return {
            id: `trigger:${schemaName}:${tableName}:${trigger.trigger_name}`,
            label: trigger.trigger_name,
            type: 'trigger',
            hasChildren: false,
            isLoaded: true,
            isExpanded: false,
            icon: 'trigger',
            secondaryLabel: `${trigger.timing} ${trigger.event}`,
            metadata: {
                connectionId: this.connectionId,
                database: this.database,
                schema: schemaName,
                tableName,
                raw: trigger as unknown as Record<string, unknown>,
            },
        };
    }

    // =========================================================================
    // ID parsers
    // =========================================================================

    private extractName(id: string, prefix: string): string {
        return id.replace(`${prefix}:`, '');
    }

    private extractTableInfo(id: string): { schema: string; table: string } {
        // Format: "table:schemaName:tableName" or "view:schemaName:viewName"
        const parts = id.split(':');
        return {
            schema: parts[1],
            table: parts[2],
        };
    }

    private parseFolderId(id: string): { folderType: FolderSubType; schema?: string; table?: string } {
        // Formats:
        // folder:tables:schemaName
        // folder:views:schemaName
        // folder:columns:schemaName:tableName
        // folder:foreign_keys:schemaName:tableName
        // folder:indexes:schemaName:tableName
        // folder:triggers:schemaName:tableName
        const parts = id.split(':');
        const folderType = parts[1] as FolderSubType;
        const schema = parts[2];
        const table = parts[3];

        return { folderType, schema, table };
    }
}
