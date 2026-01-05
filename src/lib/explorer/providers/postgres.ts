// src/lib/explorer/providers/postgres.ts
import { invoke } from "@tauri-apps/api/core";
import type { ExplorerProvider } from "./types";
import type { ExplorerNode, NodeKind } from "../types";
import { nodes } from "../stores/nodes.svelte";
import { jobs } from "../stores/jobs.svelte";
import Folder from "@tabler/icons-svelte/icons/folder"; // Fallback
// Note: We might want specific icons later, but for logic, this is fine.

interface MetaDatabase { name: string; is_introspected: boolean; }
interface MetaSchema { name: string; is_introspected: boolean; }
interface MetaTable { table_name: string; table_type: string; } // Simplified

export const PostgresProvider: ExplorerProvider = {
    id: "postgres",

    async refresh(node: ExplorerNode) {
        console.log(`[PostgresProvider] Refreshing ${node.kind}: ${node.label}`);

        let scope: any = { type: 'global' };

        if (node.kind === 'database') {
            scope = { type: 'database', name: node.meta!.database };
        } else if (node.kind === 'schema') {
            // Full schema refresh (tables, etc.)
            scope = { type: 'schema', database: node.meta!.database, schema: node.meta!.schema };
        } else if (node.kind === 'table') {
            scope = { type: 'table', database: node.meta!.database, schema: node.meta!.schema, table: node.meta!.table };
        }

        // Track job
        const jobId = crypto.randomUUID();
        jobs.start({
            id: jobId,
            provider: 'postgres',
            connectionId: node.connectionId,
            scope,
            status: 'running',
            startTime: Date.now()
        });

        try {
            await invoke("refresh_schema_unified", {
                connectionId: node.connectionId,
                options: { scope, force: true }
            });
        } catch (e: any) {
            jobs.fail(jobId, e.toString());
            console.error("Refresh failed:", e);
            throw e;
        } finally {
            jobs.complete(jobId);
        }
    },

    async listChildren(node: ExplorerNode): Promise<ExplorerNode[]> {
        const cid = node.connectionId;

        // ROOT -> Databases
        if (node.kind === 'connection') {
            const dbs = await invoke<MetaDatabase[]>("get_cached_databases", { connectionId: cid });
            return dbs.map(db => ({
                id: `${cid}/${db.name}`,
                parentId: node.id,
                provider: 'postgres',
                connectionId: cid,
                kind: 'database',
                label: db.name,
                loadState: db.is_introspected ? 'ready' : 'idle', // Basic heuristic
                meta: { database: db.name }
            }));
        }

        // DATABASE -> Schemas
        if (node.kind === 'database') {
            const dbName = node.meta!.database!;
            const schemas = await invoke<MetaSchema[]>("get_cached_schemas", {
                connectionId: cid,
                database: dbName
            });

            return schemas.map(s => ({
                id: `${cid}/${dbName}/${s.name}`,
                parentId: node.id,
                provider: 'postgres',
                connectionId: cid,
                kind: 'schema',
                label: s.name,
                loadState: s.is_introspected ? 'partial' : 'idle', // Partial because tables might not be loaded even if schema list is
                meta: { database: dbName, schema: s.name }
            }));
        }

        // SCHEMA -> Virtual Groups (Tables, Views)
        if (node.kind === 'schema') {
            // We know what groups we want
            return [
                {
                    id: `${node.id}/group/tables`,
                    parentId: node.id,
                    provider: 'postgres',
                    connectionId: cid,
                    kind: 'group',
                    label: 'Tables',
                    loadState: 'ready',
                    meta: { ...node.meta, groupType: 'tables' }
                },
                {
                    id: `${node.id}/group/views`,
                    parentId: node.id,
                    provider: 'postgres',
                    connectionId: cid,
                    kind: 'group',
                    label: 'Views',
                    loadState: 'ready',
                    meta: { ...node.meta, groupType: 'views' }
                }
            ];
        }

        // GROUP -> Tables/Views
        if (node.kind === 'group') {
            const dbName = node.meta!.database!;
            const schemaName = node.meta!.schema!;
            const groupType = node.meta!.groupType!;

            // Fetch all tables/views for this schema from cache
            const allObjects = await invoke<MetaTable[]>("get_cached_tables", {
                connectionId: cid,
                database: dbName,
                schema: schemaName
            });

            // Filter based on group type
            const filtered = allObjects.filter(t => {
                if (groupType === 'tables') return t.table_type === 'table';
                if (groupType === 'views') return t.table_type === 'view';
                return false;
            });

            return filtered.map(t => ({
                id: `${cid}/${dbName}/${schemaName}/${t.table_name}`,
                parentId: node.id,
                provider: 'postgres',
                connectionId: cid,
                kind: t.table_type === 'view' ? 'view' : 'table',
                label: t.table_name,
                loadState: 'idle', // Table details are lazy
                meta: { database: dbName, schema: schemaName, table: t.table_name }
            }));
        }

        // TABLE -> Virtual Groups (Columns, Indexes, FKs, Triggers)
        if (node.kind === 'table') {
            return [
                { id: `${node.id}/columns`, parentId: node.id, provider: 'postgres', connectionId: cid, kind: 'group', label: 'Columns', loadState: 'ready', meta: { ...node.meta, groupType: 'columns' } },
                { id: `${node.id}/indexes`, parentId: node.id, provider: 'postgres', connectionId: cid, kind: 'group', label: 'Indexes', loadState: 'ready', meta: { ...node.meta, groupType: 'indexes' } },
                { id: `${node.id}/fks`, parentId: node.id, provider: 'postgres', connectionId: cid, kind: 'group', label: 'Foreign Keys', loadState: 'ready', meta: { ...node.meta, groupType: 'foreign_keys' } },
                { id: `${node.id}/triggers`, parentId: node.id, provider: 'postgres', connectionId: cid, kind: 'group', label: 'Triggers', loadState: 'ready', meta: { ...node.meta, groupType: 'triggers' } },
            ];
        }

        // COLUMNS GROUP -> Actual Columns
        if (node.kind === 'group' && node.meta!.groupType === 'columns') {
            const details = await invoke<any>("get_cached_table_details", {
                connectionId: cid,
                database: node.meta!.database,
                schema: node.meta!.schema,
                tableName: node.meta!.table
            });

            if (!details || !details.columns) return [];

            return details.columns.map((c: any) => ({
                id: `${node.id}/${c.column_name}`,
                parentId: node.id,
                provider: 'postgres',
                connectionId: cid,
                kind: 'column',
                label: `${c.column_name} (${c.logical_type})`,
                loadState: 'ready',
                meta: { ...node.meta, column: c.column_name }
            }));
        }

        // TODO: Implement Indexes, FKs, Triggers listing similar to Columns

        return [];
    },

    groupsFor(kind: NodeKind) {
        if (kind === 'schema') {
            return [
                { id: 'tables', label: 'Tables' },
                { id: 'views', label: 'Views' }
            ];
        }
        if (kind === 'table') {
            return [
                { id: 'columns', label: 'Columns' },
                { id: 'indexes', label: 'Indexes' },
                { id: 'foreign_keys', label: 'Foreign Keys' },
                { id: 'triggers', label: 'Triggers' }
            ];
        }
        return [];
    }
};
